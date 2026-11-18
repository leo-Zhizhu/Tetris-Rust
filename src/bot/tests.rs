use std::{path::Path, sync::Arc};

use super::{
    search::{ACTIONS, SearchContext, SearchWorker, choose_result},
    simulation::SimulationControl,
    *,
};

#[test]
fn counterclockwise_shortcut() {
    let resources = Resources::load(Path::new("resources"));
    let mut grid = [[false; 10]; 20];
    // Bottom row first. The two-column well admits the J in either vertical
    // orientation, but turning left fits directly without moving sideways.
    let rows = ["ooo..ooooo", "ooo..ooooo", "ooo..ooooo"];
    for row in 0..3 {
        for col in 0..10 {
            grid[row][col] = rows[row].as_bytes()[col] == b'o';
        }
    }
    let context = SearchContext {
        board: DeadPiece::from_grid(&grid),
        piece: ActivePiece::from_pose(
            PieceKind::J,
            resources.definition(PieceKind::J).clone(),
            Point { x: 4, y: 5 },
            Rotation::Spawn,
        ),
    };
    let mut worker = SearchWorker::new();
    let mut results = Vec::new();
    for action in ACTIONS {
        results.push(worker.search(&context, &resources.config, action));
    }
    let best = choose_result(results).unwrap();
    // W,H is shorter than C,C,C,H. Its bottom two cells clear one row.
    assert_eq!(best.actions, vec![TetrisAction::W, TetrisAction::H]);
    assert_eq!(best.score, 94);
}

#[test]
fn parallel_search_matches_sequential() {
    let resources = Arc::new(Resources::load(Path::new("resources")));
    let context = Arc::new(SearchContext {
        board: DeadPiece::new(10, 20),
        piece: ActivePiece::from_pose(
            PieceKind::T,
            resources.definition(PieceKind::T).clone(),
            Point { x: 4, y: 2 },
            Rotation::Spawn,
        ),
    });
    let mut worker = SearchWorker::new();
    let mut results = Vec::new();
    for action in ACTIONS {
        results.push(worker.search(&context, &resources.config, action));
    }
    let expected = choose_result(results);

    // Two searches on one controller must give the same complete path and
    // score as running those exact seven jobs sequentially.
    let mut pool = SimulationControl::new(Arc::clone(&resources));
    let workers = pool.worker_ids();
    assert_eq!(workers.len(), 4);
    assert_eq!(pool.search(Arc::clone(&context)), expected);
    assert_eq!(pool.search(context), expected);
    assert_eq!(pool.worker_ids(), workers);
    // This must close the queue and join the workers, not leave them waiting.
    drop(pool);
}

#[test]
fn tracked_timer_and_saved_path_match_game() {
    let mut game = Game::new(Resources::load(Path::new("resources")));
    let mut bot = SimpleBot::new();
    // Give the bot a valid path that crosses a gravity fall and a soft drop.
    // It must keep its counters and consume the saved path one frame at a time.
    bot.actions = vec![TetrisAction::E; 20];
    for _ in 0..10 {
        bot.actions.push(TetrisAction::D);
    }
    bot.actions.push(TetrisAction::L);
    bot.actions.push(TetrisAction::H);

    for _ in 0..31 {
        let state = BoardState::from_game(&game).unwrap();
        let action = bot.get_move(&state);
        game.step(action.into());
        let predicted = bot.predicted_piece.as_ref().unwrap();
        let actual = game.active().unwrap();
        assert_eq!(predicted.cells(), actual.cells());
        assert_eq!(predicted.frames_since_fall(), actual.frames_since_fall());
    }
    assert_eq!(game.active().unwrap().origin(), Point { x: 3, y: 18 });
    assert_eq!(game.active().unwrap().frames_since_fall(), 1);
    let state = BoardState::from_game(&game).unwrap();
    assert_eq!(bot.get_move(&state), TetrisAction::H);
    game.step(Action::HardDrop);
    assert!(bot.predicted_piece.is_none());
    assert!(bot.actions.is_empty());

    // The successor starts with a fresh timer; the old path is finished.
    bot.actions = vec![TetrisAction::E, TetrisAction::H];
    let state = BoardState::from_game(&game).unwrap();
    assert_eq!(bot.get_move(&state), TetrisAction::E);
    game.step(Action::Wait);
    assert_eq!(bot.predicted_piece.as_ref().unwrap().frames_since_fall(), 1);
    assert_eq!(
        bot.predicted_piece.as_ref().unwrap().cells(),
        game.active().unwrap().cells()
    );
}

