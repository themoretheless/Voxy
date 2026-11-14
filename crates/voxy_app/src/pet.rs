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

    /// Computes a circadian and weather comfort multiplier in `[0.25, 1.25]` affecting
    /// pet happiness/energy based on rain, snow, and hour of day (`0..24`).
    #[allow(dead_code)]
    pub fn weather_comfort_modifier(self, rain: f32, snow: f32, hour_of_day: f32) -> f32 {
        let r = if rain.is_finite() {
            rain.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let s = if snow.is_finite() {
            snow.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let h = if hour_of_day.is_finite() {
            hour_of_day.rem_euclid(24.0)
        } else {
            12.0
        };
        let is_night = !(6.5..=20.5).contains(&h);
        let circadian = if self.sleeping {
            if is_night { 1.15 } else { 0.95 }
        } else if is_night {
            0.80
        } else {
            1.10
        };
        let precipitation_penalty = r * 0.35 + s * 0.45;
        (circadian - precipitation_penalty).clamp(0.25, 1.25)
    }
}

/// Computes Craig Reynolds arrival-and-pursuit steering velocity `[vx, vy, vz]` toward a
/// moving target with lookahead time proportional to distance and linear ramp-down inside `slowing_radius`.
#[allow(dead_code)]
pub fn pet_pursuit_steering_velocity(
    pet_pos: [f32; 3],
    target_pos: [f32; 3],
    target_vel: [f32; 3],
    max_speed: f32,
    slowing_radius: f32,
) -> [f32; 3] {
    if !pet_pos.iter().all(|v| v.is_finite())
        || !target_pos.iter().all(|v| v.is_finite())
        || !target_vel.iter().all(|v| v.is_finite())
        || !max_speed.is_finite()
        || !slowing_radius.is_finite()
        || max_speed <= 0.0
    {
        return [0.0; 3];
    }
    let dx = target_pos[0] - pet_pos[0];
    let dy = target_pos[1] - pet_pos[1];
    let dz = target_pos[2] - pet_pos[2];
    let dist = (dx * dx + dy * dy + dz * dz).sqrt();
    if dist <= 1e-4 {
        return [0.0; 3];
    }
    let lookahead = (dist / max_speed.max(0.5)).clamp(0.0, 1.5);
    let future = [
        target_pos[0] + target_vel[0] * lookahead,
        target_pos[1] + target_vel[1] * lookahead,
        target_pos[2] + target_vel[2] * lookahead,
    ];
    let to_future = [
        future[0] - pet_pos[0],
        future[1] - pet_pos[1],
        future[2] - pet_pos[2],
    ];
    let future_dist =
        (to_future[0] * to_future[0] + to_future[1] * to_future[1] + to_future[2] * to_future[2])
            .sqrt();
    if future_dist <= 1e-4 {
        return [0.0; 3];
    }
    let ramp = if slowing_radius > 1e-4 {
        (future_dist / slowing_radius).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let speed = max_speed * ramp;
    let inv = speed / future_dist;
    [to_future[0] * inv, to_future[1] * inv, to_future[2] * inv]
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

    #[test]
    fn weather_comfort_modifier_prefers_clear_daytime_and_night_sleep() {
        let mut pet = PetState::default();
        let clear_day = pet.weather_comfort_modifier(0.0, 0.0, 14.0);
        let stormy_day = pet.weather_comfort_modifier(1.0, 0.5, 14.0);
        assert!(clear_day > 1.0);
        assert!(stormy_day < clear_day);
        pet.act(PetAction::Sleep);
        assert!(pet.weather_comfort_modifier(0.0, 0.0, 2.0) > 1.0);
    }

    #[test]
    fn pet_pursuit_steering_leads_moving_target_and_slows_on_arrival() {
        let far = pet_pursuit_steering_velocity(
            [0.0, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [0.0, 0.0, 2.0],
            4.0,
            2.0,
        );
        let near = pet_pursuit_steering_velocity(
            [0.0, 0.0, 0.0],
            [0.5, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            4.0,
            2.0,
        );
        let far_speed = (far[0] * far[0] + far[1] * far[1] + far[2] * far[2]).sqrt();
        let near_speed = (near[0] * near[0] + near[1] * near[1] + near[2] * near[2]).sqrt();
        assert!((far_speed - 4.0).abs() < 1e-4);
        assert!(far[2] > 0.0, "pursuit should lead target velocity in +Z");
        assert!(near_speed < far_speed);
    }
}
