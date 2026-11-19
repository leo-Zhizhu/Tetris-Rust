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

pub fn parse_args(mut args: impl Iterator<Item = OsString>) -> Result<CliOptions, String> {
    let mut single = None;
    let mut multi = Vec::new();
    let mut resources = None;
    let mut list = false;
    let mut trace = false;
    let mut players = [None, None];
    let mut outputs = [None, None];

    while let Some(flag) = args.next() {
        let mut value = || {
            args.next()
                .ok_or_else(|| format!("missing value for {}", flag.to_string_lossy()))
        };
        match flag.to_str() {
            Some("--resources") => resources = Some(PathBuf::from(value()?)),
            Some("--list-bots") => list = true,
            Some("--trace-bot-run") => trace = true,
            Some("--simulate-inputs-single-player") if single.is_none() => {
                single = Some(PathBuf::from(value()?))
            }
            Some("--simulate-inputs-multi-player") => multi.push(PathBuf::from(value()?)),
            Some("--p1" | "--p2") => {
                players[usize::from(flag == "--p2")] =
                    Some(value()?.into_string().map_err(|_| "invalid bot name")?);
            }
            Some("--o1" | "--o2") => {
                outputs[usize::from(flag == "--o2")] = Some(PathBuf::from(value()?))
            }
            _ => return Err(format!("unexpected argument: {}", flag.to_string_lossy())),
        }
    }

    let modes = [list, trace, single.is_some(), !multi.is_empty()];
    if modes.into_iter().filter(|&selected| selected).count() != 1 {
        return Err("select exactly one replay, bot list, or trace mode".into());
    }
    if !trace && (players.iter().any(Option::is_some) || outputs.iter().any(Option::is_some)) {
        return Err("player and output flags require --trace-bot-run".into());
    }
    if list {
        return Ok(CliOptions::ListBots);
    }
    let resources = resources.ok_or("missing --resources")?;
    if trace {
        let [Some(p1), Some(p2)] = players else {
            return Err("expected --p1 and --p2".into());
        };
        let [Some(o1), Some(o2)] = outputs else {
            return Err("expected --o1 and --o2".into());
        };
        if o1 == o2 {
            return Err("trace outputs must be distinct".into());
        }
        return Ok(CliOptions::Trace {
            resources,
            players: [p1, p2],
            outputs: [o1, o2],
        });
    }
    if let Some(input) = single {
        return Ok(CliOptions::Single { resources, input });
    }
    Ok(CliOptions::Multi {
        resources,
        inputs: multi
            .try_into()
            .map_err(|_| "expected two multiplayer input files")?,
    })
}

pub fn parse_actions(bytes: &[u8]) -> Vec<Action> {
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
            _ => panic!("invalid action"),
        };
        actions.push(action);
    }
    actions
}
