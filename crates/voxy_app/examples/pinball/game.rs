use physics::planar::{Ball, Capsule, Vec2, resolve_contact};

pub const STEP: f64 = 1.0 / 240.0;
pub const BALL_RADIUS: f64 = 0.22;
pub const FLIPPER_LENGTH: f64 = 2.3;
pub const FLIPPER_RADIUS: f64 = 0.19;
pub const PIVOTS: [Vec2; 2] = [Vec2::new(-2.8, 2.8), Vec2::new(2.8, 2.8)];
pub const BUMPERS: [Vec2; 3] = [
    Vec2::new(-1.7, 12.2),
    Vec2::new(1.4, 12.8),
    Vec2::new(-0.1, 15.0),
];
pub const BUMPER_RADIUS: f64 = 0.65;
pub const WALLS: [(Vec2, Vec2); 9] = [
    (Vec2::new(-5.0, 0.0), Vec2::new(-5.0, 16.0)),
    (Vec2::new(-5.0, 16.0), Vec2::new(-3.0, 18.0)),
    (Vec2::new(-3.0, 18.0), Vec2::new(3.0, 18.0)),
    (Vec2::new(3.0, 18.0), Vec2::new(5.0, 16.0)),
    (Vec2::new(5.0, 16.0), Vec2::new(5.0, 0.0)),
    (Vec2::new(3.9, 0.0), Vec2::new(3.9, 14.5)),
    (Vec2::new(-5.0, 5.0), Vec2::new(-2.8, 2.8)),
    (Vec2::new(3.9, 4.5), Vec2::new(2.8, 2.8)),
    (Vec2::new(-3.4, 8.5), Vec2::new(-2.4, 6.5)),
];

