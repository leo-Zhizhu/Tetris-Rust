use std::{collections::HashMap, error::Error, fs, path::Path};

use serde::{Deserialize, de::DeserializeOwned};

use crate::piece::{PieceDefinition, PieceKind, Point};

#[derive(Debug, Deserialize)]
pub struct GameConfig {
    pub width: usize,
    pub height: usize,
    pub gravity: u32,
    pub gravity_decrease: u32,
    pub lock_delay: u32,
    pub random_seed: [u8; 16],
}

impl GameConfig {
    pub fn gravity_for_frame(&self, soft_drop: bool) -> u32 {
        if !soft_drop {
            return self.gravity;
        }

        if self.gravity_decrease >= self.gravity {
            return 0;
        }

        self.gravity - self.gravity_decrease
    }
}

pub struct Resources {
    pub config: GameConfig,
    pieces: Vec<PieceDefinition>,
}

#[derive(Deserialize)]
struct RawPiece {
    states: [[[i32; 2]; 4]; 4],
    wall_kick: String,
}

#[derive(Clone, Deserialize)]
struct RawKicks {
    o: Vec<[i32; 2]>,
    r: Vec<[i32; 2]>,
    t: Vec<[i32; 2]>,
    l: Vec<[i32; 2]>,
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, Box<dyn Error>> {
    let text = fs::read_to_string(path).map_err(|_| "cannot read resource file")?;
    serde_json::from_str(&text).map_err(|_| "invalid resource JSON".into())
}

impl Resources {
    pub fn load(dir: &Path) -> Result<Self, Box<dyn Error>> {
        let config: GameConfig = read_json(&dir.join("config.json"))?;
        if config.width == 0 || config.height == 0 || config.gravity == 0 || config.lock_delay == 0
        {
            return Err("invalid config".into());
        }
        let shapes: HashMap<String, RawPiece> = read_json(&dir.join("tetrominoes.json"))?;
        let kicks: HashMap<String, RawKicks> = read_json(&dir.join("wall_kicks.json"))?;
        let mut pieces = Vec::new();

        for kind in PieceKind::ALL {
            let raw = shapes.get(kind.name()).ok_or("missing piece")?;
            let offsets = kicks
                .get(&raw.wall_kick)
                .ok_or("missing wall kicks")?
                .clone();
            let kick_offsets = [offsets.o, offsets.r, offsets.t, offsets.l].map(|list| {
                list.into_iter()
                    .map(|[x, y]| Point { x, y })
                    .collect::<Vec<_>>()
            });
            let count = kick_offsets[0].len();
            if count == 0 || kick_offsets.iter().any(|list| list.len() != count) {
                return Err("invalid wall kicks".into());
            }
            let states = raw.states.map(|state| state.map(|[x, y]| Point { x, y }));
            for state in &states {
                for index in 0..state.len() {
                    if state[..index].contains(&state[index]) {
                        return Err("repeated cell in piece".into());
                    }
                }
            }
            pieces.push(PieceDefinition {
                states,
                kick_offsets,
            });
        }

        Ok(Self { config, pieces })
    }

    pub fn definition(&self, kind: PieceKind) -> &PieceDefinition {
        &self.pieces[kind as usize]
    }
}
