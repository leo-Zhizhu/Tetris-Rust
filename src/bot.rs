use std::path::Path;
use crate::{
    game::Game, 
    piece::{PieceKind, Turn},
    input::Action,
    dead_piece::DeadPiece,
    piece::{Point, Rotation, ActivePiece},
    resources::Resources,
};

pub enum TetrominoType {
    I, J, L, O, S, Z, T
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TetrisAction {
    C, W, L, R, D, H, E
}

impl From<TetrisAction> for Action {
    fn from(action: TetrisAction) -> Self {
        match action {
            TetrisAction::C => Action::RotateClockwise,
            TetrisAction::W => Action::RotateCounterclockwise,
            TetrisAction::L => Action::Left,
            TetrisAction::R => Action::Right,
            TetrisAction::D => Action::SoftDrop,
            TetrisAction::H => Action::HardDrop,
            TetrisAction::E => Action::Wait,
        }
    }
}

#[derive(Clone, Copy)]
pub enum CellState {
    Filled, Empty
}

pub struct BoardState {
    pub tetromino_row: usize,
    pub tetromino_col: usize,
    pub tetromino_rotation: usize,
    pub tetromino: TetrominoType,
    pub board: [[CellState; 10]; 20],
}

impl BoardState {
    pub fn from_game(game: &Game) -> Option<Self> {
        let piece = game.active()?;
        let origin = piece.origin();
        let locked = game.dead();
        let mut board = [[CellState::Empty; 10]; 20];
        for row in 0..20 {
            for col in 0..10 {
                if locked.occupied(col, row) {
                    board[row][col] = CellState::Filled;
                }
            }
        }
        let tetromino = match piece.kind() {
            PieceKind::I => TetrominoType::I,
            PieceKind::J => TetrominoType::J,
            PieceKind::L => TetrominoType::L,
            PieceKind::O => TetrominoType::O,
            PieceKind::S => TetrominoType::S,
            PieceKind::Z => TetrominoType::Z,
            PieceKind::T => TetrominoType::T,
        };
        Some(Self {
            tetromino_row: usize::try_from(origin.y).expect("needs a nonnegative row"),
            tetromino_col: usize::try_from(origin.x).expect("needs a nonnegetive column"),
            tetromino_rotation: piece.rotation() as usize,
            tetromino,
            board
        })
    }
    
    pub fn get_dead_piece(&self) -> DeadPiece {
        let mut grid = [[false; 10]; 20];
        for row in 0..20 {
            for col in 0..10 {
                grid[row][col] = matches!(self.board[row][col], CellState::Filled);
            }
        }
        DeadPiece::from_grid(&grid)
    }

    pub fn get_active_piece(&self, resources: &Resources) -> ActivePiece {
        let kind = match self.tetromino {
            TetrominoType::I => PieceKind::I,
            TetrominoType::J => PieceKind::J,
            TetrominoType::L => PieceKind::L,
            TetrominoType::O => PieceKind::O,
            TetrominoType::S => PieceKind::S,
            TetrominoType::Z => PieceKind::Z,
            TetrominoType::T => PieceKind::T,
        };
        let rotation = match self.tetromino_rotation {
            0 => Rotation::Spawn,
            1 => Rotation::Right,
            2 => Rotation::Half,
            3 => Rotation::Left,
            _ => panic!("needs an integer between 0 and 3"),
        };
        let origin = Point {
            x: i32::try_from(self.tetromino_col).expect("needs an i32"),
            y: i32::try_from(self.tetromino_row).expect("needs an i32"),
        };
        ActivePiece::from_pose(kind, resources.definition(kind).clone(), origin, rotation)
    }
}

pub trait SinglePlayerBot {
    /// Create a new bot.
    fn new() -> Self;

    /// Gets an action to perform based on the board state. This should be called
    /// every frame.
    fn get_move(&mut self, board_state: &BoardState) -> TetrisAction;
}

pub struct SimpleBot {
    resources: Resources,
}

impl SinglePlayerBot for SimpleBot {
    fn new() -> Self {
        Self {
            resources: Resources::load(Path::new("resources")),
        }
    }

    fn get_move(&mut self, board_state: &BoardState) -> TetrisAction {
        let board = board_state.get_dead_piece();
        let piece = board_state.get_active_piece(&self.resources);
        let candidates = get_candidates(&piece, &board);
        let mut best_key = (i32::MAX, usize::MAX);
        let mut best_actions = Vec::new();
        for candidate in candidates {
            let Some(score) = landing_score(&candidate.piece, &board) else {
                continue;
            };
            let key = (score, candidate.actions.len());
            if key < best_key {
                best_key = key;
                best_actions = candidate.actions;
            }
        }
        best_actions.first().copied().unwrap_or(TetrisAction::H)
    }
}

pub fn board_score(board: &[[CellState; 10]; 20], lines_cleared: usize) -> i32 {
    let mut heights = [0_i32; 10];
    let mut holes = 0;
    for col in 0..10 {
        let mut flag = false;
        for row in (0..20).rev() {
            match board[row][col] {
                CellState::Filled => {
                    if !flag {
                        heights[col] = (row + 1) as i32;
                        flag = true;
                    }
                }
                CellState::Empty => {
                    if flag {
                        holes += 1;
                    }
                }
            }
        }
    }
    let total_height: i32 = heights.iter().sum();
    let mut unevenness = 0;
    for col in 0..9 {
        unevenness += (heights[col] - heights[col + 1]).abs();
    }
    5 * total_height + 20 * holes + 3 * unevenness - 8 * lines_cleared as i32
}

pub fn landing_score(piece: &ActivePiece, board: &DeadPiece) -> Option<i32> {
    let mut trial_piece = piece.clone();
    let mut trial_board = board.clone();
    if !trial_board.can_place(&trial_piece.cells()) {
        return None;
    }
    trial_piece.hard_drop(&trial_board);
    let cells = trial_piece.cells();
    if cells.iter().any(|cell| cell.y >= trial_board.height() as i32) {
        return None;
    }
    trial_board.merge(&cells);
    let lines_cleared = trial_board.clear_full_rows();
    let mut result = [[CellState::Empty; 10]; 20];
    for row in 0..20 {
        for col in 0..10 {
            if trial_board.occupied(col, row) {
                result[row][col] = CellState::Filled;
            }
        }
    }
    Some(board_score(&result, lines_cleared))
}

pub struct Candidate {
    pub piece: ActivePiece,
    pub actions: Vec<TetrisAction>,
}

pub fn get_candidates(piece: &ActivePiece, board: &DeadPiece) -> Vec<Candidate> {
    let mut candidates = Vec::new();
    let mut rotated = piece.clone();
    let mut rotation_actions = Vec::new();
    for turns in 0..4 {
        if turns > 0 {
            if !rotated.try_rotate(Turn::Clockwise, board) {
                break;
            }
            rotation_actions.push(TetrisAction::C);
        }
        candidates.push(Candidate {
            piece: rotated.clone(),
            actions: rotation_actions.clone(),
        });
        for (dx, action) in [(-1, TetrisAction::L), (1, TetrisAction::R)] {
            let mut moved = rotated.clone();
            let mut actions = rotation_actions.clone();
            while moved.try_translate(dx, 0, board) {
                actions.push(action);
                candidates.push(Candidate {
                    piece: moved.clone(),
                    actions: actions.clone(),
                });
            }
        }
    }
    candidates
}


