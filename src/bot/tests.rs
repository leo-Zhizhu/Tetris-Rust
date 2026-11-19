use std::{path::Path, sync::Arc};

use crate::{
    game::step_piece,
    piece::{PieceKind, Point, Rotation},
    render,
};

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
    let pool = SimulationControl::new(Arc::clone(&resources));
    let workers = pool.worker_ids();
    assert_eq!(workers.len(), 4);
    assert_eq!(pool.search(Arc::clone(&context)), expected);
    assert_eq!(pool.search(context), expected);
    assert_eq!(pool.worker_ids(), workers);
    // This must close the queue and join the workers, not leave them waiting.
    drop(pool);
}

#[test]
fn updates_and_saved_path_match_game() {
    let mut game = Game::new(Resources::load(Path::new("resources")));
    let mut bot = SimpleBot::new(Path::new("resources"));
    // Exercise gravity, soft drop, locking, and spawning along a saved path.
    let mut path = vec![TetrisAction::E; 20];
    path.extend([TetrisAction::D; 10]);
    path.extend([TetrisAction::L, TetrisAction::H]);
    *bot.actions.get_mut() = path.into();

    for _ in 0..32 {
        let before = render::serialize(&bot.game);
        let timer = bot.game.active().unwrap().frames_since_fall();
        let action = bot.get_move();
        assert_eq!(bot.get_move(), action);
        assert_eq!(render::serialize(&bot.game), before);
        assert_eq!(bot.game.active().unwrap().frames_since_fall(), timer);
        game.step(action.into());
        bot.update_with(action);
        assert_eq!(render::serialize(&bot.game), render::serialize(&game));
        assert_eq!(
            bot.game.active().unwrap().frames_since_fall(),
            game.active().unwrap().frames_since_fall()
        );
    }
    assert!(bot.actions.borrow().is_empty());
    assert_eq!(bot.game.active().unwrap().frames_since_fall(), 0);

    // Applying a different action must invalidate the cached plan.
    *bot.actions.get_mut() = VecDeque::from([TetrisAction::L, TetrisAction::H]);
    bot.update_with(TetrisAction::R);
    game.step(Action::Right);
    assert!(bot.actions.borrow().is_empty());
    assert_eq!(render::serialize(&bot.game), render::serialize(&game));
    let action = bot.get_move();
    bot.update_with(action);
    game.step(action.into());
    assert_eq!(render::serialize(&bot.game), render::serialize(&game));

    // Updates also work without a preceding get_move, through top-out.
    while !game.is_over() {
        game.step(Action::HardDrop);
        bot.update_with(TetrisAction::H);
    }
    assert!(bot.game.is_over());
    assert_eq!(bot.get_move(), TetrisAction::E);
    bot.update_with(TetrisAction::L);
    assert_eq!(render::serialize(&bot.game), render::serialize(&game));
}

#[test]
fn bot_uses_supplied_resources() {
    let dir = std::env::temp_dir().join(format!("p5-tetris-bot-resources-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for file in ["tetrominoes.json", "wall_kicks.json"] {
        std::fs::copy(Path::new("resources").join(file), dir.join(file)).unwrap();
    }
    let mut config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string("resources/config.json").unwrap()).unwrap();
    config["gravity"] = 1.into();
    config["random_seed"] = serde_json::json!([7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7]);
    std::fs::write(dir.join("config.json"), config.to_string()).unwrap();
    let mut game = Game::new(Resources::load(&dir));
    let mut bot = SimpleBot::new(&dir);
    std::fs::remove_dir_all(&dir).unwrap();
    bot.update_with(TetrisAction::E);
    game.step(Action::Wait);
    assert_eq!(bot.game.active().unwrap().origin().y, 19);
    assert_eq!(render::serialize(&bot.game), render::serialize(&game));
    let action = bot.get_move();
    bot.update_with(action);
    game.step(action.into());
    assert_eq!(render::serialize(&bot.game), render::serialize(&game));
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
