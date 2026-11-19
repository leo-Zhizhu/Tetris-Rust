use std::path::PathBuf;

use p5_tetris::bot::{MultiPlayerBot, SinglePlayerBot};

pub type MySinglePlayerBot = p5_tetris::bot::SimpleBot;
pub type MyMultiPlayerBot = p5_tetris::bot::GoodBot;

#[test]
fn create_bot_and_get_any_action_test() {
    let config_path: PathBuf = "resources".into();
    let mut bot = MySinglePlayerBot::new(&config_path);
    let action = bot.get_move();
    bot.update_with(action);
}

#[test]
fn create_mut_bot_and_get_any_action_test() {
    let config_path: PathBuf = "resources".into();
    let mut bot = MyMultiPlayerBot::new(&config_path);
    let action = bot.get_move();
    bot.update_with(&[action, p5_tetris::bot::TetrisAction::E]);
}