#[derive(Clone, Copy, Debug, Default)]
pub struct Input {
    pub left: bool,
    pub right: bool,
    pub launch: bool,
}
#[derive(Clone, Debug)]
pub struct Game {
    pub ball: Ball,
    pub angles: [f64; 2],
    pub score: u32,
    pub lives: u32,
    pub ready: bool,
    pub charge: f64,
    pub flashes: [f64; 3],
    pub impacts: u32,
    pub flipper_hits: u32,
    was_launch: bool,
}
impl Default for Game {
    fn default() -> Self {
        Self {
            ball: Ball {
                position: Vec2::new(4.46, 1.4),
                velocity: Vec2::default(),
                radius: BALL_RADIUS,
            },
            angles: [-0.42, std::f64::consts::PI + 0.42],
            score: 0,
            lives: 3,
            ready: true,
            charge: 0.0,
            flashes: [0.0; 3],
            impacts: 0,
            flipper_hits: 0,
            was_launch: false,
        }
    }
}
impl Game {
    pub fn flipper(&self, index: usize, angular_velocity: f64) -> Capsule {
        let pivot = PIVOTS[index];
        let angle = self.angles[index];
        let mut capsule = Capsule::fixed(
            pivot,
            pivot + Vec2::new(angle.cos(), angle.sin()) * FLIPPER_LENGTH,
            FLIPPER_RADIUS,
        );
        capsule.angular_velocity = angular_velocity;
        capsule
    }
    pub fn tick(&mut self, input: Input) {
        if self.lives == 0 {
            return;
        }
        if self.ready {
            if input.launch {
                self.charge = (self.charge + STEP).min(1.0);
            }
            if self.was_launch && !input.launch {
                self.ball.velocity = Vec2::new(0.0, 24.0 + self.charge * 12.0);
                self.ready = false;
                self.charge = 0.0;
            }
        }
        self.was_launch = input.launch;
        // Four substeps: <= 0.047 units of ball travel at the 45-unit/s speed cap.
        // A flipper tip travels <= 0.034 units. Both are well below the 0.22 ball radius.
        for _ in 0..4 {
            self.substep(input, STEP / 4.0);
        }
        if self.ball.position.y < 0.4 {
            self.lives -= 1;
            self.ready = true;
            self.charge = 0.0;
            self.ball.position = Vec2::new(4.46, 1.4);
            self.ball.velocity = Vec2::default();
        }
    }
    fn substep(&mut self, input: Input, dt: f64) {
        let targets = [
            if input.left { 0.5 } else { -0.42 },
            std::f64::consts::PI + if input.right { -0.5 } else { 0.42 },
        ];
        let mut speeds = [0.0; 2];
        for i in 0..2 {
            let delta = (targets[i] - self.angles[i]).clamp(-14.0 * dt, 14.0 * dt);
            speeds[i] = delta / dt;
            self.angles[i] += delta;
        }
        for flash in &mut self.flashes {
            *flash = (*flash - dt).max(0.0);
        }
        if self.ready {
            return;
        }
        self.ball.velocity.y -= 10.0 * dt;
        self.ball.velocity = self.ball.velocity * (1.0 - 0.035 * dt);
        self.cap_speed();
        self.ball.position = self.ball.position + self.ball.velocity * dt;
        for (a, b) in WALLS {
            resolve_contact(&mut self.ball, Capsule::fixed(a, b, 0.12), 0.82);
        }
        for (i, center) in BUMPERS.into_iter().enumerate() {
            let impact = resolve_contact(
                &mut self.ball,
                Capsule::fixed(center, center, BUMPER_RADIUS),
                0.95,
            );
            if impact > 0.1 && self.flashes[i] == 0.0 {
                let offset = self.ball.position - center;
                self.ball.velocity = self.ball.velocity + offset * (7.0 / offset.length());
                self.score = self.score.saturating_add(100);
                self.impacts += 1;
                self.flashes[i] = 0.15;
            }
        }
        for (i, speed) in speeds.into_iter().enumerate() {
            let flipper = self.flipper(i, speed);
            if resolve_contact(&mut self.ball, flipper, 0.75) > 0.1 {
                self.flipper_hits += 1;
            }
        }
        self.cap_speed();
    }
    fn cap_speed(&mut self) {
        let speed = self.ball.velocity.length();
        if speed > 45.0 {
            self.ball.velocity = self.ball.velocity * (45.0 / speed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn launch_release_and_three_drains_end_game() {
        let mut game = Game::default();
        for _ in 0..240 {
            game.tick(Input {
                launch: true,
                ..Input::default()
            });
        }
        assert!(game.ready);
        game.tick(Input::default());
        assert!(!game.ready);
        assert!(game.ball.velocity.y > 35.0);
        for lives in (0..3).rev() {
            game.ready = false;
            game.ball.position = Vec2::new(0.0, 0.0);
            game.tick(Input::default());
            assert_eq!(game.lives, lives);
        }
        let position = game.ball.position;
        game.tick(Input {
            launch: true,
            left: true,
            right: true,
        });
        assert_eq!(game.ball.position, position);
    }
    #[test]
    fn fast_ball_does_not_cross_wall() {
        let mut game = Game {
            ready: false,
            ..Game::default()
        };
        game.ball.position = Vec2::new(-4.0, 10.0);
        game.ball.velocity = Vec2::new(-45.0, 0.0);
        for _ in 0..15 {
            game.tick(Input::default());
            assert!(game.ball.position.x > -4.67);
        }
        assert!(game.ball.velocity.x > 0.0);
    }
    #[test]
    fn bumper_scores_and_kicks_outward() {
        let mut game = Game {
            ready: false,
            ..Game::default()
        };
        game.ball.position = BUMPERS[0] + Vec2::new(0.0, -0.86);
        game.ball.velocity = Vec2::new(0.0, 8.0);
        game.tick(Input::default());
        assert_eq!(game.score, 100);
        assert!(game.ball.velocity.y < -10.0);
    }
    #[test]
    fn scripted_play_remains_finite_and_hits_bumpers() {
        let mut game = Game::default();
        let mut hits = 0;
        for tick in 0..30_000 {
            if game.lives == 0 {
                hits += game.impacts;
                game = Game::default();
            }
            game.tick(Input {
                launch: game.ready && tick % 200 < 150,
                left: tick % 90 < 45,
                right: tick % 110 < 55,
            });
            assert!(game.ball.position.x.is_finite() && game.ball.position.y.is_finite());
            assert!(game.ball.velocity.length() <= 45.000_001);
            assert!(game.ball.position.x.abs() < 5.0);
            assert!(game.ball.position.y < 18.0);
        }
        assert!(hits + game.impacts > 0);
    }
}
