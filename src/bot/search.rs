use std::collections::{HashMap, HashSet};

use super::{TetrisAction, locked_score};
use crate::{dead_piece::DeadPiece, game::step_piece, piece::ActivePiece, resources::GameConfig};

const MAX_PATH_FRAMES: usize = 300;
const MAX_SEARCH_STATES: usize = 20_000;

// This order also breaks ties between equally good, equally short paths.
pub(super) const ACTIONS: [TetrisAction; 7] = [
    TetrisAction::H,
    TetrisAction::W,
    TetrisAction::C,
    TetrisAction::L,
    TetrisAction::R,
    TetrisAction::D,
    TetrisAction::E,
];

pub(super) struct SearchContext {
    pub board: DeadPiece,
    pub piece: ActivePiece,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct SearchResult {
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

// Each worker owns these containers and reuses their capacity for later jobs.
// Nothing here is shared with another worker or protected by a mutex.
pub(super) struct SearchWorker {
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

        // Jobs differ only in their first action. Each worker generates all
        // further movements itself, including turns after moving or falling.
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

        // Appending to the vector and advancing this index gives us a simple
        // breadth-first search: all short paths are visited before longer ones.
        let mut next = 0;
        while next < self.nodes.len() {
            let piece = self.nodes[next].piece.clone();
            let frames = self.nodes[next].frames + 1;
            for action in ACTIONS {
                let mut moved = piece.clone();
                let locked = step_piece(&mut moved, &context.board, config, action.into());
                if locked {
                    // A lock ends this branch. Do not generate a successor
                    // piece or let later commands move the locked piece.
                    self.consider_landing(context, &moved, Some(next), action, frames, &mut best);
                } else if frames < MAX_PATH_FRAMES && self.nodes.len() < MAX_SEARCH_STATES {
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
        // The timer changes what happens next, even at an identical position.
        // The rotation counter is not used by the assignment's current rules.
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
        // Many paths lock in the same pose. Its score does not depend on the
        // timer, so merge and clear rows only once per landing in this job.
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

        // Follow parent indices backward, then reverse the path. This avoids
        // copying a long action vector for every state we explore.
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

// Results arrive in an unpredictable order. The controller stores them by
// job number first, so equal results always favor the same first action.
pub(super) fn choose_result(results: Vec<Option<SearchResult>>) -> Option<SearchResult> {
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
