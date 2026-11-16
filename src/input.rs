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
pub struct CliOptions {
    pub inputs_path: PathBuf,
    pub resources_dir: PathBuf,
}

pub fn parse_args(mut args: impl Iterator<Item = OsString>) -> CliOptions {
    let mut inputs_path = None;
    let mut resources_dir = None;

    while let Some(flag) = args.next() {
        match flag.to_str().unwrap() {
            "--simulate-inputs-single-player" => {
                inputs_path = Some(PathBuf::from(args.next().unwrap()));
            }
            "--resources" => {
                resources_dir = Some(PathBuf::from(args.next().unwrap()));
            }
            _ => panic!("unknown argument"),
        }
    }

    CliOptions {
        inputs_path: inputs_path.unwrap(),
        resources_dir: resources_dir.unwrap(),
    }
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
