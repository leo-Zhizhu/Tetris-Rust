use std::path::Path;

use p5_tetris::{
    game::Game,
    input::{Action, parse_actions},
    multiplayer::MultiplayerGame,
    render,
    resources::Resources,
};

fn resources() -> Resources {
    let mut resources = Resources::load(Path::new("resources"));
    resources.config.width = 4;
    resources.config.random_seed = [5; 16];
    resources
}

fn bottom_rows(game: &Game, count: usize) -> Vec<String> {
    let mut rows = Vec::new();
    for row in 0..count {
        let mut text = String::new();
        for column in 0..4 {
            if game.dead().occupied(column, row) {
                text.push('o');
            } else {
                text.push('.');
            }
        }
        rows.push(text);
    }
    rows
}

#[test]
fn garbage_waits_for_expiry_and_lock() {
    let mut game = MultiplayerGame::new([resources(), resources()]);

    for action in parse_actions(b"H LH CCRH RH CH CRH CLH").unwrap() {
        game.step([action, Action::Wait]);
    }
    assert_eq!(game.games()[0].lines_cleared(), 3);

    game.step([Action::Wait, Action::HardDrop]);
    assert_eq!(bottom_rows(&game.games()[1], 3), [".oo.", "oo..", "...."]);

    for _ in 0..9 {
        game.step([Action::Wait, Action::Wait]);
    }
    assert_eq!(bottom_rows(&game.games()[1], 3), [".oo.", "oo..", "...."]);

    game.step([Action::Wait, Action::HardDrop]);
    assert_eq!(
        bottom_rows(&game.games()[1], 7),
        ["oo.o", "oo.o", ".oo.", "oo..", ".oo.", ".oo.", "...."]
    );
}

#[test]
fn attack_cancels_pending_garbage() {
    let mut game = MultiplayerGame::new([resources(), resources()]);

    for action in parse_actions(b"CH LH CCCRR").unwrap() {
        game.step([action, action]);
    }
    game.step([Action::HardDrop, Action::Wait]);
    for _ in 0..9 {
        game.step([Action::Wait, Action::Wait]);
    }

    game.step([Action::Wait, Action::HardDrop]);
    assert_eq!(
        bottom_rows(&game.games()[1], 4),
        [".o..", ".oo.", "...o", "...."]
    );
    for _ in 0..10 {
        game.step([Action::Wait, Action::Wait]);
    }
    game.step([Action::HardDrop, Action::Wait]);
    assert_eq!(bottom_rows(&game.games()[0], 1), [".o.."]);
}

#[test]
fn simultaneous_attacks_do_not_cancel() {
    let mut game = MultiplayerGame::new([resources(), resources()]);

    for action in parse_actions(b"CH LH CCCRRH").unwrap() {
        game.step([action, action]);
    }
    for _ in 0..10 {
        game.step([Action::Wait, Action::Wait]);
    }
    game.step([Action::HardDrop, Action::HardDrop]);
    assert_eq!(bottom_rows(&game.games()[0], 2), ["oo.o", ".o.."]);
    assert_eq!(bottom_rows(&game.games()[1], 2), ["oo.o", ".o.."]);
}

#[test]
fn garbage_top_out() {
    let mut second = resources();
    second.config.height = 3;
    second.config.gravity = 1000;
    let mut game = MultiplayerGame::new([resources(), second]);

    for action in parse_actions(b"H LH CCRH RH CH CRH CLH").unwrap() {
        game.step([action, Action::Wait]);
    }
    for _ in 0..10 {
        game.step([Action::Wait, Action::Wait]);
    }
    game.step([Action::Wait, Action::HardDrop]);
    assert!(game.games()[1].is_over());

    let final_board = render::serialize(&game.games()[1]);
    game.step([Action::Wait, Action::HardDrop]);
    assert_eq!(render::serialize(&game.games()[1]), final_board);
}
