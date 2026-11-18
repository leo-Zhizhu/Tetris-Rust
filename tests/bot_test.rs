use p5_tetris::{
    bot::{BoardState, CellState, SinglePlayerBot, TetrominoType, board_score, landing_score},
    game::Game,
    input::Action,
    render,
    resources::Resources,
};
use std::{path::Path, time::Instant};

pub type MySinglePlayerBot = p5_tetris::bot::SimpleBot;

#[test]
fn create_bot_and_get_any_action_test() {
    // The assignment requires this public interface to accept a spawn snapshot.
    let mut bot = MySinglePlayerBot::new();
    let _ = bot.get_move(&BoardState {
        tetromino_row: 20,
        tetromino_col: 4,
        tetromino_rotation: 0,
        tetromino: TetrominoType::J,
        board: [[CellState::Empty; 10]; 20],
    });
}

#[test]
fn game_to_board_state_test() {
    // A fresh snapshot contains the spawn pose and no locked cells.
    let game = Game::new(Resources::load(Path::new("resources")));
    let state = BoardState::from_game(&game).unwrap();
    assert_eq!(state.tetromino_row, 20);
    assert_eq!(state.tetromino_col, 4);
    assert_eq!(state.tetromino_rotation, 0);
    for row in 0..20 {
        for col in 0..10 {
            assert_eq!(state.board[row][col], CellState::Empty);
        }
    }
}

#[test]
fn simple_bot_test() {
    let mut game = Game::new(Resources::load(Path::new("resources")));
    let mut bot = MySinglePlayerBot::new();
    // Use the supplied seed. The frame limit makes a stalled bot fail.
    let start = Instant::now();
    let mut frames = 0;

    while !game.is_over() && game.lines_cleared() < 40 && frames < 20_000 {
        let state = BoardState::from_game(&game).unwrap();
        let action = bot.get_move(&state);
        game.step(action.into());
        frames += 1;
    }
    assert!(game.lines_cleared() >= 40, "too few lines");
    println!(
        "Lines: {}, frames: {}, game over: {}, elapsed: {:?}",
        game.lines_cleared(),
        frames,
        game.is_over(),
        start.elapsed()
    );
}

#[test]
fn board_score_test() {
    // A hole makes the board worse; clearing a row improves its score.
    let board = [[CellState::Empty; 10]; 20];
    assert_eq!(board_score(&board, 0), 0);
    let mut stacked = board;
    stacked[0][4] = CellState::Filled;
    stacked[1][4] = CellState::Filled;
    let mut with_hole = stacked;
    with_hole[0][4] = CellState::Empty;
    assert!(board_score(&with_hole, 0) > board_score(&stacked, 0));
    assert!(board_score(&board, 1) < board_score(&board, 0));
}

#[test]
fn landing_score_test() {
    // Trying a landing must not change the real game.
    let game = Game::new(Resources::load(Path::new("resources")));
    let before = render::serialize(&game);
    let score = landing_score(game.active().unwrap(), game.dead());
    assert!(score.is_some());
    assert_eq!(render::serialize(&game), before);
}

#[test]
fn get_dead_piece_test() {
    // After one hard drop, every locked cell must survive the conversion.
    let mut game = Game::new(Resources::load(Path::new("resources")));
    game.step(Action::HardDrop);
    let state = BoardState::from_game(&game).unwrap();
    let dead_piece = state.get_dead_piece();
    for row in 0..20 {
        for col in 0..10 {
            assert_eq!(
                dead_piece.occupied(col, row),
                game.dead().occupied(col, row)
            );
        }
    }
}

#[test]
fn get_active_piece_test() {
    // Movement, rotation, and a gravity fall must survive the pose conversion.
    let resources = Resources::load(Path::new("resources"));
    let mut game = Game::new(Resources::load(Path::new("resources")));
    game.step(Action::Left);
    game.step(Action::RotateClockwise);
    for _ in 0..20 {
        game.step(Action::Wait);
    }
    let state = BoardState::from_game(&game).unwrap();
    let active_piece = state.get_active_piece(&resources);
    let original = game.active().unwrap();
    assert_eq!(active_piece.kind(), original.kind());
    assert_eq!(active_piece.origin(), original.origin());
    assert_eq!(active_piece.rotation(), original.rotation());
    assert_eq!(active_piece.cells(), original.cells());
}
