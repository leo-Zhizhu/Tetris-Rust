use std::{path::Path, sync::Arc};

use crate::{
    dead_piece::DeadPiece,
    game::{Game, step_piece},
    input::Action,
    piece::{ActivePiece, PieceKind, Point, Rotation},
    resources::Resources,
};

mod search;
mod simulation;
#[cfg(test)]
mod tests;

use search::SearchContext;
use simulation::SimulationControl;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TetrominoType {
    I,
    J,
    L,
    O,
    S,
    Z,
    T,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TetrisAction {
    C,
    W,
    L,
    R,
    D,
    H,
    E,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellState {
    Filled,
    Empty,
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
            tetromino_row: usize::try_from(origin.y).expect("invalid row"),
            tetromino_col: usize::try_from(origin.x).expect("invalid column"),
            tetromino_rotation: piece.rotation() as usize,
            tetromino,
            board,
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
            _ => panic!("invalid rotation"),
        };
        let origin = Point {
            x: i32::try_from(self.tetromino_col).expect("invalid position"),
            y: i32::try_from(self.tetromino_row).expect("invalid position"),
        };
        ActivePiece::from_pose(kind, resources.definition(kind).clone(), origin, rotation)
    }
}

pub trait SinglePlayerBot {
    fn new() -> Self;

    fn get_move(&mut self, board_state: &BoardState) -> TetrisAction;
}

pub struct SimpleBot {
    resources: Arc<Resources>,
    simulation: SimulationControl,
    predicted_piece: Option<ActivePiece>,
    expected_board: [[CellState; 10]; 20],
    actions: Vec<TetrisAction>,
    next_action: usize,
}

impl SinglePlayerBot for SimpleBot {
    fn new() -> Self {
        // Load once. Arc allows the bot and its four workers to read the same
        // resources without making separate copies or reading files again.
        let resources = Arc::new(Resources::load(Path::new("resources")));
        let simulation = SimulationControl::new(Arc::clone(&resources));
        Self {
            resources,
            simulation,
            predicted_piece: None,
            expected_board: [[CellState::Empty; 10]; 20],
            actions: Vec::new(),
            next_action: 0,
        }
    }

    fn get_move(&mut self, board_state: &BoardState) -> TetrisAction {
        let observed_piece = board_state.get_active_piece(&self.resources);

        // BoardState omits the timer. Keep the piece we predicted on the last
        // call instead of rebuilding it with a zero timer on every frame.
        if let Some(predicted) = &self.predicted_piece {
            if predicted.kind() != observed_piece.kind()
                || predicted.origin() != observed_piece.origin()
                || predicted.rotation() != observed_piece.rotation()
                || self.expected_board != board_state.board
            {
                // We cannot recover an unknown timer from a snapshot. Finish
                // this piece, then start tracking again from the next spawn.
                self.predicted_piece = None;
                self.actions.clear();
                self.next_action = 0;
                return TetrisAction::H;
            }
        } else {
            let spawn = Point {
                x: ((self.resources.config.width - 1) / 2) as i32,
                y: self.resources.config.height as i32,
            };
            if observed_piece.origin() != spawn || observed_piece.rotation() != Rotation::Spawn {
                return TetrisAction::H;
            }
            self.predicted_piece = Some(observed_piece);
        }

        let board = board_state.get_dead_piece();
        if self.next_action == self.actions.len() {
            // Only the snapshot is prepared here. Workers generate paths and
            // simulate them. Keep the chosen path until this piece locks.
            let context = Arc::new(SearchContext {
                board: board.clone(),
                piece: self.predicted_piece.as_ref().unwrap().clone(),
            });
            self.actions = match self.simulation.search(context) {
                Some(result) => result.actions,
                None => vec![TetrisAction::H],
            };
            self.next_action = 0;
        }

        let action = self.actions[self.next_action];
        self.next_action += 1;
        let locked = step_piece(
            self.predicted_piece.as_mut().unwrap(),
            &board,
            &self.resources.config,
            action.into(),
        );
        self.expected_board = board_state.board;
        if locked {
            // The next call describes a new piece, even if its kind happens
            // to match this one. Its timer must start at zero.
            self.predicted_piece = None;
            self.actions.clear();
            self.next_action = 0;
        }
        action
    }
}

pub fn board_score(board: &[[CellState; 10]; 20], lines_cleared: usize) -> i32 {
    let mut heights = [0_i32; 10];
    let mut holes = 0;
    for col in 0..10 {
        let mut found_block = false;
        for row in (0..20).rev() {
            match board[row][col] {
                CellState::Filled => {
                    if !found_block {
                        heights[col] = (row + 1) as i32;
                        found_block = true;
                    }
                }
                CellState::Empty => {
                    if found_block {
                        holes += 1;
                    }
                }
            }
        }
    }
    let mut total_height = 0;
    for height in heights {
        total_height += height;
    }
    let mut unevenness = 0;
    for col in 0..9 {
        unevenness += (heights[col] - heights[col + 1]).abs();
    }
    5 * total_height + 20 * holes + 3 * unevenness - 8 * lines_cleared as i32
}

// Convenience helper for a direct hard drop. The path search scores pieces
// at the actual frame where they lock, including locks caused by gravity.
pub fn landing_score(piece: &ActivePiece, board: &DeadPiece) -> Option<i32> {
    let mut dropped = piece.clone();
    if !board.can_place(&dropped.cells()) {
        return None;
    }
    dropped.hard_drop(board);
    locked_score(&dropped, board)
}

fn locked_score(piece: &ActivePiece, board: &DeadPiece) -> Option<i32> {
    let cells = piece.cells();
    if !board.can_place(&cells) {
        return None;
    }
    // The assignment checks top-out before clearing any completed rows.
    for cell in cells {
        if cell.y >= board.height() as i32 {
            return None;
        }
    }
    let mut trial_board = board.clone();
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
