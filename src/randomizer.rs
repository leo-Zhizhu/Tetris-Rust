use rand::{SeedableRng, seq::SliceRandom};
use rand_xoshiro::Xoshiro128PlusPlus;

pub struct BagRandomizer<T> {
    bag: Vec<T>,
    idx: usize,
    rng: Xoshiro128PlusPlus,
}

impl<T> BagRandomizer<T> {
    pub fn from_parts(items: impl Iterator<Item = T>, seed: [u8; 16]) -> Self {
        let mut rng = Xoshiro128PlusPlus::from_seed(seed);
        let mut bag: Vec<T> = items.collect();
        bag.shuffle(&mut rng);
        Self { bag, idx: 0, rng }
    }

    pub fn cur(&self) -> &T {
        &self.bag[self.idx]
    }

    pub fn next(&mut self) {
        self.idx += 1;
        if self.idx == self.bag.len() {
            self.bag.shuffle(&mut self.rng);
            self.idx = 0;
        }
    }
}
