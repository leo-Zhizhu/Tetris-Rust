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

pub fn step_piece(
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

    piece.advance_clock();
    let gravity = config.gravity_for_frame(action == Action::SoftDrop);
    if piece.frames_since_fall() >= gravity && piece.try_translate(0, -1, board) {
        piece.record_fall();
    }

    !piece.can_move_down(board) && piece.frames_since_fall() >= config.lock_delay
}
