# Part 1: Tetris architecture

This design follows the six modules we discussed: input, terminal output, a small game kernel, a seven-bag randomizer, the falling piece, and the collection of locked cells. The only extra module is a resource loader for the JSON files.

The main idea is simple: **the kernel decides when; the piece and the board decide how.** The kernel advances one frame, asks the active piece to move or rotate, decides whether to lock it, and then asks the dead-piece module to merge cells and clear rows.

This is a proposal for Part 1 only. It does not implement the game. The Rust snippets describe types and interfaces; they are not complete source files.

## 1. Scope and requirements

The source for this design is the human-readable README from the introduction through the end of “Part 1 Implementation Advice.” Only two external pages were opened, both directly linked in that section: the SRS explanation and the top-out explanation. No internet searches were used. Resource JSON, test contents, and later assignment sections were not read for this design.

The reviewed README portion contained no HTML comments, hidden-content styling, invisible/control characters, or long opaque payloads indicative of hidden instructions. No suspicious malicious instructions were found in that portion. This is a report about the reviewed material, not an audit of the rest of the repository.

The program must:

| Requirement | Design consequence |
| --- | --- |
| Accept `--simulate-inputs-single-player <inputs-file>` and `--resources <directory>` | Parse the two paths before creating a game. |
| Interpret `C W L R D H E`, ignoring whitespace | Each action character produces exactly one frame. Whitespace produces no frames. |
| Simulate discrete frames | Call `Game::step(action)` in a loop. No real clock or sleeping is needed. |
| Load configuration, piece states, and wall-kick offsets from JSON | Load once at startup and pass typed data to the game. |
| Use the required deterministic seven-bag | Preserve the specified seed, versions, algorithm, ordering, and reshuffle procedure. |
| Apply actions before gravity and locking | Keep the frame sequence explicit in one short kernel method. |
| Check top-out before clearing lines | Use one shared lock-completion path for hard drops and timed locks. |
| Print a 10-column, 22-row board | Serialize the locked cells and current falling piece once at the end. |
| Stop advancing after top-out and show no next piece | Set `active = None`, mark the game finished, and stop consuming actions. |

“Render in the terminal” means writing the required plain text to stdout. Part 1 does not need keyboard polling, animation, a terminal UI library, or a 60 Hz timer. An empty input still has an initial falling piece to render. Reaching the end of a nonempty input leaves the current piece where it is; it does not force a final drop.

## 2. Module layout and ownership

Proposed files:

```text
src/
  main.rs         startup, action loop, final output, error reporting
  input.rs        command-line arguments and action parsing
  render.rs       exact board serialization
  game.rs         kernel: frame order, lock decisions, top-out, spawning
  randomizer.rs   the README's deterministic BagRandomizer
  piece.rs        active piece, shapes, rotations, candidate movement
  dead_piece.rs   locked cells, collision checks, merging, line clearing
  resources.rs    JSON loading and validation
```

These are ordinary Rust modules in one binary crate. A separate library crate, plugin system, event bus, or trait hierarchy would not help this part.

```mermaid
flowchart TD
    Files["Arguments and input file"] --> Input["input.rs: paths and Actions"]
    Input --> Main["main.rs: run the simulation"]
    JSON["Resource JSON files"] --> Loader["resources.rs: load and validate"]
    Loader --> Main
    Main -->|"one Action per frame"| Kernel["game.rs: Game kernel"]
    Kernel -->|"owns and commands"| Active["piece.rs: ActivePiece"]
    Kernel -->|"owns; merge and clear"| Dead["dead_piece.rs: DeadPiece"]
    Kernel -->|"owns; advance after a successful lock"| Bag["randomizer.rs: BagRandomizer"]
    Active -->|"read-only placement checks"| Dead
    Main -->|"borrow final game"| Render["render.rs: serialize"]
    Render --> Output["stdout: 22 rows of 10 characters"]
```

