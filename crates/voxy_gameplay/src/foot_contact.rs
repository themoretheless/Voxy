//! Per-foot contact authority. Update returns a candidate; publication is caller-owned.
use super::{
    PhysicsError, SupportAnchor, SupportContact, SupportProbe, SupportQueryBudget, SupportWorld,
};
use glam::DVec3;
use serde::{Deserialize, Serialize};

/// Distances are world units. Acquisition is narrower than release (hysteresis).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FootContactSettings {
    pub probe_lift: f64,
    pub probe_drop: f64,
    pub plant_distance: f64,
    pub release_distance: f64,
    pub min_up_dot: f64,
}
impl Default for FootContactSettings {
    fn default() -> Self {
        Self {
            probe_lift: 0.2,
            probe_drop: 0.3,
            plant_distance: 0.08,
            release_distance: 0.35,
            min_up_dot: 0.7,
        }
    }
}
impl FootContactSettings {
    /// Validates authored support distances and slope admission without queries.
    pub fn validate(&self) -> Result<(), PhysicsError> {
        let distances = [
            self.probe_lift,
            self.probe_drop,
            self.plant_distance,
            self.release_distance,
        ];
        if distances
            .iter()
            .any(|v| !v.is_finite() || !(0. ..=1e6).contains(v))
            || self.probe_lift + self.probe_drop > 1e6
            || self.release_distance < self.plant_distance
            || !self.min_up_dot.is_finite()
            || !(0. ..=1.).contains(&self.min_up_dot)
        {
            return Err(PhysicsError::InvalidMotion);
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug)]
pub struct FootContactInput {
    /// Animated sole before IK, in the physically accepted actor world frame.
    pub sole: DVec3,
    pub up: DVec3,
    pub grounded: bool,
    /// Authored stance/contact intent. False begins a new swing and rearms planting.
    pub plant: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FootContactStatus {
    Swing,
    Airborne,
    Searching,
    Planted,
    Released,
    /// Failed support/reach cannot replant until swing or a new airborne landing.
    AwaitingSwing,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FootContactState {
    anchor: Option<SupportAnchor>,
    awaiting_swing: bool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FootContactCandidate {
    pub state: FootContactState,
    pub contact: Option<SupportContact>,
    pub status: FootContactStatus,
}
impl FootContactState {
    /// Releases an unreachable IK target; rearm on swing or airborne landing.
    #[must_use]
    pub fn release_until_swing(&self) -> Self {
        Self { anchor: None, awaiting_swing: true }
    }

    #[must_use]
    pub fn anchor(&self) -> Option<SupportAnchor> {
        self.anchor
    }
    /// # Errors
    /// Invalid settings/input, foreign supports or budget exhaustion preserve self.
    /// Query budget is consumed during preparation and is not restored on failure.
    pub fn prepare(
        &self,
        world: &SupportWorld,
        settings: FootContactSettings,
        input: FootContactInput,
        budget: &mut SupportQueryBudget,
    ) -> Result<FootContactCandidate, PhysicsError> {
        settings.validate()?;
        if !input.sole.is_finite()
            || input.sole.abs().max_element() > 1e6
            || !input.up.is_finite()
            || (input.up.length_squared() - 1.).abs() > 1e-12
        {
            return Err(PhysicsError::InvalidMotion);
        }
        let empty = |status, awaiting_swing| FootContactCandidate {
            state: Self {
                anchor: None,
                awaiting_swing,
            },
            contact: None,
            status,
        };
        if !input.grounded {
            return Ok(empty(FootContactStatus::Airborne, false));
        }
        if !input.plant {
            return Ok(empty(FootContactStatus::Swing, false));
        }
        if self.awaiting_swing {
            return Ok(empty(FootContactStatus::AwaitingSwing, true));
        }
        if let Some(anchor) = self.anchor {
            let Some(contact) = world.resolve(anchor, budget)? else {
                return Ok(empty(FootContactStatus::Released, true));
            };
            if contact.normal.dot(input.up) < settings.min_up_dot
                || contact.position.distance(input.sole) > settings.release_distance
            {
                return Ok(empty(FootContactStatus::Released, true));
            }
            return Ok(FootContactCandidate {
                state: *self,
                contact: Some(contact),
                status: FootContactStatus::Planted,
            });
        }
        let contact = world
            .probe(
                SupportProbe {
                    origin: input.sole + input.up * settings.probe_lift,
                    direction: -input.up,
                    up: input.up,
                    max_distance: settings.probe_lift + settings.probe_drop,
                    min_up_dot: settings.min_up_dot,
                },
                budget,
            )?
            .filter(|contact| contact.position.distance(input.sole) <= settings.plant_distance);
        Ok(FootContactCandidate {
            state: Self {
                anchor: contact.map(|contact| contact.anchor),
                awaiting_swing: false,
            },
            status: if contact.is_some() {
                FootContactStatus::Planted
            } else {
                FootContactStatus::Searching
            },
            contact,
        })
    }
}
#[cfg(test)]
mod tests;
