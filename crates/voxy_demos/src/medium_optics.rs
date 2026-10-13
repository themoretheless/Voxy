//! Bridges physics extinction fields (participating medium, suspensions,
//! droplets) to the renderer's borrowed grid view. The renderer only takes
//! `physics` as a dev-dependency, so the conversion lives in the application layer.

use physics::liquid::DropletExtinctionGrid;
use voxy_render::ExtinctionGridView;

/// Borrows a physics extinction grid for a render pass without copying it.
#[must_use]
pub fn extinction_view(grid: &DropletExtinctionGrid) -> ExtinctionGridView<'_> {
    ExtinctionGridView {
        origin: grid.origin(),
        spacing: grid.spacing(),
        shape: grid.shape(),
        extinction_m_inverse: grid.extinction_m_inverse(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use physics::liquid::{FiniteDropletGasGrid, VaporCell};
    use physics::medium::{MediumSpecies, MediumSpeciesProperties, ParticipatingMedium};
    use voxy_render::DropletExtinctionComputeInput;

    #[test]
    fn medium_extinction_feeds_the_render_input_and_matches_the_cpu_depth() {
        let volume = 0.125;
        let gas = FiniteDropletGasGrid::new(
            [0.; 3],
            [0.5; 3],
            [2, 1, 1],
            vec![
                VaporCell {
                    mass: 1.2 * volume,
                    volume,
                    temperature: 300.,
                    velocity: [0.; 3],
                    specific_heat_cv: 718.,
                };
                2
            ],
        )
        .unwrap();
        let mut medium = ParticipatingMedium::new(
            gas,
            vec![MediumSpeciesProperties {
                kind: MediumSpecies::Smoke,
                mass_extinction_m2_kg: 2.,
                albedo: 0.5,
                settling_velocity_m_s: 0.,
                diffusivity_m2_s: 0.,
                buoyancy_per_kg_m3: 0.,
            }],
        )
        .unwrap();
        medium.deposit(0, [0.25, 0.25, 0.25], 0.3).unwrap();
        let grid = medium.extinction_grid().unwrap();
        let view = extinction_view(&grid);
        assert_eq!(view.shape, [2, 1, 1]);
        assert_eq!(view.extinction_m_inverse, grid.extinction_m_inverse());
        let depth = grid
            .optical_depth_segment([0., 0.25, 0.25], [1., 0.25, 0.25])
            .unwrap();
        assert!((depth - 2. * 0.3 / volume * 0.5).abs() < 1e-12);
        let input = DropletExtinctionComputeInput::new(
            view,
            &[([0., 0.25, 0.25], [1., 0.25, 0.25])],
            1 << 16,
        )
        .unwrap();
        assert_eq!(input.workgroups()[0], 1);
    }
}
