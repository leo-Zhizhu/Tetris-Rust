use std::{
    env,
    error::Error,
    fs,
    io::{self, Write},
    process,
};

use p5_tetris::{game::Game, input, render, resources::Resources};

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args_os().skip(1).collect();

    let options = input::parse_args(args.into_iter());
    let resources = Resources::load(&options.resources_dir);
    let bytes = fs::read(&options.inputs_path)
        .map_err(|error| format!("{}: {error}", options.inputs_path.display()))?;
    let actions = input::parse_actions(&bytes);
    let mut game = Game::new(resources);

    for action in actions {
        if game.is_over() {
            break;
        }
        game.step(action);
    }

    io::stdout()
        .lock()
        .write_all(render::serialize(&game).as_bytes())?;
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error}");
        process::exit(1);
    }
}
