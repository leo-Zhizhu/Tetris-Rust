use std::path::Path;

use p5_tetris::{
    dead_piece::DeadPiece,
    game::Game,
    input::{self, Action},
    piece::{ActivePiece, PieceKind, Point, Turn},
    render,
    resources::Resources,
};

fn resources() -> Resources {
    Resources::load(Path::new("resources"))
}

fn piece(kind: PieceKind) -> ActivePiece {
    ActivePiece::spawn(kind, resources().definition(kind).clone(), 10, 20)
}

fn wait(game: &mut Game, frames: usize) {
    for _ in 0..frames {
        game.step(Action::Wait);
    }
}

#[test]
fn input_and_whitespace() {

    let expected = vec![
        Action::RotateClockwise,
        Action::RotateCounterclockwise,
        Action::Left,
        Action::Right,
        Action::SoftDrop,
        Action::HardDrop,
        Action::Wait,
    ];
    assert_eq!(input::parse_actions(b"CWLRDHE"), expected);

    assert_eq!(
        input::parse_actions(b" C W\nL R\tD H E\n"),
        expected
    );
}

#[test]
fn gravity_and_lock() {
    let mut game = Game::new(resources());
    let initial = game.active().unwrap().cells();

    wait(&mut game, 19);
    assert!(!game.is_over());
    assert_eq!(game.active().unwrap().cells(), initial);
    game.step(Action::Wait);
    // The first piece is L. One fall moves its origin from (4, 20) to (4, 19).
    assert_eq!(
        game.active().unwrap().cells(),
        [
            Point { x: 3, y: 19 },
            Point { x: 4, y: 19 },
            Point { x: 5, y: 19 },
            Point { x: 5, y: 20 },
        ]
    );
    assert_eq!(game.active().unwrap().frames_since_fall(), 0);

    wait(&mut game, 380);
    assert!(!game.active().unwrap().can_move_down(game.dead()));
    assert_eq!(game.active().unwrap().frames_since_fall(), 0);
    wait(&mut game, 14);
    assert_eq!(game.active().unwrap().kind(), PieceKind::L);
    assert!(!game.dead().occupied(3, 0));

    game.step(Action::Wait);
    assert!(game.dead().occupied(3, 0));
    assert_eq!(game.active().unwrap().kind(), PieceKind::T);
    assert_eq!(game.active().unwrap().frames_since_fall(), 0);
}

#[test]
fn soft_drop() {
    let data = resources();
    assert_eq!(data.config.gravity_for_frame(false), 20);
    assert_eq!(data.config.gravity_for_frame(true), 10);

    for reduction in [20, 30] {
        let mut data = resources();
        data.config.gravity_decrease = reduction;
        assert_eq!(data.config.gravity_for_frame(true), 0);
        assert_eq!(data.config.gravity_for_frame(false), 20);

        let mut game = Game::new(data);
        game.step(Action::SoftDrop);
        // Even with a zero interval, the L moves down only one row this frame.
        assert_eq!(
            game.active().unwrap().cells(),
            [
                Point { x: 3, y: 19 },
                Point { x: 4, y: 19 },
                Point { x: 5, y: 19 },
                Point { x: 5, y: 20 },
            ]
        );
        assert_eq!(game.active().unwrap().frames_since_fall(), 0);
    }
}

#[test]
fn rotation() {
    let board = DeadPiece::new(10, 20);
    let mut active = piece(PieceKind::T);
    active.hard_drop(&board);

    assert!(active.try_rotate(Turn::Clockwise, &board));
    assert_eq!(
        active.cells(),
        [
            Point { x: 3, y: 1 },
            Point { x: 3, y: 2 },
            Point { x: 4, y: 1 },
            Point { x: 3, y: 0 }
        ]
    );
}