`Game` owns the mutable state. The active piece borrows the dead piece while checking a move; it never changes locked cells. The renderer borrows the game and never changes anything. Modules communicate with direct method calls and return values.

| Owner | Data it owns | Why |
| --- | --- | --- |
| `Game` | Resources, randomizer, one `Option<ActivePiece>`, one `DeadPiece`, game status | There is one place to coordinate the simulation. |
| `ActivePiece` | Kind, shape/kick definition, origin, rotation, and per-piece counters | All state specific to the current falling piece stays together. |
| `DeadPiece` | A grid of occupied cells | After locking, original piece identities are irrelevant. |
| `Resources` | Config and the seven piece definitions | JSON is read once; gameplay does no file I/O. |
| `BagRandomizer` | Bag order, current index, seeded RNG | Randomness stays isolated and reproducible. |

At spawn, copy/clone the small definition for that piece into `ActivePiece`. This avoids lifetimes and shared ownership machinery. It is a few small arrays and lists per spawn, which is entirely reasonable here.

## 3. Coordinates and the board

Use the README's coordinate convention everywhere:

- `x` increases to the right, from `0` through `9`.
- `y` increases upward; `y = 0` is the floor row.
- The ordinary playfield is 20 rows high.
- The spawn origin is `(4, 20)`, with rotation `0`.
- A piece cell is `origin + shape_offset`.
- The output includes rows `21` down through `0`, in that order.

```text
               x = 0                 x = 9
                  +--------------------+
y = 21            | output / spawn area|  first printed row
y = 20            |     origin (4,20)  |
                  +--------------------+
y = 19            |                    |  top ordinary playfield row
                  |                    |
                  |                    |
y =  0            |                    |  last printed row
                  +--------------------+
                          floor
```

Keep coordinates signed while calculating candidates. A left move at `x = 0` or a downward move at the floor should fail a bounds check, not underflow an unsigned integer. Convert to array indices only after checking bounds.

Use a row-oriented representation for the dead piece:

```rust
pub const WIDTH: usize = 10;
pub const PLAY_HEIGHT: i32 = 20;
pub const OUTPUT_HEIGHT: usize = 22;

pub struct DeadPiece {
    rows: Vec<[bool; WIDTH]>, // rows[y][x], initially 22 empty rows
}
```

A vector of fixed-width rows is nearly as simple as a fixed 22-row array. It also lets a final locked piece retain cells above the output window without an out-of-bounds write. `merge` grows the vector only if necessary. The renderer still prints exactly 22 rows.

The proposed placement rule rejects side walls, the floor, and occupied cells. It treats positions above the stored rows as empty, rather than treating the output window as a ceiling. This allows a rotated piece near spawn to extend above row 21. The README does not explicitly define the upper collision boundary; this choice is recorded again in Section 10.

There is no separate board that stores both falling and locked cells. The dead piece stores locked cells; the falling piece is overlaid only when rendering. This avoids having to erase the old falling piece before every move.

## 4. Module interfaces

### 4.1 Input: turn text into actions

`input.rs` handles the external input format. It should not know anything about collision detection or gravity.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    RotateClockwise,
    RotateCounterclockwise,
    Left,
    Right,
    SoftDrop,
    HardDrop,
    Wait,
}

pub struct CliOptions {
    pub inputs_path: PathBuf,
    pub resources_dir: PathBuf,
}

pub fn parse_args(args: impl Iterator<Item = OsString>)
    -> Result<CliOptions, Box<dyn std::error::Error>>;

pub fn parse_actions(bytes: &[u8])
    -> Result<Vec<Action>, Box<dyn std::error::Error>>;
