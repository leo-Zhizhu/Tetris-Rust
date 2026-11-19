use crate::{
    dead_piece::DeadPiece,
    input::Action,
    piece::{ActivePiece, PieceKind, Turn},
    randomizer::BagRandomizer,
    resources::{GameConfig, Resources},
};

pub struct Game {
    resources: Resources,
    bag: BagRandomizer<PieceKind>,
    active: Option<ActivePiece>,
    dead: DeadPiece,
    game_over: bool,
    lines_cleared: usize,
}

impl Game {
    pub fn new(resources: Resources) -> Self {
        let bag =
            BagRandomizer::from_parts(PieceKind::ALL.into_iter(), resources.config.random_seed);
        let dead = DeadPiece::new(resources.config.width, resources.config.height);
        let mut game = Self {
            resources,
            bag,
            active: None,
            dead,
            game_over: false,
            lines_cleared: 0,
        };
        game.spawn_current();
        game
    }

    pub fn lines_cleared(&self) -> usize {
        self.lines_cleared
    }

    pub fn is_over(&self) -> bool {
        self.game_over
    }

    pub fn active(&self) -> Option<&ActivePiece> {
        self.active.as_ref()
    }

    pub fn dead(&self) -> &DeadPiece {
        &self.dead
    }

    /// Advances one frame and returns whether a piece locked this frame.
    /// A lock that tops out still returns true; later frames return false.
    pub fn step(&mut self, action: Action) -> bool {
        if self.is_over() {
            return false;
        }
        let piece = self.active.as_mut().expect("no active piece");
        let locked = step_piece(piece, &self.dead, &self.resources.config, action);
        if locked {
            self.finish_lock();
        }
        locked
    }

    pub(crate) fn add_garbage(&mut self, rows: usize, hole: usize) {
        if self.is_over() {
            return;
        }
        if self.dead.add_garbage(rows, hole) {
            self.game_over = true;
            self.active = None;
        }
    }

    fn finish_lock(&mut self) {
        let cells = self
            .active
            .take()
            .expect("panic! no active piece to lock")
            .cells();
        self.dead.merge(&cells);

        for cell in cells {
            if cell.y >= self.resources.config.height as i32 {
                self.game_over = true;
                return;
            }
        }

        self.lines_cleared += self.dead.clear_full_rows();
        self.bag.next();
        self.spawn_current();
    }

    fn spawn_current(&mut self) {
        let kind = *self.bag.cur();
        self.active = Some(ActivePiece::spawn(
            kind,
            self.resources.definition(kind).clone(),
            self.resources.config.width,
            self.resources.config.height,
        ));
    }
}

// Advance exactly one frame. Both the game and the bot's search use this
// function, so a simulated action has the same gravity and lock timing.
// Returning true asks the caller to merge this piece into its board.
pub(crate) fn step_piece(
    piece: &mut ActivePiece,
    board: &DeadPiece,
    config: &GameConfig,
    action: Action,
) -> bool {
    match action {
        Action::HardDrop => {
            piece.hard_drop(board);
            return true;
        }
        Action::RotateClockwise => {
            piece.try_rotate(Turn::Clockwise, board);
        }
        Action::RotateCounterclockwise => {
            piece.try_rotate(Turn::Counterclockwise, board);
        }
        Action::Left => {
            piece.try_translate(-1, 0, board);
        }
        Action::Right => {
            piece.try_translate(1, 0, board);
        }
        Action::SoftDrop | Action::Wait => {}
    }

    // An ignored movement still consumes a frame. Soft drop changes this
    // frame's gravity interval; it does not immediately move down one row.
    piece.advance_clock();
    let gravity = config.gravity_for_frame(action == Action::SoftDrop);
    if piece.frames_since_fall() >= gravity && piece.try_translate(0, -1, board) {
        piece.record_fall();
    }

    // Only a grounded piece can lock. Check after gravity, since a successful
    // fall resets the timer. Horizontal moves and rotations do not reset it.
    !piece.can_move_down(board) && piece.frames_since_fall() >= config.lock_delay
}
