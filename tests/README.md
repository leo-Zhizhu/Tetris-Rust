# Tests

Run `cargo test` from the project root. There are **24 correctness tests**: eight bot unit tests, seven bot integration tests, eight gameplay tests, and one CLI test that runs all **12 supplied single-player cases**. One additional speed comparison is ignored unless explicitly requested.

The suite focuses on correct gameplay with valid inputs. The descriptions below explain what each test checks. The supplied resources and expected-output files are never modified.

## Bot search and worker tests

The unit tests are in `src/bot/tests.rs` so they can check the private search and controller without expanding the public bot interface.

```sh
cargo test --lib
```

Expected summary: `8 passed; 0 failed; 1 ignored`.

| Test | What it checks |
| --- | --- |
| `counterclockwise_shortcut` | A hardcoded J-piece well is filled using `W,H`, instead of three clockwise turns. |
| `move_before_rotating` | A floor and roof initially block both I-piece rotations. The search moves left before rotating and clears four rows. |
| `descend_then_slide_into_gap` | An O descends through an opening and slides left under a roof. Every action is replayed, and the final board is compared with three explicit rows. |
| `frame_rules_include_gravity_and_locking` | Ten soft-drop frames cause one fall; a sideways move does not reset the grounded piece's 15-frame lock timer. |
| `search_stops_at_timed_lock` | A grounded piece with 14 elapsed frames locks on the next action. The returned path ends there. |
| `search_checks_top_out_before_row_clears` | A vertical I completes two rows but extends above the board. The search rejects this landing before clearing those rows. |
| `tracked_timer_and_saved_path_match_game` | The bot consumes a saved path, preserves its hidden timer across calls, matches the real game through gravity and soft drop, and resets at the next spawn. |
| `parallel_search_matches_sequential` | The same jobs produce the same path and score in both modes. Two searches keep the same four thread IDs, and dropping the pool finishes cleanup. |

## Bot integration tests

```sh
cargo test --test bot_test
cargo test --release --test bot_test simple_bot_test -- --exact --nocapture
```

The seven tests in `tests/bot_test.rs` cover the required public interface, game snapshots, scoring, simulation without changing the real board, board conversion, piece conversion, and a complete 40-line game.

`simple_bot_test` uses only the supplied seed. It stops at 40 cleared lines, game over, or 20,000 frames, then asserts that at least 40 lines were cleared. The current result is 40 lines in 428 frames. Elapsed time depends on the machine and whether this is a debug or release build.

## Optional speed comparison

```sh
cargo test --release --lib compare_search_speed -- --ignored --nocapture
```

This runs the same bounded search sequentially and on four persistent workers. Both versions play three complete games using the supplied seed, and every returned action must match. It prints the median time for each version, including pool creation and shutdown. It does not assert a time threshold, since machine load varies.

The bot searches at most 20,000 states per job, with paths up to 300 frames. It keeps a chosen path until the piece locks. These are search limits, not changes to the game's movement rules. Four workers share one job queue and one result queue; their local search storage is reused between jobs.

## Focused gameplay tests

```sh
cargo test --test gameplay
```

Expected summary: `test result: ok. 8 passed; 0 failed`.

| Test | Why it stays |
| --- | --- |
| `input_and_whitespace` | Checks the meaning of all seven controls and ensures whitespace does not add frames. |
| `gravity_and_lock` | Checks no premature airborne lock, descent exactly on frame 20, locking exactly on frame 415, and the successor's reset timer. |
| `soft_drop` | Checks the normal 20-to-10 gravity reduction and accepted reductions that reach zero. Prevents the unsigned-subtraction crash found during review; zero still permits only one descent per frame. |
| `rotation` | Checks that rotation tries candidates in order and accepts the first one that fits above the floor. |
| `blocked_rotation` | Checks an ordinary blocked rotation: when no kick works, the original pose and rotation counter remain unchanged. |
| `i_and_o_pieces_use_their_own_kick_tables` | Checks I's origin correction and O's unchanged visible square through all four rotations. |
| `rows_clear` | Checks adjacent and separated full rows, the positions of surviving cells, the empty rows at the top, and doing nothing when no full rows remain. |
| `top_out` | Checks simultaneous line completion and top-out on a small board: loss takes priority, no successor spawns, and later moves cannot change the board. |

To run just one test:

```sh
cargo test --test gameplay rotation -- --exact
```

Expected summary: `1 passed; 0 failed; 7 filtered out`.

## Supplied single-player cases

```sh
cargo test --test cli
```

The test lists the 12 case names directly and uses simple relative paths. For each case, it runs `./target/debug/p5_tetris`, reads the matching `.expect` file, and compares the two complete strings with `assert_eq!(actual, expected)`, including the final newline. Run it with normal `cargo test` because the executable path is fixed to the debug build.

| Case | Main functionality |
| --- | --- |
| `drop-after-doing-nothing` | Gravity moves a piece down after 20 wait frames. |
| `drop-two-after-doing-nothing` | Long simulation combines repeated descent, timed locking, spawning, and fresh counters. |
| `hard-drop` | Hard drop locks the first piece immediately and displays its successor. |
| `hard-drop-right-hard-drop` | Successive pieces collide correctly with locked cells. |
| `last-minute-soft-drop` | A soft-drop frame uses the time already accumulated by wait actions. |
| `line-clear` | Movement, rotation, hard drops, line removal, and the seeded sequence across a bag boundary work together. |
| `move-left` | Repeated left movement stops at the wall. |
| `move-right` | Repeated right movement stops at the wall. |
| `rotate-counter-clockwise` | Counterclockwise rotation produces the correct shape and position. |
| `soft-drop` | Ten soft-drop frames produce one descent. |
| `start-soft-drop-but-stop` | Returning to wait actions restores ordinary gravity. |
| `topped-out` | The losing piece remains on the board, no successor is drawn, and trailing moves are ignored. |

Expected summary: `test result: ok. 1 passed; 0 failed`. Cargo reports one test because one Rust test function loops over the 12 files.

Basic movement, hard drops, normal soft drops, seeded spawning, and board serialization already have complete supplied examples. Redundant focused tests for those same scenarios were removed. The remaining focused tests check boundaries and interactions that the supplied final boards do not isolate.

The original `runt cli-tests` configuration also includes multiplayer cases. Use these Cargo test suites to check Part 1 alone.
