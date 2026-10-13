//! Shell measurements relative to the current stress-free reference mesh.
use physics::skin::Skin;
use serde_json::{Value, json};

pub fn measurements(skin: &Skin, solver_active: bool) -> Value {
    let metrics = match skin.surface_metrics() {
        Ok(metrics) => metrics,
        Err(error) => return json!({"valid":false,"error":error,"solverActive":solver_active}),
    };
    let mut stretches = [f64::INFINITY, f64::NEG_INFINITY];
    let mut area = [f64::INFINITY, f64::NEG_INFINITY];
    let mut thickness = [f64::INFINITY, f64::NEG_INFINITY];
    for metric in &metrics {
        stretches[0] = stretches[0].min(metric.principal_stretches[0]);
        stretches[1] = stretches[1].max(metric.principal_stretches[1]);
        area[0] = area[0].min(metric.area_ratio);
        area[1] = area[1].max(metric.area_ratio);
        thickness[0] = thickness[0].min(metric.thickness);
        thickness[1] = thickness[1].max(metric.thickness);
    }
    let energy = skin.stored_energy();
    json!({"valid":true,"solverActive":solver_active,"physicalTriangles":metrics.len(),
        "principalStretchRange":stretches,"areaRatioRange":area,"thicknessRangeM":thickness,
        "shellMassKg":skin.masses().iter().sum::<f64>(),"storedEnergyJ":energy.as_ref().ok(),
        "energyError":energy.err(),"reference":"currentStressFreeShell",
        "calibratedPhysiology":false,"scope":"skinShellOnly"})
}

#[cfg(test)]
mod tests {
    use super::*;
    use physics::skin::{SkinMaterial, patch};
    #[test]
    fn affine_surface_measures_and_invalid_geometry_are_reported() {
        let mut skin = patch(3, 3, 0.01, SkinMaterial::default()).unwrap();
        let positions = skin
            .rest_positions()
            .iter()
            .map(|p| [p[0] * 1.2, p[1] * 0.8, p[2]])
            .collect();
        skin.set_state(positions, vec![[0.; 3]; 9]).unwrap();
        let v = measurements(&skin, true);
        assert_eq!(v["physicalTriangles"], 8);
        assert!((v["principalStretchRange"][0].as_f64().unwrap() - 0.8).abs() < 1e-12);
        assert!((v["principalStretchRange"][1].as_f64().unwrap() - 1.2).abs() < 1e-12);
        assert!((v["areaRatioRange"][0].as_f64().unwrap() - 0.96).abs() < 1e-12);
        assert!(v["storedEnergyJ"].as_f64().unwrap().is_finite());
        assert_eq!(measurements(&skin, false)["solverActive"], false);
        assert!(skin.set_state(vec![[0.; 3]; 9], vec![[0.; 3]; 9]).is_err());
        assert_eq!(measurements(&skin, true), v);
    }
}
