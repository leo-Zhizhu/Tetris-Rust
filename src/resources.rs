use std::{collections::HashMap, fs, path::Path};

use serde::Deserialize;

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

#[derive(Deserialize)]
struct RawKicks {
    o: Vec<[i32; 2]>,
    r: Vec<[i32; 2]>,
    t: Vec<[i32; 2]>,
    l: Vec<[i32; 2]>,
}

impl Resources {
    pub fn load(dir: &Path) -> Self {
        let config_text = fs::read_to_string(dir.join("config.json")).unwrap();
        let config: GameConfig = serde_json::from_str(&config_text).unwrap();

        let shapes_text = fs::read_to_string(dir.join("tetrominoes.json")).unwrap();
        let shapes: HashMap<String, RawPiece> = serde_json::from_str(&shapes_text).unwrap();

        let kicks_text = fs::read_to_string(dir.join("wall_kicks.json")).unwrap();
        let kicks: HashMap<String, RawKicks> = serde_json::from_str(&kicks_text).unwrap();
        let mut pieces = Vec::new();

        for kind in PieceKind::ALL {
            let raw = &shapes[kind.name()];
            let offsets = &kicks[&raw.wall_kick];
            let lists = [&offsets.o, &offsets.r, &offsets.t, &offsets.l];
            let mut kick_offsets = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
            for rotation in 0..4 {
                for offset in lists[rotation] {
                    kick_offsets[rotation].push(Point {
                        x: offset[0],
                        y: offset[1],
                    });
                }
            }

            let mut states = [[Point { x: 0, y: 0 }; 4]; 4];
            for rotation in 0..4 {
                for cell in 0..4 {
                    states[rotation][cell] = Point {
                        x: raw.states[rotation][cell][0],
                        y: raw.states[rotation][cell][1],
                    };
                }
            }
            pieces.push(PieceDefinition {
                states,
                kick_offsets,
            });
        }

        Self { config, pieces }
    }

    pub fn definition(&self, kind: PieceKind) -> &PieceDefinition {
        &self.pieces[kind as usize]
    }
}
