use p5_tetris::{
    bot::{MultiPlayerBot, SinglePlayerBot, board_score},
    dead_piece::DeadPiece,
    game::Game,
    resources::Resources,
};
use std::{
    path::{Path, PathBuf},
    time::Instant,
};

pub type MyMultiPlayerBot = p5_tetris::bot::GoodBot;

#[test]
fn create_mut_bot_and_get_any_action_test() {
    let config_path: PathBuf = "resources".into();
    let mut bot = MyMultiPlayerBot::new(&config_path);
    let m = bot.get_move();
    bot.update_with(&[m, p5_tetris::bot::TetrisAction::E]);
}

pub type MySinglePlayerBot = p5_tetris::bot::SimpleBot;

#[test]
fn create_bot_and_get_any_action_test() {
    let config_path: PathBuf = "resources".into();
    let mut bot = MySinglePlayerBot::new(&config_path);
    let m = bot.get_move();
    bot.update_with(m);
}

#[test]
fn simple_bot_test() {
    let config = Path::new("resources");
    let mut game = Game::new(Resources::load(config));
    let mut bot = MySinglePlayerBot::new(config);
    // Use the supplied seed. The frame limit makes a stalled bot fail.
    let start = Instant::now();
    let mut frames = 0;

    while !game.is_over() && game.lines_cleared() < 40 && frames < 20_000 {
        let action = bot.get_move();
        game.step(action.into());
        bot.update_with(action);
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
    let board = DeadPiece::new(10, 20);
    assert_eq!(board_score(&board, 0), 0);
    let mut grid = [[false; 10]; 20];
    grid[0][4] = true;
    grid[1][4] = true;
    let stacked = DeadPiece::from_grid(&grid);
    grid[0][4] = false;
    let with_hole = DeadPiece::from_grid(&grid);
    assert!(board_score(&with_hole, 0) > board_score(&stacked, 0));
    assert!(board_score(&board, 1) < board_score(&board, 0));
}
