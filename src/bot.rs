use std::{cell::RefCell, collections::VecDeque, path::Path, sync::Arc};

use crate::{
    dead_piece::DeadPiece, game::Game, input::Action, multiplayer::MultiplayerGame,
    piece::ActivePiece, resources::Resources,
};

mod search;
mod simulation;
#[cfg(test)]
mod tests;

use search::SearchContext;
use simulation::SimulationControl;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TetrisAction {
    C = b'C',
    W = b'W',
    L = b'L',
    R = b'R',
    D = b'D',
    H = b'H',
    E = b'E',
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

pub trait SinglePlayerBot {
    fn new(config: &Path) -> Self;

    fn get_move(&self) -> TetrisAction;

    fn update_with(&mut self, action: TetrisAction);
}

pub trait MultiPlayerBot {
    /// Create a new bot given the path to the resources dir.
    fn new(config: &Path) -> Self
    where
        Self: Sized;

    /// Gets an action to perform based on the board state. This should be called
    /// every frame.
    fn get_move(&self) -> TetrisAction;

    /// Updates the current bot state assuming the player plays a given action.
    /// `p_actions[i]` is the action taken by player `i`. This should be called
    /// every frame.
    fn update_with(&mut self, p_actions: &[TetrisAction]);
}

pub struct BadBot;

impl MultiPlayerBot for BadBot {
    fn new(_: &Path) -> Self {
        Self
    }
    fn get_move(&self) -> TetrisAction {
        TetrisAction::H
    }
    fn update_with(&mut self, _: &[TetrisAction]) {}
}

pub struct GoodBot {
    game: MultiplayerGame,
    player: usize,
}

impl GoodBot {
    pub fn for_player(config: &Path, player: usize) -> Self {
        Self {
            game: MultiplayerGame::new(std::array::from_fn(|_| Resources::load(config))),
            player,
        }
    }
}

impl MultiPlayerBot for GoodBot {
    fn new(config: &Path) -> Self {
        Self::for_player(config, 0)
    }

    fn get_move(&self) -> TetrisAction {
        let player = &self.game.games()[self.player];
        let Some(piece) = player.active() else {
            return TetrisAction::E;
        };
        let board = player.dead();
        let drop_score = |mut candidate: ActivePiece| {
            candidate.hard_drop(board);
            locked_score(&candidate, board).unwrap_or(i32::MAX)
        };

        // Keep a direct drop on ties. Move toward a strictly better column.
        let mut best = drop_score(piece.clone());
        let mut action = TetrisAction::H;
        for (dx, direction) in [(-1, TetrisAction::L), (1, TetrisAction::R)] {
            let mut candidate = piece.clone();
            while candidate.try_translate(dx, 0, board) {
                let score = drop_score(candidate.clone());
                if score < best {
                    best = score;
                    action = direction;
                }
            }
        }
        action
    }

    fn update_with(&mut self, p_actions: &[TetrisAction]) {
        let &[p1, p2] = p_actions else {
            panic!("expected two player actions");
        };
        self.game.step([p1.into(), p2.into()]);
    }
}

pub struct SimpleBot {
    game: Game,
    simulation: SimulationControl,
    actions: RefCell<VecDeque<TetrisAction>>,
}

impl SinglePlayerBot for SimpleBot {
    fn new(config: &Path) -> Self {
        let resources = Arc::new(Resources::load(config));
        Self {
            game: Game::new(Resources::load(config)),
            simulation: SimulationControl::new(resources),
            actions: RefCell::new(VecDeque::new()),
        }
    }

    fn get_move(&self) -> TetrisAction {
        let Some(piece) = self.game.active() else {
            return TetrisAction::E;
        };
        // Cache the chosen path, but do not consume a move or advance time
        // until the caller tells us which action was actually played.
        let mut actions = self.actions.borrow_mut();
        if actions.is_empty() {
            let context = Arc::new(SearchContext {
                board: self.game.dead().clone(),
                piece: piece.clone(),
            });
            *actions = match self.simulation.search(context) {
                Some(result) => result.actions.into(),
                None => VecDeque::from([TetrisAction::H]),
            };
        }
        actions[0]
    }

    fn update_with(&mut self, action: TetrisAction) {
        self.game.step(action.into());
        let actions = self.actions.get_mut();
        if actions.pop_front() != Some(action) || self.game.is_over() {
            // A different action invalidates the rest of the planned path.
            actions.clear();
        }
    }
}

pub fn board_score(board: &DeadPiece, lines_cleared: usize) -> i32 {
    let mut heights = vec![0_i32; board.width()];
    let mut holes = 0;
    for col in 0..board.width() {
        let mut found_block = false;
        for row in (0..board.height()).rev() {
            if board.occupied(col, row) {
                if !found_block {
                    heights[col] = (row + 1) as i32;
                    found_block = true;
                }
            } else if found_block {
                holes += 1;
            }
        }
    }
    let mut total_height = 0;
    for &height in &heights {
        total_height += height;
    }
    let mut unevenness = 0;
    for pair in heights.windows(2) {
        unevenness += (pair[0] - pair[1]).abs();
    }
    5 * total_height + 20 * holes + 3 * unevenness - 8 * lines_cleared as i32
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
    Some(board_score(&trial_board, lines_cleared))
}
