# Run and try Part 1

Part 1 simulates a file of moves and prints the final board. Each non-whitespace action advances one frame. The program is not a real-time keyboard game; you can append moves and replay the file to continue a deterministic game.

## 1. Open the project and build

```sh
cd "/Users/zhizhu/Desktop/DreamFold梦中梦/CS 4414/codebase/assignments/p5-tetris"
cargo build
```

Expected: Cargo finishes successfully and creates `target/debug/p5_tetris`. The build should have no warnings. Compilation time and the exact Cargo status messages can vary.

## 2. Run the focused gameplay suite

```sh
cargo test --test gameplay
```

Expected: eight tests end in `ok`, followed by a summary containing:

```text
test result: ok. 8 passed; 0 failed
```

These cover controls, gravity/locking timing, soft-drop intervals, wall kicks, row clearing, and top-out. [tests/README.md](tests/README.md) explains why each test is included.

## 3. Run the supplied single-player suite

```sh
cargo test --test cli
```

Expected output includes:

```text
test supplied_single_player_input ... ok
```

The summary says `1 passed; 0 failed`, because one Rust test compares the output strings for all 12 input/output pairs.

To run both suites together, use:

```sh
cargo test
```

Expected: one CLI test and eight gameplay tests pass, with zero failures. The empty library, binary, and documentation test sections may also say `0 tests`; that is normal.

## 4. Display the initial piece

Create a fresh demo file inside the ignored `target/` directory:

```sh
mkdir -p target/part1-demo
printf '' > target/part1-demo/moves.txt
./target/debug/p5_tetris --simulate-inputs-single-player target/part1-demo/moves.txt --resources resources
```

Expected output, exactly 22 rows of 10 characters:

```text
.....o....
...ooo....
..........
..........
..........
..........
..........
..........
..........
..........
..........
..........
..........
..........
..........
..........
..........
..........
..........
..........
..........
..........
```

The seeded first piece is L. It is still in the two spawn rows. `o` means occupied and `.` means empty. Rows print from top to bottom.

## 5. Hard-drop the first piece

```sh
printf 'H\n' >> target/part1-demo/moves.txt
./target/debug/p5_tetris --simulate-inputs-single-player target/part1-demo/moves.txt --resources resources
```

Expected output:

```text
....o.....
...ooo....
..........
..........
..........
..........
..........
..........
..........
..........
..........
..........
..........
..........
..........
..........
..........
..........
..........
..........
.....o....
...ooo....
```

The L is locked at the bottom. The next piece, T, appears at the top. Hard drop locks immediately without waiting for a timer.

## 6. Move the next piece right and drop it

```sh
printf 'RH\n' >> target/part1-demo/moves.txt
./target/debug/p5_tetris --simulate-inputs-single-player target/part1-demo/moves.txt --resources resources
```

Expected output:

```text
....oo....
....oo....
..........
..........
..........
..........
..........
..........
..........
..........
..........
..........
..........
..........
..........
..........
..........
..........
.....o....
....ooo...
.....o....
...ooo....
```

The T is locked above the L, one column to the right. The next piece is O, visible as a square in the top two rows.

Each invocation starts from the same seed and replays the entire input. Appending moves therefore extends the same game. Spaces and line breaks do not advance time. The outputs above were checked against the running program, and both drop examples also match the supplied `.expect` files.

## 7. Try your own controls

| Character | Action |
| --- | --- |
| `L` | Move one column left if possible. |
| `R` | Move one column right if possible. |
| `C` | Rotate clockwise, trying wall kicks when needed. |
| `W` | Rotate counterclockwise. |
| `D` | Apply faster gravity for this frame. |
| `H` | Drop to the last valid position and lock immediately. |
| `E` | Wait for one frame; gravity and locking still run. |

For example, start a fresh file with 20 wait frames:

```sh
printf 'EEEEEEEEEEEEEEEEEEEE\n' > target/part1-demo/moves.txt
./target/debug/p5_tetris --simulate-inputs-single-player target/part1-demo/moves.txt --resources resources
```

Expected: the first L moves down one row. The first three printed rows become:

```text
..........
.....o....
...ooo....
```

The remaining 19 rows are empty. Replacing the input with ten `D` characters produces the same board with the supplied gravity settings.

For repeated entry of short move sequences, this optional shell loop prints a new board after each line:

```sh
printf '' > target/part1-demo/moves.txt
while true; do
    printf 'Moves (C W L R D H E), or q to quit: '
    IFS= read -r moves || break
    [ "$moves" = q ] && break
    printf '%s\n' "$moves" >> target/part1-demo/moves.txt
    ./target/debug/p5_tetris --simulate-inputs-single-player target/part1-demo/moves.txt --resources resources || break
done
```

Enter uppercase moves such as `LLCH` or `RRH`. Nothing advances while you think or type. Enter `q` to leave the loop.

## 8. Inspect a line clear or game over

```sh
./target/debug/p5_tetris --simulate-inputs-single-player cli-tests/single-player/line-clear.txt --resources resources
./target/debug/p5_tetris --simulate-inputs-single-player cli-tests/single-player/topped-out.txt --resources resources
```

Expected: each board matches the same-named `.expect` file in `cli-tests/single-player/`. On top-out, the losing piece is merged, line clearing stops, no successor is drawn, and remaining actions are ignored.

To check one manually:

```sh
./target/debug/p5_tetris --simulate-inputs-single-player cli-tests/single-player/line-clear.txt --resources resources > target/part1-demo/line-clear.out
diff -u cli-tests/single-player/line-clear.expect target/part1-demo/line-clear.out
```

Expected: `diff` prints nothing, meaning the files match.

## 9. Optional release checks and packaging

```sh
cargo test --release --test gameplay
cargo package --allow-dirty
```

Expected: the eight gameplay tests pass in release mode. The CLI test uses a hardcoded debug executable path, so run it with normal `cargo test` as shown above. Packaging builds and verifies `target/package/p5_tetris-0.1.0.crate`; it does not publish it. `--allow-dirty` permits your current uncommitted changes to be packaged. Cargo may show an optional package-metadata warning.

The original `runt cli-tests` configuration also runs multiplayer tests, which are outside Part 1. The Cargo suites above select single-player behavior only.
