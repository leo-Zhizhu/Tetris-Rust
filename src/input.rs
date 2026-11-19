use std::{ffi::OsString, path::PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    RotateClockwise,
    RotateCounterclockwise,
    Left,
    Right,
    SoftDrop,
    HardDrop,
    Wait,
}

#[derive(Debug)]
pub enum CliOptions {
    ListBots,
    Single {
        resources: PathBuf,
        input: PathBuf,
    },
    Multi {
        resources: PathBuf,
        inputs: [PathBuf; 2],
    },
    Trace {
        resources: PathBuf,
        players: [String; 2],
        outputs: [PathBuf; 2],
    },
}

pub fn parse_args(mut args: impl Iterator<Item = OsString>) -> Result<CliOptions, &'static str> {
    let mut single = None;
    let mut multi = Vec::new();
    let mut resources = None;
    let mut list = false;
    let mut trace = false;
    let mut player1 = None;
    let mut player2 = None;
    let mut output1 = None;
    let mut output2 = None;

    while let Some(flag) = args.next() {
        match flag.to_str() {
            Some("--resources") => {
                resources = Some(PathBuf::from(args.next().ok_or("invalid input")?));
            }
            Some("--list-bots") => list = true,
            Some("--trace-bot-run") => trace = true,
            Some("--simulate-inputs-single-player") => {
                single = Some(PathBuf::from(args.next().ok_or("invalid input")?));
            }
            Some("--simulate-inputs-multi-player") => {
                multi.push(PathBuf::from(args.next().ok_or("invalid input")?));
            }
            Some("--p1") => {
                player1 = Some(
                    args.next()
                        .ok_or("invalid input")?
                        .into_string()
                        .map_err(|_| "invalid input")?,
                );
            }
            Some("--p2") => {
                player2 = Some(
                    args.next()
                        .ok_or("invalid input")?
                        .into_string()
                        .map_err(|_| "invalid input")?,
                );
            }
            Some("--o1") => {
                output1 = Some(PathBuf::from(args.next().ok_or("invalid input")?));
            }
            Some("--o2") => {
                output2 = Some(PathBuf::from(args.next().ok_or("invalid input")?));
            }
            _ => return Err("invalid input"),
        }
    }

    if list {
        return Ok(CliOptions::ListBots);
    }
    let resources = resources.ok_or("invalid input")?;
    if trace {
        return Ok(CliOptions::Trace {
            resources,
            players: [
                player1.ok_or("invalid input")?,
                player2.ok_or("invalid input")?,
            ],
            outputs: [
                output1.ok_or("invalid input")?,
                output2.ok_or("invalid input")?,
            ],
        });
    }
    if let Some(input) = single {
        return Ok(CliOptions::Single { resources, input });
    }
    if multi.len() != 2 {
        return Err("invalid input");
    }
    Ok(CliOptions::Multi {
        resources,
        inputs: [multi[0].clone(), multi[1].clone()],
    })
}

pub fn parse_actions(bytes: &[u8]) -> Result<Vec<Action>, &'static str> {
    let mut actions = Vec::new();
    for &byte in bytes {
        let action = match byte {
            b'C' => Action::RotateClockwise,
            b'W' => Action::RotateCounterclockwise,
            b'L' => Action::Left,
            b'R' => Action::Right,
            b'D' => Action::SoftDrop,
            b'H' => Action::HardDrop,
            b'E' => Action::Wait,
            byte if byte.is_ascii_whitespace() => continue,
            _ => return Err("invalid input"),
        };
        actions.push(action);
    }
    Ok(actions)
}
