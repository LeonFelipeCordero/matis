extern crate std;
use crate::brewing::recipe::*;
use std::collections::VecDeque;

pub struct Scheduling {
    waiting: VecDeque<Recipe>,
}

impl Scheduling {
    pub fn new() -> Self {
        Self { waiting: VecDeque::new() }
    }

    pub fn enqueue(&mut self, drink: Recipe) {
        self.waiting.push_back(drink);
    }

    pub fn next(&mut self) -> Option<Recipe> {
        self.waiting.pop_front()
    }
}
