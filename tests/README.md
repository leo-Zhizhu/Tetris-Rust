# Crucial Part 1 tests

Run `cargo test` from the project root. There are **9 tests**: eight focused gameplay tests and one CLI test that runs all **12 supplied single-player cases**.

The suite focuses on correct gameplay with valid inputs. Each test function has a purpose comment, and the setup/action/assertion blocks have comments explaining what they establish. The supplied resources and expected-output files are never modified.

## Focused gameplay tests

```sh
cargo test --test gameplay
```

Expected summary: `test result: ok. 8 passed; 0 failed`.

| Test | Why it stays |
| --- | --- |
| `input_actions_and_whitespace_are_parsed_correctly` | Checks the meaning of all seven controls and ensures whitespace does not add frames. |
| `gravity_and_lock_delay_use_the_correct_frames` | Checks no premature airborne lock, descent exactly on frame 20, locking exactly on frame 415, and the successor's reset timer. |
| `soft_drop_handles_normal_and_zero_intervals` | Checks the normal 20-to-10 gravity reduction and accepted reductions that reach zero. Prevents the unsigned-subtraction crash found during review; zero still permits only one descent per frame. |
| `rotation_uses_the_first_valid_floor_kick` | Checks that rotation tries candidates in order and accepts the first one that fits above the floor. |
| `blocked_rotation_keeps_the_original_piece` | Checks an ordinary blocked rotation: when no kick works, the original pose and rotation counter remain unchanged. |
| `i_and_o_pieces_use_their_own_kick_tables` | Checks I's origin correction and O's unchanged visible square through all four rotations. |
| `full_rows_clear_without_reordering_survivors` | Checks adjacent and separated full rows, the positions of surviving cells, the empty rows at the top, and doing nothing when no full rows remain. |
| `top_out_precedes_line_clearing_and_freezes_the_game` | Checks simultaneous line completion and top-out on a small board: loss takes priority, no successor spawns, and later moves cannot change the board. |

To run just one test:

```sh
cargo test --test gameplay rotation_uses_the_first_valid_floor_kick
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
