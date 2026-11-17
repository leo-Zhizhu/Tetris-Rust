use crate::{
    dead_piece::DeadPiece,
    input::Action,
    piece::{ActivePiece, PieceKind, Turn},
    randomizer::BagRandomizer,
    resources::Resources,
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

    pub fn step(&mut self, action: Action) {
        if self.is_over() {
            return;
        }
        let piece = self.active.as_mut().expect("panic! no active piece");
        match action {
            Action::HardDrop => {
                piece.hard_drop(&self.dead);
                self.finish_lock();
                return;
            }
            Action::RotateClockwise => {
                piece.try_rotate(Turn::Clockwise, &self.dead);
            }
            Action::RotateCounterclockwise => {
                piece.try_rotate(Turn::Counterclockwise, &self.dead);
            }
            Action::Left => {
                piece.try_translate(-1, 0, &self.dead);
            }
            Action::Right => {
                piece.try_translate(1, 0, &self.dead);
            }
            Action::SoftDrop | Action::Wait => {}
        }

        piece.advance_clock();
        let gravity = self
            .resources
            .config
            .gravity_for_frame(action == Action::SoftDrop);
        if piece.frames_since_fall() >= gravity && piece.try_translate(0, -1, &self.dead) {
            piece.record_fall();
        }
        if self.lock_due() {
            self.finish_lock();
        }
    }

    fn lock_due(&self) -> bool {
        let piece = self.active.as_ref().expect("panic! no active piece");

        !piece.can_move_down(&self.dead)
            && piece.frames_since_fall() >= self.resources.config.lock_delay
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
