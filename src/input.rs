use std::{error::Error, ffi::OsString, path::PathBuf};

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

pub fn parse_args(mut args: impl Iterator<Item = OsString>) -> Result<CliOptions, Box<dyn Error>> {
    let mut inputs_path = None;
    let mut resources_dir = None;

    while let Some(flag) = args.next() {
        let destination = match flag.to_str() {
            Some("--simulate-inputs-single-player") => &mut inputs_path,
            Some("--resources") => &mut resources_dir,
            _ => return Err("unknown argument".into()),
        };
        if destination.is_some() {
            return Err("argument repeated".into());
        }
        let value = args.next().ok_or("missing path")?;
        if value.to_string_lossy().starts_with("--") {
            return Err("missing path".into());
        }
        *destination = Some(PathBuf::from(value));
    }

    Ok(CliOptions {
        inputs_path: inputs_path.ok_or("missing input file")?,
        resources_dir: resources_dir.ok_or("missing resources directory")?,
    })
}

pub fn parse_actions(bytes: &[u8]) -> Result<Vec<Action>, Box<dyn Error>> {
    let mut actions = Vec::new();
    for (index, &byte) in bytes.iter().enumerate() {
        let action = match byte {
            b'C' => Action::RotateClockwise,
            b'W' => Action::RotateCounterclockwise,
            b'L' => Action::Left,
            b'R' => Action::Right,
            b'D' => Action::SoftDrop,
            b'H' => Action::HardDrop,
            b'E' => Action::Wait,
            byte if byte.is_ascii_whitespace() => continue,
            _ => {
                return Err(format!("invalid action at position {}", index + 1).into());
            }
        };
        actions.push(action);
    }
    Ok(actions)
}