#[test]
#[ignore] // Run explicitly in release mode; timings are not correctness assertions.
fn compare_search_speed() {
    use std::time::Instant;

    let mut sequential_times = Vec::new();
    let mut parallel_times = Vec::new();
    let mut expected_actions = Vec::new();
    for round in 0..3 {
        // Alternate order so one version does not always get the first run.
        let order = if round % 2 == 0 {
            [false, true]
        } else {
            [true, false]
        };
        for parallel in order {
            let resources = Arc::new(Resources::load(Path::new("resources")));
            let mut game = Game::new(Resources::load(Path::new("resources")));
            let start = Instant::now();
            let mut pool = None;
            if parallel {
                pool = Some(SimulationControl::new(Arc::clone(&resources)));
            }
            let mut worker = SearchWorker::new();
            let mut actions = Vec::new();
            while !game.is_over() && game.lines_cleared() < 40 && actions.len() < 20_000 {
                let context = Arc::new(SearchContext {
                    board: game.dead().clone(),
                    piece: game.active().unwrap().clone(),
                });
                let result = if let Some(pool) = &mut pool {
                    pool.search(context)
                } else {
                    let mut results = Vec::new();
                    for action in ACTIONS {
                        results.push(worker.search(&context, &resources.config, action));
                    }
                    choose_result(results)
                };
                let path = match result {
                    Some(result) => result.actions,
                    None => vec![TetrisAction::H],
                };
                for action in path {
                    game.step(action.into());
                    actions.push(action);
                }
            }
            // Include pool startup and shutdown in the measured time.
            drop(pool);
            let elapsed = start.elapsed();
            assert!(game.lines_cleared() >= 40, "too few lines");
            if expected_actions.is_empty() {
                expected_actions = actions;
            } else {
                assert_eq!(actions, expected_actions);
            }
            if parallel {
                parallel_times.push(elapsed);
            } else {
                sequential_times.push(elapsed);
            }
        }
    }
    sequential_times.sort();
    parallel_times.sort();
    println!("Sequential: {:?}", sequential_times[1]);
    println!("Four workers: {:?}", parallel_times[1]);
}

#[test]
fn frame_rules_include_gravity_and_locking() {
    let resources = Resources::load(Path::new("resources"));
    let board = DeadPiece::new(10, 20);
    let mut piece = ActivePiece::from_pose(
        PieceKind::O,
        resources.definition(PieceKind::O).clone(),
        Point { x: 4, y: 1 },
        Rotation::Spawn,
    );

    // Nine soft-drop frames do not move the O. The tenth moves it to row 0
    // and resets the timer, leaving time to slide along the floor.
    for _ in 0..9 {
        assert!(!step_piece(
            &mut piece,
            &board,
            &resources.config,
            Action::SoftDrop
        ));
    }
    assert_eq!(piece.origin(), Point { x: 4, y: 1 });
    assert!(!step_piece(
        &mut piece,
        &board,
        &resources.config,
        Action::SoftDrop
    ));
    assert_eq!(piece.origin(), Point { x: 4, y: 0 });
    assert_eq!(piece.frames_since_fall(), 0);

    // Moving sideways consumes a frame without resetting the lock timer.
    assert!(!step_piece(
        &mut piece,
        &board,
        &resources.config,
        Action::Left
    ));
    for _ in 0..13 {
        assert!(!step_piece(
            &mut piece,
            &board,
            &resources.config,
            Action::Wait
        ));
    }
    assert_eq!(piece.frames_since_fall(), 14);
    assert!(step_piece(
        &mut piece,
        &board,
        &resources.config,
        Action::Wait
    ));
    assert_eq!(piece.origin(), Point { x: 3, y: 0 });
}

#[test]
fn search_stops_at_timed_lock() {
    let resources = Resources::load(Path::new("resources"));
    let mut piece = ActivePiece::from_pose(
        PieceKind::O,
        resources.definition(PieceKind::O).clone(),
        Point { x: 4, y: 0 },
        Rotation::Spawn,
    );
    // The O is grounded with only one frame left before locking. Moving left
    // uses that frame. The search must not append a rotation or hard drop.
    for _ in 0..14 {
        piece.advance_clock();
    }
    let context = SearchContext {
        board: DeadPiece::new(10, 20),
        piece,
    };
    let mut worker = SearchWorker::new();
    let best = worker
        .search(&context, &resources.config, TetrisAction::L)
        .unwrap();
    assert_eq!(best.actions, vec![TetrisAction::L]);
    assert_eq!(best.score, 32);
}