```

Map `C W L R D H E` to the seven variants. Skip ASCII whitespace, including newlines, spaces, tabs, and carriage returns. Reject other bytes with a useful error, including their position. Do not silently turn invalid input into `Wait`.

A `Vec<Action>` is sufficient for Part 1. Loading and validating the file first does not advance the simulation. Once top-out occurs, the action loop stops consuming this vector. Behavior for malformed input after a would-be top-out is not specified; whole-file validation is a deliberate simple choice for invalid files.

Use a small argument parser from the standard library for these two flags. Accept either flag order; report missing values, duplicate flags, and unknown flags. `clap` is also allowed by the README, but this interface does not require it.

### 4.2 Resources: translate JSON into usable game data

`resources.rs` loads the supplied files from the requested directory and validates the resulting data before starting the game.

```rust
pub struct GameConfig {
    pub gravity_frames: u32,
    pub soft_drop_reduction: u32,
    pub lock_delay_frames: u32,
    pub seed: [u8; 16],
}

pub struct Resources {
    pub config: GameConfig,
    pieces: [PieceDefinition; 7], // indexed by I, J, L, O, S, T, Z
}

impl Resources {
    pub fn load(dir: &Path)
        -> Result<Self, Box<dyn std::error::Error>>;
    pub fn definition(&self, kind: PieceKind) -> &PieceDefinition;
}

impl GameConfig {
    pub fn gravity_for_frame(&self, soft_drop: bool) -> u32;
}
```

These config field names are **internal design names**, not claims about the actual JSON keys. The resource contents have not been inspected. During implementation, match the supplied schema, using private `Deserialize` structs and a small conversion if needed. Use the resource filenames actually provided rather than guessing the shape file's name.

Validation should cover the seven required kinds, four rotations per kind, four distinct cells per shape, complete kick tables with matching candidate counts across rotations, valid seed data, and sensible numeric ranges. Preserve array order in the kick lists. A malformed resource should produce an error naming its file instead of causing a later indexing panic.

For soft drop, the proposed literal interpretation is to subtract the configured reduction from the ordinary gravity interval for that frame. Use saturating subtraction to avoid unsigned underflow. An effective interval of zero means the single gravity attempt is immediately due; it does not cause an inner fall loop. Confirm the resource's meaning when its schema is read. Do not turn a `D` action into a direct one-cell translation or permanently modify the config.

Dependencies: `serde` with its derive feature, `serde_json`, and the two required RNG crates. Pin the RNG crates exactly as the README specifies: `rand = "=0.10.3"` and `rand_xoshiro = "=0.8.1"`. This proposal does not install or verify those packages.

### 4.3 Active piece: own the falling shape and its movement

`piece.rs` represents only the falling piece. It knows how to form candidate cells, test them against the dead piece, and commit a successful movement.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PieceKind { I, J, L, O, S, T, Z }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation { Spawn, Right, Half, Left }

#[derive(Debug, Clone, Copy)]
pub enum Turn { Clockwise, Counterclockwise }

#[derive(Clone)]
pub struct PieceDefinition {
    pub states: [[Point; 4]; 4],
    pub kick_offsets: [Vec<Point>; 4],
}

pub struct ActivePiece {
    kind: PieceKind,
    definition: PieceDefinition,
    origin: Point,
    rotation: Rotation,
    frames_since_fall: u32,
    rotations_since_descent: u32,
}

impl ActivePiece {
    pub fn spawn(kind: PieceKind, definition: PieceDefinition) -> Self;
    pub fn cells(&self) -> [Point; 4];
    pub fn try_translate(&mut self, dx: i32, dy: i32,
                         board: &DeadPiece) -> bool;
    pub fn try_rotate(&mut self, turn: Turn, board: &DeadPiece) -> bool;
    pub fn hard_drop(&mut self, board: &DeadPiece) -> u32;
    pub fn advance_clock(&mut self);
    pub fn frames_since_fall(&self) -> u32;
    pub fn record_fall(&mut self);
}
```

`spawn` sets the origin to `(4, 20)`, rotation to `Spawn`, and both counters to zero. `cells` returns the four absolute cell positions from the current state; it does not cache another copy of the board.

`try_translate` calculates a proposed origin, gets its four cells, and calls `board.can_place`. It changes the origin only if the entire candidate is valid. A blocked translation returns `false` and leaves the piece unchanged.

