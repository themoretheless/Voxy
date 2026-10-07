//! Explicit immutable input snapshot for a rest-material search backend.
//! This is a search metric, not the nonlinear physical force/energy model.
use super::{Body, Vec3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TissueSearchElement {
    pub nodes: [usize; 4],
    pub gradients_m_inverse: [Vec3; 4],
    pub reference_volume_m3: f64,
    pub shear_pa: f64,
    pub bulk_pa: f64,
}

/// Owned input for one explicitly selected immutable search solve.
/// Callers must rebuild after changing topology, material or support ownership.
/// No device allocation, persistent cache or physical-state mutation occurs here.
#[derive(Clone, Debug)]
pub struct TissueSearchSnapshot {
    elements: Vec<TissueSearchElement>,
    pinned: Vec<bool>,
    inertia_weights: Vec<f64>,
}
impl TissueSearchSnapshot {
    #[must_use]
    pub fn elements(&self) -> &[TissueSearchElement] {
        &self.elements
    }
    #[must_use]
    pub fn pinned(&self) -> &[bool] {
        &self.pinned
    }
    #[must_use]
    pub fn inertia_weights(&self) -> &[f64] {
        &self.inertia_weights
    }
}
impl Body {
    /// Export the existing implicit rest-material search operator's inputs.
    /// Effective Ogden/Maxwell rest moduli use the canonical native law.
    /// Anisotropy, active stress, contacts, history and endpoint energy remain in
    /// the physical evaluator and must not be replaced by this search snapshot.
    /// # Errors
    /// Invalid inertia weights or nonfinite/unrepresentable rest data.
    pub fn tissue_search_snapshot(
        &self,
        inertia_weights: &[f64],
    ) -> Result<TissueSearchSnapshot, &'static str> {
        if inertia_weights.len() != self.pinned.len()
            || inertia_weights
                .iter()
                .zip(&self.pinned)
                .any(|(&w, &p)| !w.is_finite() || w < 0. || (!p && w == 0.))
        {
            return Err("invalid tissue search inertia weights");
        }
        let elements: Vec<_> = self
            .elements
            .iter()
            .map(|e| {
                let (shear_pa, bulk_pa) = e
                    .viscoelastic
                    .as_ref()
                    .map_or((e.material.shear_pa, e.material.bulk_pa), |m| {
                        m.rest_moduli()
                    });
                TissueSearchElement {
                    nodes: e.nodes,
                    gradients_m_inverse: e.gradients,
                    reference_volume_m3: e.volume,
                    shear_pa,
                    bulk_pa,
                }
            })
            .collect();
        if elements.iter().any(|e| {
            e.nodes.iter().any(|&i| i >= self.pinned.len())
                || e.gradients_m_inverse
                    .iter()
                    .flatten()
                    .any(|v| !v.is_finite())
                || !e.reference_volume_m3.is_finite()
                || e.reference_volume_m3 <= 0.
                || !e.shear_pa.is_finite()
                || e.shear_pa <= 0.
                || !e.bulk_pa.is_finite()
                || e.bulk_pa <= 0.
        }) {
            return Err("invalid tissue search rest data");
        }
        Ok(TissueSearchSnapshot {
            elements,
            pinned: self.pinned.clone(),
            inertia_weights: inertia_weights.to_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biomechanics::{Material, MaxwellBranch, OgdenTerm, ViscoelasticOgden};
    fn specimen() -> Body {
        Body::new(
            vec![[0.; 3], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            vec![true, false, false, false],
            vec![(
                [0, 1, 2, 3],
                Material::from_young_poisson(300., 0.4).unwrap(),
            )],
        )
        .unwrap()
    }
    #[test]
    fn exported_operator_data_has_canonical_order_moduli_and_immutable_ownership() {
        let mut body = specimen();
        body.elements[0].viscoelastic = Some(
            ViscoelasticOgden::new(
                vec![OgdenTerm {
                    shear_pa: 100.,
                    exponent: 2.,
                }],
                1000.,
                vec![MaxwellBranch {
                    shear_pa: 700.,
                    relaxation_seconds: 0.2,
                }],
            )
            .unwrap(),
        );
        let before = format!("{body:?}");
        let snapshot = body.tissue_search_snapshot(&[0., 2., 3., 4.]).unwrap();
        assert_eq!(format!("{body:?}"), before);
        assert_eq!(snapshot.pinned(), &[true, false, false, false]);
        assert_eq!(snapshot.inertia_weights(), &[0., 2., 3., 4.]);
        assert_eq!(snapshot.elements()[0].nodes, body.elements[0].nodes);
        assert_eq!(
            snapshot.elements()[0].gradients_m_inverse,
            body.elements[0].gradients
        );
        assert!((snapshot.elements()[0].reference_volume_m3 - 1. / 6.).abs() < 1e-15);
        assert_eq!(
            (
                snapshot.elements()[0].shear_pa,
                snapshot.elements()[0].bulk_pa
            ),
            (800., 1000.)
        );
        body.pinned[0] = false;
        body.elements[0].viscoelastic = None;
        let changed = body.tissue_search_snapshot(&[1., 2., 3., 4.]).unwrap();
        assert_eq!(snapshot.pinned()[0], true);
        assert_eq!(snapshot.elements()[0].shear_pa, 800.);
        assert_eq!(changed.pinned()[0], false);
        assert_ne!(changed.elements()[0].shear_pa, 800.);
    }
    #[test]
    fn invalid_weights_and_rest_overflow_reject_without_mutating_body() {
        let mut body = specimen();
        let before = format!("{body:?}");
        for weights in [
            vec![1.; 3],
            vec![1., 0., 1., 1.],
            vec![1., f64::NAN, 1., 1.],
            vec![-1., 1., 1., 1.],
        ] {
            assert!(body.tissue_search_snapshot(&weights).is_err());
            assert_eq!(format!("{body:?}"), before);
        }
        body.elements[0].gradients[0][0] = f64::NAN;
        assert!(body.tissue_search_snapshot(&[0., 1., 1., 1.]).is_err());
    }
}

impl Body {
    /// Evaluate the canonical native rest-material search operator for backend qualification.
    /// This does not replace the nonlinear physical evaluator or mutate the body.
    /// # Errors
    /// Invalid metric inputs or nonfinite operator output.
    pub fn tissue_search_action(
        &self,
        inertia_weights: &[f64],
        direction: &[Vec3],
    ) -> Result<Vec<Vec3>, &'static str> {
        self.tissue_search_snapshot(inertia_weights)?;
        if direction.len() != self.pinned.len()
            || direction.iter().flatten().any(|x| !x.is_finite())
        {
            return Err("invalid tissue search direction");
        }
        let result = super::inertia::rest_material_action(self, inertia_weights, direction);
        if result.iter().flatten().any(|x| !x.is_finite()) {
            return Err("tissue search numerical overflow");
        }
        Ok(result)
    }
}

/// Explicit rest-material search backend. Physical force/energy admission stays native.
pub trait TissueSearchBackend: std::fmt::Debug + Send + Sync {
    /// Prepare one immutable operator for a solve, without publishing physical state.
    /// # Errors
    /// Allocation, backend or invalid input failure; callers must not silently fall back.
    fn prepare(
        &self,
        snapshot: TissueSearchSnapshot,
    ) -> Result<Box<dyn TissueSearchOperation>, &'static str>;
}
/// Prepared search operation. Backend failures propagate through the physical transaction.
pub trait TissueSearchOperation: std::fmt::Debug {
    /// Apply the search metric to one vector. Returns exactly one vector per node.
    /// # Errors
    /// Backend/transfer/numerical failure.
    fn apply(&self, direction: &[Vec3]) -> Result<Vec<Vec3>, &'static str>;
}
pub(super) fn checked_search_action(
    operation: &dyn TissueSearchOperation,
    direction: &[Vec3],
    pinned: &[bool],
) -> Result<Vec<Vec3>, &'static str> {
    let result = operation.apply(direction)?;
    if result.len() != pinned.len()
        || result.iter().flatten().any(|x| !x.is_finite())
        || result
            .iter()
            .zip(pinned)
            .any(|(v, &pin)| pin && v.iter().any(|&x| x != 0.))
    {
        return Err("invalid tissue search backend output");
    }
    Ok(result)
}
