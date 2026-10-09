//! Read-only physical contact geometry for accelerator divergence diagnostics.
use super::math::{V, add, mul};
use super::{ContactSource, HairRod};
#[derive(Clone, Debug, PartialEq)]
pub struct HairContactDiagnostic {
    pub source: ContactSource,
    pub segment: usize,
    pub fraction: f64,
    /// Unit direction of the linearized contact force.
    pub normal: V,
    /// Linearized plane target; medial targets compensate metric_scale.
    pub target: V,
    /// Effective surface velocity of this contact linearization.
    pub surface_velocity: V,
    /// Physical residual per unit-normal plane residual.
    pub metric_scale:f64,
    /// Normalized time of a past trajectory constraint; None is endpoint geometry.
    pub trajectory_time:Option<f64>,
    /// Current physical signed contact residual, in metres.
    pub gap_m: f64,
}
impl HairRod {
    pub fn contact_diagnostics(&self) -> Vec<HairContactDiagnostic> {
        self.contacts
            .iter()
            .map(|contact| {
                let position = add(
                    mul(self.x[contact.segment], 1. - contact.fraction),
                    mul(self.x[contact.segment + 1], contact.fraction),
                );
                HairContactDiagnostic {
                    source: contact.source,
                    segment: contact.segment,
                    fraction: contact.fraction,
                    normal: contact.normal,
                    target: contact.target,
                    surface_velocity: contact.surface_velocity,
                    metric_scale:contact.metric_scale,
                    trajectory_time:contact.trajectory_time,
                    gap_m: contact.physical_gap(position),
                }
            })
            .collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagnostics_report_current_geometry_without_mutating_contacts() {
        let mut rod = HairRod::new(
            vec![[0., 0., 0.], [0., 0.01, 0.], [0., 0.02, 0.]],
            Default::default(),
        )
        .unwrap();
        rod.record_point_contact(1, [1., 0., 0.], [0., 0.01, 0.], ContactSource::Mesh(7));
        rod.contacts[0].surface_velocity = [0.25, 0., 0.];
        rod.x[1][0] = 0.001;
        let before = format!("{rod:?}");
        let diagnostics = rod.contact_diagnostics();
        assert_eq!(before, format!("{rod:?}"));
        assert_eq!(diagnostics.len(), 1);
        let contact = &diagnostics[0];
        assert_eq!(contact.source, ContactSource::Mesh(7));
        assert_eq!(contact.gap_m, 0.001);
        assert_eq!(contact.surface_velocity, [0.25, 0., 0.]);
        rod.x[1][0] = -0.00002;
        assert_eq!(rod.contact_diagnostics()[0].gap_m, -0.00002);
        rod.contacts[0].metric_scale=0.25;
        let scaled=rod.contact_diagnostics();
        assert_eq!(scaled[0].metric_scale,0.25);
        assert_eq!(scaled[0].gap_m,-0.000005);
    }
}
