use std::path::Path;

use p5_tetris::{
    bot::{SimpleBot, SinglePlayerBot, TetrisAction},
    game::Game,
    render,
    resources::Resources,
};

#[test]
fn single_player_clears_40_lines() {
    let mut game = Game::new(Resources::load(Path::new("resources")));
    let mut bot = SimpleBot::new(Path::new("resources"));

    for _ in 0..20_000 {
        if game.is_over() || game.lines_cleared() >= 40 {
            break;
        }
        let action = bot.get_move();
        game.step(action.into());
        bot.update_with(action);
    }
    assert!(game.lines_cleared() >= 40, "too few lines");
}

#[test]
fn bot_updates_follow_inputs() {
    let mut game = Game::new(Resources::load(Path::new("resources")));
    let mut bot = SimpleBot::new(Path::new("resources"));
    let mut fresh_bot = SimpleBot::new(Path::new("resources"));

    bot.get_move();
    let inputs = [
        TetrisAction::E,
        TetrisAction::L,
        TetrisAction::D,
        TetrisAction::H,
    ];
    for action in inputs {
        game.step(action.into());
        bot.update_with(action);
        fresh_bot.update_with(action);
    }

    assert_eq!(render::serialize(bot.game()), render::serialize(&game));
    let expected = fresh_bot.get_move();
    assert_eq!(bot.get_move(), expected);
    assert_eq!(bot.get_move(), expected);
}