#[test]
fn search_checks_top_out_before_row_clears() {
    let resources = Resources::load(Path::new("resources"));
    let mut grid = [[false; 10]; 20];
    grid[17][4] = true;
    for col in 0..10 {
        if col != 4 {
            grid[18][col] = true;
            grid[19][col] = true;
        }
    }
    // The vertical I completes rows 18 and 19, but its other two cells are
    // above the board. Clearing those rows must not turn a loss into a win.
    let context = SearchContext {
        board: DeadPiece::from_grid(&grid),
        piece: ActivePiece::from_pose(
            PieceKind::I,
            resources.definition(PieceKind::I).clone(),
            Point { x: 4, y: 20 },
            Rotation::Right,
        ),
    };
    let mut worker = SearchWorker::new();
    assert!(
        worker
            .search(&context, &resources.config, TetrisAction::H)
            .is_none()
    );
}

#[test]
fn descend_then_slide_into_gap() {
    let resources = Resources::load(Path::new("resources"));
    let mut grid = [[false; 10]; 20];
    // Bottom row first. The roof above columns 2 and 3 prevents dropping
    // straight into the left half of the gap. Enter through columns 4 and 5.
    let rows = ["oo....oooo", "oo....oooo", "oooo..oooo"];
    for row in 0..3 {
        for col in 0..10 {
            grid[row][col] = rows[row].as_bytes()[col] == b'o';
        }
    }
    let context = SearchContext {
        board: DeadPiece::from_grid(&grid),
        piece: ActivePiece::from_pose(
            PieceKind::O,
            resources.definition(PieceKind::O).clone(),
            Point { x: 4, y: 4 },
            Rotation::Spawn,
        ),
    };
    let mut worker = SearchWorker::new();
    let mut results = Vec::new();
    for action in ACTIONS {
        results.push(worker.search(&context, &resources.config, action));
    }
    let best = choose_result(results).unwrap();
    let mut piece = context.piece.clone();
    let mut descended = false;
    let mut slid = false;
    for index in 0..best.actions.len() {
        let action = best.actions[index];
        let old_cells = piece.cells();
        let locked = step_piece(&mut piece, &context.board, &resources.config, action.into());
        let cells = piece.cells();
        if cells[0].y < old_cells[0].y && action == TetrisAction::D {
            descended = true;
        }
        if descended && action == TetrisAction::L && cells[0].x < old_cells[0].x {
            slid = true;
        }
        // Every earlier action must leave the piece active.
        assert_eq!(locked, index == best.actions.len() - 1);
    }
    assert!(descended);
    assert!(slid);
    assert_eq!(best.score, 138);
    let mut board = context.board.clone();
    board.merge(&piece.cells());
    let expected = ["oooo..oooo", "oooo..oooo", "oooo..oooo"];
    for row in 0..3 {
        for col in 0..10 {
            assert_eq!(
                board.occupied(col, row),
                expected[row].as_bytes()[col] == b'o'
            );
        }
    }
}

#[test]
fn move_before_rotating() {
    let resources = Resources::load(Path::new("resources"));
    let mut grid = [[false; 10]; 20];
    // Bottom row first. The I starts in the horizontal opening at row 4.
    // The floor and roof block both turns until it moves left out of the roof.
    let rows = [
        "o.oooooooo",
        "o.oooooooo",
        "o.oooooooo",
        "o.oooooooo",
        "..........",
        "....oooo..",
        "....oooo..",
        "....oooo..",
    ];
    for row in 0..8 {
        for col in 0..10 {
            grid[row][col] = rows[row].as_bytes()[col] == b'o';
        }
    }
    let context = SearchContext {
        board: DeadPiece::from_grid(&grid),
        piece: ActivePiece::from_pose(
            PieceKind::I,
            resources.definition(PieceKind::I).clone(),
            Point { x: 5, y: 4 },
            Rotation::Spawn,
        ),
    };
    let mut blocked = context.piece.clone();
    assert!(!blocked.try_rotate(crate::piece::Turn::Counterclockwise, &context.board));
    assert!(!blocked.try_rotate(crate::piece::Turn::Clockwise, &context.board));
    let mut worker = SearchWorker::new();
    let mut results = Vec::new();
    for action in ACTIONS {
        results.push(worker.search(&context, &resources.config, action));
    }
    let best = choose_result(results).unwrap();
    assert_eq!(
        best.actions,
        vec![
            TetrisAction::L,
            TetrisAction::W,
            TetrisAction::W,
            TetrisAction::C,
            TetrisAction::H,
        ]
    );

    // Replay the whole path. It must remain active until the hard drop,
    // then occupy column 1 and clear the four bottom rows.
    let mut piece = context.piece.clone();
    for index in 0..best.actions.len() {
        let locked = step_piece(
            &mut piece,
            &context.board,
            &resources.config,
            best.actions[index].into(),
        );
        assert_eq!(locked, index == 4);
    }
    let mut board = context.board.clone();
    board.merge(&piece.cells());
    assert_eq!(board.clear_full_rows(), 4);
    assert_eq!(best.score, 152);
}
