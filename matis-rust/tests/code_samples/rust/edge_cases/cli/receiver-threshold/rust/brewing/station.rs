pub struct BrewingStation {
    water: i32,
    beans: i32,
    cups: i32,
}

impl BrewingStation {
    pub fn new(water: i32, beans: i32, cups: i32) -> Self {
        Self { water, beans, cups }
    }

    #[inline]
    pub fn ready(
        &self,
        water: i32, // Milliliters required by the recipe.
        beans: i32, // Ground coffee in grams.
        cups: i32, // Number of servings.
    ) -> bool {
        self.water >= water && self.beans >= beans && self.cups >= cups
    }

    #[inline]
    pub fn prepare(&mut self, water: i32, beans: i32, cups: i32) -> bool {
        if !self.ready(water, beans, cups) {
            return false;
        }
        self.water -= water;
        self.beans -= beans;
        self.cups -= cups;
        true
    }

    pub fn remaining(&self) -> i32 {
        self.cups
    }
}
