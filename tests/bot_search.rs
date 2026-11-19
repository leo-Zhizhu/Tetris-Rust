use std::{path::Path, sync::Arc};

use p5_tetris::{
    bot::{
        TetrisAction,
        search::{ACTIONS, SearchContext, SearchWorker, choose_result},
        simulation::SimulationControl,
    },
    dead_piece::DeadPiece,
    game::step_piece,
    piece::{ActivePiece, PieceKind, Point, Rotation},
    resources::Resources,
};

#[test]
fn counterclockwise_shortcut() {
    let resources = Resources::load(Path::new("resources"));
    let mut grid = [[false; 10]; 20];

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
    assert_eq!(best.actions, vec![TetrisAction::W, TetrisAction::H]);
}

#[test]
fn move_before_rotating() {
    let resources = Resources::load(Path::new("resources"));
    let mut grid = [[false; 10]; 20];
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
    let mut worker = SearchWorker::new();
    let mut results = Vec::new();
    for action in ACTIONS {
        results.push(worker.search(&context, &resources.config, action));
    }
    let best = choose_result(results).unwrap();
    assert_eq!(best.actions[0], TetrisAction::L);

    let mut piece = context.piece.clone();
    for index in 0..best.actions.len() {
        let locked = step_piece(
            &mut piece,
            &context.board,
            &resources.config,
            best.actions[index].into(),
        );
        assert_eq!(locked, index == best.actions.len() - 1);
    }
    let mut board = context.board.clone();
    board.merge(&piece.cells());
    assert_eq!(board.clear_full_rows(), 4);
}

#[test]
fn descend_then_slide_into_gap() {
    let resources = Resources::load(Path::new("resources"));
    let mut grid = [[false; 10]; 20];
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
    for index in 0..best.actions.len() {
        let locked = step_piece(
            &mut piece,
            &context.board,
            &resources.config,
            best.actions[index].into(),
        );
        assert_eq!(locked, index == best.actions.len() - 1);
    }

    let mut board = context.board.clone();
    board.merge(&piece.cells());
    let mut actual = Vec::new();
    for row in 0..3 {
        let mut text = String::new();
        for column in 0..10 {
            if board.occupied(column, row) {
                text.push('o');
            } else {
                text.push('.');
            }
        }
        actual.push(text);
    }
    assert_eq!(actual, ["oooo..oooo", "oooo..oooo", "oooo..oooo"]);
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

    let pool = SimulationControl::new(Arc::clone(&resources));
    assert_eq!(pool.search(Arc::clone(&context)), expected);
    assert_eq!(pool.search(context), expected);
    drop(pool);
}
