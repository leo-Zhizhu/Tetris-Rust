use std::collections::{HashMap, HashSet};

use super::{TetrisAction, locked_score};
use crate::{dead_piece::DeadPiece, game::step_piece, piece::ActivePiece, resources::GameConfig};

pub const ACTIONS: [TetrisAction; 7] = [
    TetrisAction::H,
    TetrisAction::W,
    TetrisAction::C,
    TetrisAction::L,
    TetrisAction::R,
    TetrisAction::D,
    TetrisAction::E,
];

pub struct SearchContext {
    pub board: DeadPiece,
    pub piece: ActivePiece,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SearchResult {
    pub score: i32,
    pub actions: Vec<TetrisAction>,
}

impl SearchResult {
    pub fn better_than(&self, other: &Self) -> bool {
        self.score < other.score
            || (self.score == other.score && self.actions.len() < other.actions.len())
    }
}

struct SearchNode {
    piece: ActivePiece,
    parent: Option<usize>,
    action: TetrisAction,
    frames: usize,
}

pub struct SearchWorker {
    nodes: Vec<SearchNode>,
    visited: HashSet<(i32, i32, usize, u32)>,
    scores: HashMap<(i32, i32, usize), Option<i32>>,
}

impl SearchWorker {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            visited: HashSet::new(),
            scores: HashMap::new(),
        }
    }

    pub fn search(
        &mut self,
        context: &SearchContext,
        config: &GameConfig,
        first_action: TetrisAction,
    ) -> Option<SearchResult> {
        self.nodes.clear();
        self.visited.clear();
        self.scores.clear();
        let mut best = None;

        let mut first_piece = context.piece.clone();
        if step_piece(
            &mut first_piece,
            &context.board,
            config,
            first_action.into(),
        ) {
            self.consider_landing(context, &first_piece, None, first_action, 1, &mut best);
            return best;
        }
        self.visited.insert(Self::state_key(&first_piece));
        self.nodes.push(SearchNode {
            piece: first_piece,
            parent: None,
            action: first_action,
            frames: 1,
        });

        let mut next = 0;
        while next < self.nodes.len() {
            let piece = self.nodes[next].piece.clone();
            let frames = self.nodes[next].frames + 1;
            for action in ACTIONS {
                let mut moved = piece.clone();
                let locked = step_piece(&mut moved, &context.board, config, action.into());
                if locked {
                    self.consider_landing(context, &moved, Some(next), action, frames, &mut best);
                } else if frames < 300 && self.nodes.len() < 20000 {
                    let key = Self::state_key(&moved);
                    if self.visited.insert(key) {
                        self.nodes.push(SearchNode {
                            piece: moved,
                            parent: Some(next),
                            action,
                            frames,
                        });
                    }
                }
            }
            next += 1;
        }
        best
    }

    fn state_key(piece: &ActivePiece) -> (i32, i32, usize, u32) {
        let origin = piece.origin();
        (
            origin.x,
            origin.y,
            piece.rotation() as usize,
            piece.frames_since_fall(),
        )
    }

    fn consider_landing(
        &mut self,
        context: &SearchContext,
        piece: &ActivePiece,
        parent: Option<usize>,
        action: TetrisAction,
        frames: usize,
        best: &mut Option<SearchResult>,
    ) {
        let origin = piece.origin();
        let key = (origin.x, origin.y, piece.rotation() as usize);
        let score = match self.scores.get(&key) {
            Some(score) => *score,
            None => {
                let score = locked_score(piece, &context.board);
                self.scores.insert(key, score);
                score
            }
        };
        let score = match score {
            Some(score) => score,
            None => return,
        };
        if let Some(previous) = best {
            if score > previous.score
                || (score == previous.score && frames >= previous.actions.len())
            {
                return;
            }
        }

        let mut actions = vec![action];
        let mut previous = parent;
        while let Some(index) = previous {
            actions.push(self.nodes[index].action);
            previous = self.nodes[index].parent;
        }
        actions.reverse();
        *best = Some(SearchResult { score, actions });
    }
}

pub fn choose_result(results: Vec<Option<SearchResult>>) -> Option<SearchResult> {
    let mut best: Option<SearchResult> = None;
    for result in results {
        if let Some(result) = result {
            if let Some(previous) = &best {
                if !result.better_than(previous) {
                    continue;
                }
            }
            best = Some(result);
        }
    }
    best
}