`try_rotate` advances the rotation index modulo four, obtains the old/new offset lists, and tries the resulting kicks in order. It commits the new rotation and origin together on the first success. If all candidates fail, neither changes. Section 6 shows the algorithm.

`hard_drop` repeatedly tries `(0, -1)` until it fails and returns the distance moved. A straightforward loop is sufficient for this small board. It does not merge cells, clear lines, choose a new piece, or end the game. Those are kernel decisions.

`advance_clock` increments the frame counter once, preferably with saturating arithmetic. `record_fall` resets that counter and the rotation counter after an actual gravity descent, including a descent accelerated by `D`. The kernel calls these methods; movement geometry does not independently change the clock.

The README lists a rotation counter but does not specify a rotation limit or timer reset rule that uses it. Provisionally, increment `rotations_since_descent` on a successful rotation and reset it on a downward gravity move or spawn. Do not invent an extra lock-reset mechanic from this field. Its exact bookkeeping can be adjusted if clarified without changing movement or locking architecture.

### 4.4 Dead piece: one collection of locked cells

`dead_piece.rs` implements the user's “one giant dead piece” idea directly. It stores occupancy, not a list of old `ActivePiece` objects.

```rust
impl DeadPiece {
    pub fn new() -> Self;
    pub fn occupied(&self, x: usize, y: usize) -> bool;
    pub fn can_place(&self, cells: &[Point; 4]) -> bool;
    pub fn merge(&mut self, cells: &[Point; 4]);
    pub fn clear_full_rows(&mut self) -> usize;
}
```

`can_place` is the shared collision rule used by translations, rotations, and drops:

1. Every cell must have `0 <= x < 10`.
2. Every cell must have `y >= 0`.
3. Every cell must be unoccupied in the locked grid. Rows above the stored grid are empty under the proposed upper-boundary rule.

`occupied` expects a valid column and returns `false` for rows beyond stored data. It is useful for both collision checking and rendering.

`merge` sets the four locked cells to occupied, growing the row vector if necessary. It does not clear lines automatically and does not decide whether the game is over. Separating merge from clear is necessary because top-out is checked between them.

`clear_full_rows` removes all full rows in one bottom-to-top compaction pass and returns the number removed. Preserve each surviving row's contents and relative order. Clear the unused rows at the top afterward. Section 7 gives the exact approach.

### 4.5 Randomizer: preserve the staff sequence

Use the `BagRandomizer<T>` design provided in the README, instantiated with `PieceKind`.

```rust
pub struct BagRandomizer<T> {
    bag: Vec<T>,
    idx: usize,
    rng: Xoshiro128PlusPlus,
}

impl<T> BagRandomizer<T> {
    pub fn from_parts(bag: impl Iterator<Item = T>, seed: [u8; 16]) -> Self;
    pub fn cur(&self) -> &T;
    pub fn next(&mut self);
}
```

Initialize with the exact order `I, J, L, O, S, T, Z`. Seed `Xoshiro128PlusPlus` with the configured seed and shuffle immediately, as in the README. On exhaustion, reshuffle the existing bag in its current order. Rebuilding it in canonical order before each shuffle changes the deterministic sequence.

Use one convention consistently: **`cur()` is the active piece's kind.** At initialization, spawn `cur()` without advancing. After a lock that does not end the game, call `next()` once and spawn the new `cur()`. Do not advance for failed moves, rotations, rendering, or top-out.

No custom randomizer trait is needed. The implementation uses the RNG crate's `SeedableRng` and slice-shuffling extension traits, matching the required versions and the README example.

### 4.6 Kernel: own the game and sequence the work

`game.rs` should read like the per-frame instructions in the README.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameStatus { Running, ToppedOut }

pub struct Game {
    resources: Resources,
    bag: BagRandomizer<PieceKind>,
    active: Option<ActivePiece>,
    dead: DeadPiece,
    status: GameStatus,
}

