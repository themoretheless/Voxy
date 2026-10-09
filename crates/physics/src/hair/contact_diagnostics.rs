//! Read-only physical contact geometry for accelerator divergence diagnostics.
use super::math::{V, add, dot, mul, sub};
use super::{ContactSource, HairRod};
#[derive(Clone, Debug, PartialEq)]
pub struct HairContactDiagnostic {
    pub source: ContactSource,
    pub segment: usize,
    pub fraction: f64,
    pub normal: V,
    pub target: V,
    pub surface_velocity: V,
    /// Current segment position relative to the contact plane, in metres.
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
                    gap_m: dot(sub(position, contact.target), contact.normal),
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
    }
}