#[test]
fn blocked_rotation() {
    let mut board = DeadPiece::new(10, 20);
    let mut active = piece(PieceKind::T);
    active.hard_drop(&board);
    let original = active.cells();

    board.merge(&[
        Point { x: 3, y: 1 },
        Point { x: 3, y: 2 },
        Point { x: 2, y: 2 },
        Point { x: 2, y: 3 },
    ]);

    assert!(!active.try_rotate(Turn::Clockwise, &board));
    assert_eq!(active.cells(), original);
    assert_eq!(active.rotations_since_descent(), 0);
}

#[test]
fn i_and_o_pieces_use_their_own_kick_tables() {
    let board = DeadPiece::new(10, 20);

    // I's first clockwise rotation needs a one-column origin correction.
    let mut i = piece(PieceKind::I);
    assert!(i.try_rotate(Turn::Clockwise, &board));
    assert_eq!(
        i.cells(),
        [
            Point { x: 5, y: 21 },
            Point { x: 5, y: 20 },
            Point { x: 5, y: 19 },
            Point { x: 5, y: 18 }
        ]
    );

    let mut o = piece(PieceKind::O);
    let original = o.cells();
    for _ in 0..4 {
        assert!(o.try_rotate(Turn::Clockwise, &board));
        assert_eq!(o.cells(), original);
    }
}


#[test]
fn rows_clear() {
    let cases = [
        (
            [
                [
                    Point { x: 0, y: 0 },
                    Point { x: 3, y: 3 },
                    Point { x: 0, y: 4 },
                    Point { x: 1, y: 5 },
                ],
                [
                    Point { x: 0, y: 1 },
                    Point { x: 1, y: 1 },
                    Point { x: 2, y: 1 },
                    Point { x: 3, y: 1 },
                ],
                [
                    Point { x: 0, y: 2 },
                    Point { x: 1, y: 2 },
                    Point { x: 2, y: 2 },
                    Point { x: 3, y: 2 },
                ],
            ],
            [
                "....",
                "....",
                "....",
                "....",
                ".o..",
                "o...",
                "...o",
                "o...",
            ],
        ),

        (
            [
                [
                    Point { x: 0, y: 0 },
                    Point { x: 2, y: 2 },
                    Point { x: 0, y: 4 },
                    Point { x: 1, y: 5 },
                ],
                [
                    Point { x: 0, y: 1 },
                    Point { x: 1, y: 1 },
                    Point { x: 2, y: 1 },
                    Point { x: 3, y: 1 },
                ],
                [
                    Point { x: 0, y: 3 },
                    Point { x: 1, y: 3 },
                    Point { x: 2, y: 3 },
                    Point { x: 3, y: 3 },
                ],
            ],

            [
                "....",
                "....",
                "....",
                "....",
                ".o..",
                "o...",
                "..o.",
                "o...",
            ],
        ),
    ];

    for (input, expected) in cases {
        let mut board = DeadPiece::new(4, 6);

        for cells in input {
            board.merge(&cells);
        }
        assert_eq!(board.clear_full_rows(), 2);

        let mut actual = Vec::new();
        for y in (0..8).rev() {
            let mut row = String::new();
            for x in 0..4 {
                if board.occupied(x, y) {
                    row.push('o');
                } else {
                    row.push('.');
                }
            }
            actual.push(row);
        }
        assert_eq!(actual, expected);

        assert_eq!(board.clear_full_rows(), 0);
    }
}


#[test]
fn top_out() {

    let mut data = resources();
    data.config.width = 3;
    data.config.height = 1;
    let mut game = Game::new(data);
    game.step(Action::HardDrop);

    assert!(game.is_over());
    assert!(game.active().is_none());
    assert!(game.dead().occupied(0, 0));
    assert!(game.dead().occupied(1, 0));
    assert!(game.dead().occupied(2, 0));
    assert!(game.dead().occupied(2, 1));

    let final_board = render::serialize(&game);
    for action in [
        Action::Left,
        Action::Wait,
        Action::RotateClockwise,
        Action::HardDrop,
    ] {
        game.step(action);
        assert_eq!(render::serialize(&game), final_board);
    }
}