impl Game {
    pub fn new(resources: Resources) -> Self;
    pub fn step(&mut self, action: Action);
    pub fn is_over(&self) -> bool;
    pub fn active(&self) -> Option<&ActivePiece>;
    pub fn dead(&self) -> &DeadPiece;

    fn finish_lock(&mut self);
    fn spawn_current(&mut self);
    fn tops_out(cells: &[Point; 4]) -> bool;
    fn lock_due(&self) -> bool;
}
```

`Game::new` creates an empty dead piece and the seeded bag, then spawns its current piece. At public method boundaries, a running game has an active piece; a topped-out game has none.

`step` routes an action and handles the timing decisions. It does not contain a wall-kick loop, a hard-drop loop, cell-by-cell collision logic, or row-compaction logic.

For `C` and `W`, the kernel asks `try_rotate` to handle the rotation, including any needed kick. It does not first duplicate the piece's overlap calculation. This is a small refinement of “the kernel decides when a wall trick is required”: the kernel decides when a rotation attempt happens, while the piece determines which geometric candidate works.

`finish_lock` is the only place where an active piece becomes locked cells. Using `Option::take` makes that transition explicit and avoids accidentally rendering the old active piece after it has been merged.

`tops_out` and `lock_due` are small private predicates, not configurable strategies. They keep the two specification-sensitive rules in one place.

### 4.7 Renderer: exact output, no game logic

```rust
pub fn serialize(game: &Game) -> String;
```

No renderer struct is necessary because the renderer owns no persistent state.

Start with a 22-by-10 temporary character grid. Copy the dead-piece occupancy into it, then overlay `game.active().cells()` if an active piece exists. Ignore active or locked cells outside the output's vertical window when drawing; this does not remove them from simulation state. Emit rows from `y = 21` down to `y = 0`, using only `o`, `.`, and newlines.

Return exactly 22 lines of 10 characters, each followed by a newline. `main` writes that string without adding another newline. No borders, labels, scores, ANSI colors, or intermediate frames belong on stdout. Report errors on stderr with a nonzero exit status.

### 4.8 Traits: only the ones that help

There are no custom traits in this design. Each module has one implementation, so concrete types make the relationships easier to follow.

| Area | Useful standard/library traits |
| --- | --- |
| `Action`, `Point`, `PieceKind`, `Rotation`, status | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` for small values and assertions. |
| Piece definitions | `Clone` to give each spawned piece its small definition. |
| JSON wire types | `serde::Deserialize`; map them to internal types if names or layout differ. |
| Randomizer | `SeedableRng` and the required version's slice-shuffling trait. |
| CLI boundary | `std::error::Error`; `std::io::Write` for output. |

For a program this size, `Box<dyn Error>` at the file/CLI boundary is enough. Add a specific error enum only if handling different error kinds becomes useful. Game actions themselves are not errors: an obstructed move simply returns `false`.

## 5. One frame, step by step

The update cycle is driven by the input file. One parsed action means one call to `step`.

```mermaid
flowchart TD
    Start["step(action)"] --> Over{"Already topped out?"}
    Over -->|"yes"| Done["Return"]
    Over -->|"no"| Hard{"Hard drop?"}
    Hard -->|"yes"| Drop["Active piece drops to its last valid position"]
    Drop --> Lock["Take active piece and merge its cells"]
    Hard -->|"no"| Act["Move, rotate, choose soft-drop gravity, or wait"]
    Act --> Clock["Increment this piece's frame counter"]
    Clock --> Gravity{"Gravity due and downward move succeeds?"}
    Gravity -->|"yes"| Reset["Reset frame and rotation counters"]
    Gravity -->|"no"| Delay{"Lock due?"}
    Reset --> Delay
    Delay -->|"no"| Done
    Delay -->|"yes"| Lock
    Lock --> Top{"Locked cells satisfy top-out rule?"}
    Top -->|"yes"| Stop["Mark ToppedOut; keep active empty"]
    Stop --> Done
    Top -->|"no"| Clear["Dead piece clears full rows"]
    Clear --> Next["Advance bag once; spawn at 4,20; reset counters"]
    Next --> Done
```

