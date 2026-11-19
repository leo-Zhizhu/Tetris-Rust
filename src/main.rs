use std::{
    env,
    error::Error,
    fs,
    io::{self, Write},
    process,
};

use p5_tetris::{
    bot::{BadBot, GoodBot, MultiPlayerBot},
    game::Game,
    input::{self, Action, CliOptions},
    multiplayer::MultiplayerGame,
    render,
    resources::Resources,
};
use std::path::{Path, PathBuf};

fn read_actions(path: &Path) -> Result<Vec<Action>, Box<dyn Error>> {
    let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(input::parse_actions(&bytes))
}

const BOT_NAMES: [&str; 2] = ["good-bot", "bad-bot"];

fn make_bot(name: &str, config: &Path, player: usize) -> Result<Box<dyn MultiPlayerBot>, String> {
    if name == BOT_NAMES[0] {
        Ok(Box::new(GoodBot::for_player(config, player)))
    } else if name == BOT_NAMES[1] {
        Ok(Box::new(BadBot::new(config)))
    } else {
        Err(format!("unknown bot: {name}"))
    }
}

fn trace_match(
    config: &Path,
    players: [String; 2],
    outputs: [PathBuf; 2],
) -> Result<String, Box<dyn Error>> {
    let mut bots = [
        make_bot(&players[0], config, 0)?,
        make_bot(&players[1], config, 1)?,
    ];
    let mut game = MultiplayerGame::new(std::array::from_fn(|_| Resources::load(config)));
    let mut traces = [Vec::new(), Vec::new()];
    while !game.games().iter().any(Game::is_over) {
        let actions = bots.each_ref().map(|bot| bot.get_move());
        game.step(actions.map(Into::into));
        for i in 0..2 {
            bots[i].update_with(&actions);
            traces[i].push(actions[i] as u8);
        }
    }
    for (output, trace) in outputs.iter().zip(traces) {
        fs::write(output, trace).map_err(|error| format!("{}: {error}", output.display()))?;
    }
    Ok(render::serialize_multiplayer(game.games()))
}

fn run() -> Result<(), Box<dyn Error>> {
    let options = input::parse_args(env::args_os().skip(1))?;
    let output = match options {
        CliOptions::ListBots => format!("{}\n", BOT_NAMES.join("\n")),
        CliOptions::Trace {
            resources,
            players,
            outputs,
        } => trace_match(&resources, players, outputs)?,
        CliOptions::Single {
            resources,
            input: path,
        } => {
            let mut game = Game::new(Resources::load(&resources));
            for action in read_actions(&path)? {
                if game.is_over() {
                    break;
                }
                game.step(action);
            }
            render::serialize(&game)
        }
        CliOptions::Multi {
            resources,
            inputs: [p1, p2],
        } => {
            let traces = [read_actions(&p1)?, read_actions(&p2)?];
            let mut game =
                MultiplayerGame::new(std::array::from_fn(|_| Resources::load(&resources)));
            for frame in 0..traces[0].len().max(traces[1].len()) {
                if game.games().iter().any(Game::is_over) {
                    break;
                }
                game.step(
                    traces
                        .each_ref()
                        .map(|trace| trace.get(frame).copied().unwrap_or(Action::Wait)),
                );
            }
            render::serialize_multiplayer(game.games())
        }
    };
    io::stdout().lock().write_all(output.as_bytes())?;
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error}");
        process::exit(1);
    }
}
