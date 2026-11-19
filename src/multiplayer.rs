use std::collections::VecDeque;

use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro128PlusPlus;

use crate::{game::Game, input::Action, resources::Resources};

#[cfg(test)]
mod tests;

struct Garbage {
    rows: usize,
    timer: u32,
}

pub struct MultiplayerGame {
    games: [Game; 2],
    queues: [VecDeque<Garbage>; 2],
    rngs: [Xoshiro128PlusPlus; 2],
    garbage_queue_time: u32,
}

impl MultiplayerGame {
    pub fn new(resources: [Resources; 2]) -> Self {
        let garbage_queue_time = resources[0].config.garbage_queue_time;
        let rngs =
            std::array::from_fn(|i| Xoshiro128PlusPlus::from_seed(resources[i].config.random_seed));
        Self {
            games: resources.map(Game::new),
            queues: std::array::from_fn(|_| VecDeque::new()),
            rngs,
            garbage_queue_time,
        }
    }

    pub fn games(&self) -> &[Game; 2] {
        &self.games
    }

    pub fn step(&mut self, actions: [Action; 2]) {
        let mut locked = [false; 2];
        let mut outgoing = [0; 2];
        for i in 0..2 {
            let before = self.games[i].lines_cleared();
            locked[i] = self.games[i].step(actions[i]);
            let cleared = self.games[i].lines_cleared() - before;
            outgoing[i] = cleared.saturating_sub(1);
        }

        // Cancel oldest incoming rows for both players before enqueueing
        // anything new. Attacks sent this frame cannot cancel each other.
        for i in 0..2 {
            while outgoing[i] > 0 {
                let Some(batch) = self.queues[i].front_mut() else {
                    break;
                };
                let cancelled = outgoing[i].min(batch.rows);
                outgoing[i] -= cancelled;
                batch.rows -= cancelled;
                if batch.rows == 0 {
                    self.queues[i].pop_front();
                }
            }
        }
        for i in 0..2 {
            if outgoing[i] > 0 {
                self.queues[1 - i].push_back(Garbage {
                    rows: outgoing[i],
                    timer: self.garbage_queue_time,
                });
            }
        }

        for i in 0..2 {
            if locked[i] {
                while !self.games[i].is_over()
                    && self.queues[i].front().is_some_and(|batch| batch.timer == 0)
                {
                    let batch = self.queues[i].pop_front().unwrap();
                    // Draw only when applying a batch, once for all its rows.
                    let hole = self.rngs[i].next_u32() as usize % self.games[i].dead().width();
                    self.games[i].add_garbage(batch.rows, hole);
                }
            }
            for batch in &mut self.queues[i] {
                batch.timer = batch.timer.saturating_sub(1);
            }
        }
    }
}