In pseudocode:

```text
step(action):
    if game is over: return

    if action is H:
        active.hard_drop(dead)
        finish_lock()
        return

    C / W: active.try_rotate(direction, dead)
    L / R: active.try_translate(horizontal_delta, 0, dead)
    D / E: no direct geometric action

    gravity = config.gravity_for_frame(action is D)
    active.advance_clock()

    if active.frames_since_fall >= gravity:
        if active.try_translate(0, -1, dead):
            active.record_fall()

    if lock_due():
        finish_lock()
```

There is at most one gravity descent per ordinary frame, even if the counter has exceeded the interval. A failed gravity move does not reset the counter. A failed input move still consumes its frame, so the clock and gravity checks still happen.

For this proposal, `lock_due` follows the README's literal counter test: `frames_since_fall >= lock_delay_frames`, after the gravity attempt and any successful-fall reset. The README does not explicitly add a “cannot move down” condition. This can matter if the lock delay is smaller than gravity; see Section 10 before treating that edge case as settled.

Hard drop locks immediately. Its branch returns after lock completion so that a newly spawned piece receives neither a clock increment nor gravity in the old piece's frame. Timed locking uses the same completion routine:

```text
finish_lock():
    piece = take active piece
    cells = piece.cells()
    dead.merge(cells)

    if tops_out(cells):
        status = ToppedOut
        return

    dead.clear_full_rows()
    bag.next()
    spawn_current()
```

Checking for line clears only after a lock is sufficient: moving a falling piece does not change locked occupancy, and the previous successful lock already removed all full rows. A loss freezes the merged board before line clearing. The piece that caused the loss remains visible where it intersects the 22-row output window, but no successor is drawn.

## 6. Rotation and wall-kick data flow

Use the shape states and offset data supplied in resources. Do not derive rotated cells with a general rotation matrix or replace the resource tables with remembered Tetris rules.

For a rotation from state `a` to state `b`, candidate `i` uses:

```text
kick[i]       = offsets[a][i] - offsets[b][i]
new_origin    = old_origin + kick[i]
candidate[j]  = new_origin + states[b][j]     for j = 0..3
```

Every candidate starts from the original origin. Do not accumulate failed kicks.

```mermaid
sequenceDiagram
    participant G as Game
    participant P as ActivePiece
    participant B as DeadPiece
    G->>P: try_rotate(Clockwise, board)
    P->>P: Compute target state and ordered offset differences
    loop Candidates in resource order, stop at first success
        P->>P: Build four cells from original origin and candidate kick
        P->>B: can_place(candidate_cells)
        B-->>P: Valid or blocked
        alt Candidate is valid
            P->>P: Commit origin and rotation together
        else Candidate is blocked
            P->>P: Leave original pose unchanged
        end
    end
    P-->>G: true if committed; false if all candidates failed
```

For the README's Normal `0 -> R` example, the ordered candidates are `(0,0)`, `(-1,0)`, `(-1,1)`, `(0,-2)`, `(-1,-2)`. The first valid one wins.

