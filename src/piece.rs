use crate::dead_piece::DeadPiece;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PieceKind {
    I,
    J,
    L,
    O,
    S,
    T,
    Z,
}

impl PieceKind {
    pub const ALL: [Self; 7] = [
        Self::I,
        Self::J,
        Self::L,
        Self::O,
        Self::S,
        Self::T,
        Self::Z,
    ];

    pub fn name(self) -> &'static str {
        ["i", "j", "l", "o", "s", "t", "z"][self as usize]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    Spawn,
    Right,
    Half,
    Left,
}

impl Rotation {
    fn turned(self, turn: Turn) -> Self {
        let increment = match turn {
            Turn::Clockwise => 1,
            Turn::Counterclockwise => 3,
        };
        [Self::Spawn, Self::Right, Self::Half, Self::Left][(self as usize + increment) % 4]
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Turn {
    Clockwise,
    Counterclockwise,
}

#[derive(Clone)]
pub struct PieceDefinition {
    pub states: [[Point; 4]; 4],
    pub kick_offsets: [Vec<Point>; 4],
}

pub struct ActivePiece {
    kind: PieceKind,
    definition: PieceDefinition,
    origin: Point,
    rotation: Rotation,
    frames_since_fall: u32,
    rotations_since_descent: u32,
}

impl ActivePiece {
    pub fn spawn(
        kind: PieceKind,
        definition: PieceDefinition,
        width: usize,
        height: usize,
    ) -> Self {
        Self {
            kind,
            definition,
            origin: Point {
                x: ((width - 1) / 2) as i32,
                y: height as i32,
            },
            rotation: Rotation::Spawn,
            frames_since_fall: 0,
            rotations_since_descent: 0,
        }
    }

    pub fn kind(&self) -> PieceKind {
        self.kind
    }

    pub fn cells(&self) -> [Point; 4] {
        self.cells_at(self.origin, self.rotation)
    }

    fn cells_at(&self, origin: Point, rotation: Rotation) -> [Point; 4] {
        self.definition.states[rotation as usize].map(|offset| Point {
            x: origin.x + offset.x,
            y: origin.y + offset.y,
        })
    }

    pub fn can_move_down(&self, board: &DeadPiece) -> bool {
        let origin = Point {
            x: self.origin.x,
            y: self.origin.y - 1,
        };
        board.can_place(&self.cells_at(origin, self.rotation))
    }

    pub fn try_translate(&mut self, dx: i32, dy: i32, board: &DeadPiece) -> bool {
        let origin = Point {
            x: self.origin.x + dx,
            y: self.origin.y + dy,
        };
        if !board.can_place(&self.cells_at(origin, self.rotation)) {
            return false;
        }
        self.origin = origin;
        true
    }

    pub fn try_rotate(&mut self, turn: Turn, board: &DeadPiece) -> bool {
        let next = self.rotation.turned(turn);
        let from = &self.definition.kick_offsets[self.rotation as usize];
        let to = &self.definition.kick_offsets[next as usize];

        for (old_offset, new_offset) in from.iter().zip(to) {
            let origin = Point {
                x: self.origin.x + old_offset.x - new_offset.x,
                y: self.origin.y + old_offset.y - new_offset.y,
            };
            if board.can_place(&self.cells_at(origin, next)) {
                self.origin = origin;
                self.rotation = next;
                self.rotations_since_descent += 1;
                return true;
            }
        }
        false
    }

    pub fn hard_drop(&mut self, board: &DeadPiece) -> u32 {
        let mut distance = 0;
        while self.try_translate(0, -1, board) {
            distance += 1;
        }
        distance
    }

    pub fn advance_clock(&mut self) {
        self.frames_since_fall += 1;
    }

    pub fn frames_since_fall(&self) -> u32 {
        self.frames_since_fall
    }

    pub fn rotations_since_descent(&self) -> u32 {
        self.rotations_since_descent
    }

    pub fn record_fall(&mut self) {
        self.frames_since_fall = 0;
        self.rotations_since_descent = 0;
    }
}
