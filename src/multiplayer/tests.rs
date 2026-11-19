use std::path::Path;

use super::*;
use crate::{piece::PieceKind, randomizer::BagRandomizer, render};

fn multiplayer() -> MultiplayerGame {
    MultiplayerGame::new(std::array::from_fn(|_| {
        Resources::load(Path::new("resources"))
    }))
}

// Set up a vertical I above an aligned well using ordinary game operations.
fn well_game(rows: usize) -> Game {
    let mut resources = Resources::load(Path::new("resources"));
    resources.config.random_seed = (0..=u8::MAX)
        .map(|value| [value; 16])
        .find(|&seed| {
            *BagRandomizer::from_parts(PieceKind::ALL.into_iter(), seed).cur() == PieceKind::I
        })
        .unwrap();
    let mut game = Game::new(resources);
    game.add_garbage(rows, 5);
    assert!(!game.step(Action::RotateClockwise));
    game
}

#[test]
fn line_clears_send_only_excess_rows() {
    for rows in 1..=4 {
        let mut game = multiplayer();
        game.games[0] = well_game(rows);
        game.step([Action::HardDrop, Action::Wait]);
        assert_eq!(game.games[0].lines_cleared(), rows);
        if rows == 1 {
            assert!(game.queues[1].is_empty());
        } else {
            assert_eq!(game.queues[1].len(), 1);
            assert_eq!(game.queues[1][0].rows, rows - 1);
            // New timers are decremented at the end of the sending frame.
            assert_eq!(game.queues[1][0].timer, 9);
        }
        assert!(game.queues[0].is_empty());
    }
}

#[test]
fn attacks_cancel_oldest_rows_before_applying_garbage() {
    let mut game = multiplayer();
    game.games = [well_game(4), well_game(4)];
    game.queues[0] = VecDeque::from([Garbage { rows: 2, timer: 0 }, Garbage { rows: 2, timer: 5 }]);
    game.queues[1].push_back(Garbage { rows: 1, timer: 4 });
    game.step([Action::HardDrop; 2]);
    // Player 0 cancels three old rows, including the expired batch, and
    // receives two new rows from player 1's surplus attack.
    let remaining: Vec<_> = game.queues[0].iter().map(|g| (g.rows, g.timer)).collect();
    assert_eq!(remaining, vec![(1, 4), (2, 9)]);
    assert!(game.queues[1].is_empty());
    for player in game.games() {
        assert_eq!(player.lines_cleared(), 4);
        for x in 0..10 {
            assert!(!player.dead().occupied(x, 0));
        }
    }
}

#[test]
fn simultaneous_attacks_do_not_cancel_each_other() {
    let mut game = multiplayer();
    game.games = [well_game(4), well_game(4)];
    game.step([Action::HardDrop; 2]);
    for queue in &game.queues {
        assert_eq!(queue.len(), 1);
        assert_eq!((queue[0].rows, queue[0].timer), (3, 9));
    }
}