Do not test an extra unshifted rotation before the table. The first offset-derived candidate is already the first permitted test, and it need not be `(0,0)` for every piece. In particular, the linked SRS explanation describes how I/O origin conventions can require a correction even for the first candidate. The resource table should drive this behavior, including the O piece. [SRS offset explanation](https://tetris.wiki/Super_Rotation_System#How_Guideline_SRS_Really_Works).

## 7. Clearing several rows correctly

Use two row indices, `read` and `write`, both starting at the bottom:

```text
write = 0
cleared = 0

for read from 0 to rows.len - 1:
    if rows[read] is full:
        cleared += 1
    else:
        rows[write] = rows[read]
        write += 1

fill rows[write..] with empty rows
return cleared
```

Because `write <= read`, copying a surviving row downward does not overwrite an unread row. This handles adjacent and separated full rows in the same pass.

For example, suppose only these bottom five rows contain cells:

```text
Before, printed top to bottom       After clearing rows 1 and 3

y=4   ..oo......  survives          y=4   ..........
y=3   oooooooooo  removed           y=3   ..........
y=2   o.........  survives          y=2   ..oo......  old row 4
y=1   oooooooooo  removed           y=1   o.........  old row 2
y=0   .o........  survives          y=0   .o........  old row 0
```

This is a row operation. Cells within a surviving row do not fall independently, and the original tetromino identities do not matter.

## 8. End-to-end execution

`main.rs` is only the composition point:

```text
parse command-line arguments
load and validate Resources
read input file and parse Actions
create Game, including the first spawned piece

for action in Actions:
    if game.is_over(): break
    game.step(action)

write render::serialize(game) to stdout
```

For the example input `CCLEEEH`, the parser returns seven actions. The kernel requests two clockwise rotations, a left translation, and three wait frames, then a hard drop. All six ordinary frames still perform their timing/gravity work. The final `H` drops and locks the current piece; if that does not top out, the next piece is spawned before rendering. Whitespace around those letters changes nothing.

The data transformations are:

```text
file bytes
   -> Action values
   -> kernel decisions
   -> proposed piece cells
   -> collision checks against locked cells
   -> committed active pose
   -> locked occupancy when the kernel calls finish_lock
   -> compacted rows if the game continues
   -> locked occupancy + active cells
   -> exactly 22 text lines
```

## 9. Implementation order and useful checks

Build in small steps so the complete input-to-output path works early.

| Step | Implement | Check before moving on |
| --- | --- | --- |
| 1 | Input parsing, resource loading, basic types | `CCLEEEH` and its whitespace variant parse identically; invalid input and bad resource files report useful errors. |
| 2 | Empty dead piece, initial bag, spawning, serialization | Empty input renders the first piece at `(4,20)` with 22 rows of 10 characters. |
| 3 | Collision checks and translations | Walls, floor, and locked cells block movement; blocked movement leaves the pose unchanged. |
| 4 | Hard drop, merge, shared lock completion | A hard drop reaches the last valid position, locks once, and spawns exactly one successor unless topped out. |
| 5 | Row compaction | One, adjacent, and separated full rows clear correctly; surviving row order is preserved. |
| 6 | Rotations and kicks | Clockwise/counterclockwise transitions, a later kick succeeding, all candidates failing, and I/O table handling. |
| 7 | Frame counter, gravity, soft drop, timed locking | Exact `>=` boundaries, one gravity step per frame, failed-fall behavior, and no extra tick on a new piece. |
| 8 | Top-out boundaries and supplied integration checks | Loss happens before a possible clear; trailing actions do nothing; no successor appears. |

The highest-value regression cases are:

- **Seeded sequence:** compare the piece sequence to the README's supplied randomizer with the same seed across multiple bags. Each group of seven must also contain each kind once; that property alone does not prove the exact sequence matches.
- **Atomic failed rotation:** surround a piece so every kick fails; its origin, rotation, and occupied cells must remain the same.
- **Offset direction:** verify the Normal `0 -> R` example yields the exact five differences shown in the README.
- **Gravity boundary:** with an unobstructed piece and gravity `g`, it does not fall at `g - 1` and does fall at `g`. Its counter then resets.
- **Soft-drop frame:** an ordinary interval of `10` and a reduction of `7` gives interval `3` on `D` under the proposed interpretation; `E` on the next frame returns to `10`.
- **Lock boundary:** a blocked piece locks when the counter reaches the delay, not one frame later. Separately test `lock_delay < gravity` to expose the ambiguity rather than accidentally choosing behavior.
- **Top-out ordering:** a locking piece that meets the loss predicate must end the game even if clearing a completed row would have lowered its cells.
- **No successor after loss:** append extra actions to a losing sequence; final output must remain unchanged and the bag must not advance after the loss.
- **Output contract:** 22 rows, each 10 characters, only `.` and `o`, correct vertical order, one final newline, and no extra blank row.

Once implementation is authorized, inspect and run the supplied integration tests using their documented invocation. They were not read or executed for this design. A board is tiny, so ordinary arrays, short loops, and direct comparisons are sufficient; optimize only if a real requirement exposes a problem.

## 10. Specification gaps to keep explicit

The architecture is usable now, but a few behavior details cannot be established confidently from the permitted instructions alone. Keep these visible instead of importing rules from another Tetris game.

| Question | What the README says | Proposed interpretation / implementation location |
| --- | --- | --- |
| Is locking at `y = 20` a loss? | Coordinates are zero-indexed, the board has 20 ordinary rows, and loss is described as any cell “above row 20.” | Follow the literal wording provisionally: `cells.iter().any(\|p\| p.y > 20)`. The alternative `>= 20` follows the ordinary playfield boundary. Isolate this in `Game::tops_out` and check rows 19, 20, and 21 explicitly when clarifying. |
| Must a piece be blocked below before timed locking? | The frame sequence compares the counter to lock delay without an explicit grounded condition. | Follow the written sequence provisionally: check the counter after gravity/reset without adding a grounded condition. If staff require grounding, add a downward-placement check in `Game::lock_due`; nothing else changes. |
| When does a successful fall reset the clock? | The state is described as frames since a forced downward move; resetting on lock is also mentioned explicitly. | Reset on each successful gravity descent and each spawn. Failed gravity, lateral movement, and rotation do not reset it. This follows the counter's meaning, though every reset is not separately enumerated. |
| How is the rotation counter used? | It is listed as state, but no threshold or effect is given. | Track successful rotations since the last gravity descent provisionally; do not make it affect locking without a specified rule. |
| What happens above row 21? | Output is 22 rows, but no complete upper collision rule is provided. | Allow active cells above the output window and retain merged cells if needed. Reject only side/floor bounds and occupancy in `can_place`; crop only during serialization. |
| What exactly does the soft-drop config value mean? | Soft drop temporarily decreases gravity by a configured value. | Subtract for the current frame, saturating at zero, and attempt at most one descent. Confirm the JSON schema and value semantics before coding the loader. |

The top-out link explains partial lock-out in terms of a locked piece extending out of bounds, including when it would clear a line. It supports checking loss before clearing, but does not settle the assignment's exact numeric boundary. [Top-out definitions](https://tetris.wiki/Top_out#Loss_condition).

There is also no explicit separate spawn-overlap loss rule in the reviewed instructions. Do not silently add the wiki's “block out” rule. If a clarification changes the boundary or adds spawn-overlap handling, keep it in the kernel's spawn/loss logic. In particular, the literal `y > 20` interpretation can leave occupancy in row 20, so spawn-overlap behavior is a related edge case that should be checked with the boundary clarification.

The illustrative J JSON in the README contains repeated coordinates in some states and has a trailing comma. Treat it as an illustration of the layout, not the source of shape data. Load the actual supplied resource definitions during implementation and report malformed data rather than quietly repairing it. This appears to be an example-quality issue, not a hidden instruction.

## 11. Sources and status

- [Assignment README: Part 1](README.md#part-1-single-player-tetris), including its input, output, randomizer, frame sequence, rotations, drops, and implementation advice.
- [SRS offset explanation](https://tetris.wiki/Super_Rotation_System#How_Guideline_SRS_Really_Works), followed directly from the README for rotation context.
- [Top-out definitions](https://tetris.wiki/Top_out#Loss_condition), followed directly from the README for loss terminology.

Only this design document was added. The existing README, resources, tests, and repository configuration were left unchanged. No implementation, dependency installation, or test run is claimed here.
