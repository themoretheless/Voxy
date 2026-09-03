#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PetAction {
    Feed,
    Play,
    Sleep,
    Heal,
    Clean,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PetState {
    hunger: f32,
    happiness: f32,
    energy: f32,
    health: f32,
    hygiene: f32,
    age_seconds: f32,
    sleeping: bool,
}

impl Default for PetState {
    fn default() -> Self {
        Self {
            hunger: 82.0,
            happiness: 78.0,
            energy: 88.0,
            health: 100.0,
            hygiene: 92.0,
            age_seconds: 0.0,
            sleeping: false,
        }
    }
}

impl PetState {
    pub fn tick(&mut self, dt: f32, moving: bool) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.age_seconds += dt;
        self.hunger = (self.hunger - dt * 0.42).max(0.0);
        self.hygiene = (self.hygiene - dt * 0.14).max(0.0);
        if self.sleeping {
            self.energy = (self.energy + dt * 7.0).min(100.0);
            if self.energy >= 99.9 {
                self.sleeping = false;
            }
        } else {
            let activity = if moving { 1.15 } else { 0.34 };
            self.energy = (self.energy - dt * activity).max(0.0);
        }
        self.happiness = (self.happiness - dt * 0.2).max(0.0);
        if self.hunger < 18.0 || self.hygiene < 15.0 {
            self.health = (self.health - dt * 1.2).max(0.0);
        }
    }

    pub fn act(&mut self, action: PetAction) {
        match action {
            PetAction::Feed => {
                self.hunger = (self.hunger + 28.0).min(100.0);
                self.happiness = (self.happiness + 3.0).min(100.0);
            }
            PetAction::Play => {
                if !self.sleeping && self.energy >= 8.0 {
                    self.happiness = (self.happiness + 24.0).min(100.0);
                    self.energy = (self.energy - 8.0).max(0.0);
                    self.hygiene = (self.hygiene - 3.0).max(0.0);
                }
            }
            PetAction::Sleep => self.sleeping = !self.sleeping,
            PetAction::Heal => {
                if self.hunger >= 20.0 {
                    self.health = (self.health + 30.0).min(100.0);
                }
            }
            PetAction::Clean => self.hygiene = 100.0,
        }
    }

    pub const fn can_move(self) -> bool {
        !self.sleeping && self.energy > 0.0 && self.health > 0.0
    }

    fn mood(self) -> &'static str {
        if self.health <= 0.0 {
            "SICK"
        } else if self.sleeping {
            "SLEEP"
        } else if self.hunger < 20.0 {
            "HUNGRY"
        } else if self.hygiene < 20.0 {
            "DIRTY"
        } else if self.happiness > 75.0 {
            "HAPPY"
        } else {
            "OK"
        }
    }

    pub fn title(self) -> String {
        format!(
            "Voxy Tamagotchi [{}]  FOOD {:.0}  FUN {:.0}  ENERGY {:.0}  HEALTH {:.0}  CLEAN {:.0}",
            self.mood(),
            self.hunger,
            self.happiness,
            self.energy,
            self.health,
            self.hygiene,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn care_actions_are_bounded_and_sleep_recovers_energy() {
        let mut pet = PetState::default();
        pet.act(PetAction::Play);
        pet.act(PetAction::Sleep);
        assert!(!pet.can_move());
        pet.tick(10.0, false);
        assert!(pet.energy <= 100.0);
        pet.act(PetAction::Feed);
        pet.act(PetAction::Clean);
        assert!(pet.hunger <= 100.0);
        assert!((pet.hygiene - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn neglected_pet_becomes_hungry_and_unhealthy() {
        let mut pet = PetState::default();
        pet.tick(240.0, false);
        assert!(pet.hunger.abs() < f32::EPSILON);
        assert!(pet.health < 100.0);
        assert!(pet.title().contains("SICK"));
    }
}
