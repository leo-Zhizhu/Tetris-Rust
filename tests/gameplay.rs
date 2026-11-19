use std::path::Path;

use p5_tetris::{
    dead_piece::DeadPiece,
    game::Game,
    input::{Action, parse_actions},
    piece::PieceKind,
    resources::Resources,
};

#[test]
fn input_and_whitespace() {
    let actions = parse_actions(b"C W\nL R\tD H E\n").unwrap();
    assert_eq!(
        actions,
        [
            Action::RotateClockwise,
            Action::RotateCounterclockwise,
            Action::Left,
            Action::Right,
            Action::SoftDrop,
            Action::HardDrop,
            Action::Wait,
        ]
    );
}

#[test]
fn gravity_and_lock() {
    let mut game = Game::new(Resources::load(Path::new("resources")));

    for _ in 0..19 {
        game.step(Action::Wait);
    }
    assert_eq!(game.active().unwrap().origin().y, 20);
    game.step(Action::Wait);
    assert_eq!(game.active().unwrap().origin().y, 19);

    for _ in 0..394 {
        game.step(Action::Wait);
    }
    assert_eq!(game.active().unwrap().kind(), PieceKind::L);
    assert!(game.step(Action::Wait));
    assert!(game.dead().occupied(3, 0));
    assert_eq!(game.active().unwrap().kind(), PieceKind::T);
}

#[test]
fn rows_clear() {
    let input = [
        "o.........",
        "oooooooooo",
        "..o.......",
        "oooooooooo",
        "....o.....",
        ".....o....",
    ];
    let mut grid = [[false; 10]; 20];
    for row in 0..6 {
        for column in 0..10 {
            grid[row][column] = input[row].as_bytes()[column] == b'o';
        }
    }
    let mut board = DeadPiece::from_grid(&grid);
    assert_eq!(board.clear_full_rows(), 2);

    let mut actual = Vec::new();
    for row in 0..6 {
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
    assert_eq!(
        actual,
        [
            "o.........",
            "..o.......",
            "....o.....",
            ".....o....",
            "..........",
            "..........",
        ]
    );
}