#[test]
fn garbage_waits_for_expiry_and_lock_and_draws_once_per_batch() {
    let resources = Resources::load(Path::new("resources"));
    let mut expected_rng = Xoshiro128PlusPlus::from_seed(resources.config.random_seed);
    let mut control = Game::new(resources);
    let mut game = multiplayer();
    game.queues[0].push_back(Garbage { rows: 2, timer: 2 });
    for action in [Action::HardDrop, Action::Wait, Action::Wait] {
        game.step([action, Action::Wait]);
        control.step(action);
        // A lock before expiry and waits after expiry must not add garbage.
        assert_eq!(
            render::serialize(&game.games[0]),
            render::serialize(&control)
        );
        assert_eq!(
            game.rngs[0].clone().next_u32(),
            expected_rng.clone().next_u32()
        );
    }
    assert_eq!(game.queues[0][0].timer, 0);
    game.step([Action::HardDrop, Action::Wait]);
    control.step(Action::HardDrop);
    let first_hole = expected_rng.next_u32() as usize % 10;
    for row in 0..2 {
        for col in 0..10 {
            assert_eq!(game.games[0].dead().occupied(col, row), col != first_hole);
        }
    }
    for row in 0..20 {
        for col in 0..10 {
            assert_eq!(
                game.games[0].dead().occupied(col, row + 2),
                control.dead().occupied(col, row)
            );
        }
    }
    // Garbage shifts the locked board, not the newly spawned piece or bag.
    assert_eq!(
        game.games[0].active().unwrap().kind(),
        control.active().unwrap().kind()
    );
    assert_eq!(
        game.games[0].active().unwrap().cells(),
        control.active().unwrap().cells()
    );
    assert!(game.queues[0].is_empty());

    // Separate batches get separate draws; the other player has its own RNG.
    game.queues[0].push_back(Garbage { rows: 3, timer: 0 });
    game.queues[1].push_back(Garbage { rows: 2, timer: 0 });
    game.step([Action::HardDrop; 2]);
    let second_hole = expected_rng.next_u32() as usize % 10;
    for row in 0..3 {
        for col in 0..10 {
            assert_eq!(game.games[0].dead().occupied(col, row), col != second_hole);
        }
    }
    for row in 0..2 {
        for col in 0..10 {
            assert_eq!(game.games[1].dead().occupied(col, row), col != first_hole);
        }
    }
    assert_eq!(game.rngs[0].next_u32(), expected_rng.next_u32());
}

#[test]
fn timed_lock_applies_expired_garbage() {
    let mut game = multiplayer();
    game.queues[0].push_back(Garbage { rows: 1, timer: 0 });
    for _ in 0..414 {
        game.step([Action::Wait; 2]);
    }
    assert_eq!(game.queues[0].len(), 1);
    assert_eq!(game.games[0].active().unwrap().kind(), PieceKind::L);
    game.step([Action::Wait; 2]);
    assert!(game.queues[0].is_empty());
    assert_eq!(game.games[0].active().unwrap().kind(), PieceKind::T);
    assert_eq!(game.games[0].active().unwrap().frames_since_fall(), 0);
    for col in 0..10 {
        assert_eq!(
            game.games[0].dead().occupied(col, 1),
            game.games[1].dead().occupied(col, 0)
        );
    }
    assert_eq!(
        (0..10)
            .filter(|&col| game.games[0].dead().occupied(col, 0))
            .count(),
        9
    );
}

#[test]
fn top_out_stops_further_garbage_and_random_draws() {
    let mut game = multiplayer();
    let mut expected_rng = game.rngs[0].clone();
    game.queues[0] = VecDeque::from([
        Garbage { rows: 19, timer: 0 },
        Garbage { rows: 1, timer: 0 },
    ]);
    game.step([Action::HardDrop, Action::Wait]);
    assert!(game.games[0].is_over());
    assert!(game.games[0].active().is_none());
    assert!(game.games[0].dead().occupied(5, 20));
    assert_eq!(game.queues[0].len(), 1);
    expected_rng.next_u32();
    assert_eq!(game.rngs[0].clone().next_u32(), expected_rng.next_u32());
    let final_board = render::serialize(&game.games[0]);
    assert_eq!(final_board.lines().count(), 22);
    game.step([Action::HardDrop; 2]);
    assert_eq!(render::serialize(&game.games[0]), final_board);

    // A piece that itself tops out must not receive any pending garbage.
    let mut game = multiplayer();
    let mut resources = Resources::load(Path::new("resources"));
    resources.config.height = 1;
    game.games[0] = Game::new(resources);
    game.queues[0].push_back(Garbage { rows: 1, timer: 0 });
    let mut expected_rng = game.rngs[0].clone();
    game.step([Action::HardDrop, Action::Wait]);
    assert!(game.games[0].is_over());
    assert_eq!(game.queues[0].len(), 1);
    assert_eq!(game.rngs[0].next_u32(), expected_rng.next_u32());
}
