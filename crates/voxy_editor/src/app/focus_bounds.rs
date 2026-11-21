//! Full world-bounds calculation and camera framing (`F`) across selected hierarchies and components.

use crate::*;

fn accumulate_local_box(
    world: glam::DMat4,
    local_min: glam::DVec3,
    local_max: glam::DVec3,
    min: &mut glam::DVec3,
    max: &mut glam::DVec3,
) -> Result<(), Box<dyn std::error::Error>> {
    for corner in 0..8 {
        let local = glam::DVec3::new(
            if corner & 1 == 0 {
                local_min.x
            } else {
                local_max.x
            },
            if corner & 2 == 0 {
                local_min.y
            } else {
                local_max.y
            },
            if corner & 4 == 0 {
                local_min.z
            } else {
                local_max.z
            },
        );
        let p = world.transform_point3(local);
        if !p.is_finite() {
            return Err("nonfinite displayed object bounds".into());
        }
        *min = min.min(p);
        *max = max.max(p);
    }
    Ok(())
}

impl App {
    fn collect_selection_hierarchy(
        &self,
        roots: &[NodeId],
    ) -> Result<Vec<NodeId>, Box<dyn std::error::Error>> {
        let mut children: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        for &node in &self.instances {
            if let Some(parent) = self.scene.parent(node)? {
                children.entry(parent).or_default().push(node);
            }
        }
        let mut seen = BTreeSet::new();
        let mut visited = Vec::new();
        let mut stack: Vec<(NodeId, bool)> =
            roots.iter().rev().copied().map(|n| (n, true)).collect();
        while let Some((node, is_root)) = stack.pop() {
            if !seen.insert(node) {
                continue;
            }
            if !is_root && !self.scene.active_self(node)? {
                continue;
            }
            visited.push(node);
            if let Some(child_nodes) = children.get(&node) {
                for &child in child_nodes.iter().rev() {
                    stack.push((child, false));
                }
            }
        }
        Ok(visited)
    }

    fn accumulate_node_bounds(
        &self,
        node: NodeId,
        vol_min: &mut glam::DVec3,
        vol_max: &mut glam::DVec3,
    ) -> Result<glam::DVec3, Box<dyn std::error::Error>> {
        let world = self.scene.world_matrix(node)?.as_dmat4();
        let origin = world.w_axis.truncate();
        if !origin.is_finite() {
            return Err("nonfinite displayed object origin".into());
        }
        for mesh in self.displayed_model_meshes(node)? {
            for vertex in mesh.vertices() {
                let p = world.transform_point3(Vec3::from_array(vertex.position).as_dvec3());
                if !p.is_finite() {
                    return Err("nonfinite displayed model bounds".into());
                }
                *vol_min = vol_min.min(p);
                *vol_max = vol_max.max(p);
            }
        }
        for half_extents in [
            self.scene
                .component::<BoxCollider>(node)?
                .map(|c| c.half_extents),
            self.scene
                .component::<voxy_gameplay::BoxTrigger>(node)?
                .map(|t| t.half_extents),
            self.scene
                .component::<CharacterBody>(node)?
                .map(|b| b.half_extents),
        ]
        .into_iter()
        .flatten()
        {
            let h = Vec3::from_array(half_extents).as_dvec3();
            accumulate_local_box(world, -h, h, vol_min, vol_max)?;
        }
        if let Some(fog) = self.scene.component::<voxy_scene::FogVolume>(node)?
            && fog.enabled
        {
            let s = Vec3::from_array(fog.size).as_dvec3();
            accumulate_local_box(world, glam::DVec3::ZERO, s, vol_min, vol_max)?;
        }
        Ok(origin)
    }

    /// Computes combined world-space bounds across all selected objects and their active subtrees.
    pub(crate) fn compute_selection_world_bounds(
        &self,
    ) -> Result<Option<(glam::DVec3, glam::DVec3)>, Box<dyn std::error::Error>> {
        let roots: Vec<NodeId> = self
            .selected_indices()
            .into_iter()
            .filter_map(|index| self.instances.get(index).copied())
            .collect();
        if roots.is_empty() {
            return Ok(None);
        }
        let mut total_min = glam::DVec3::splat(f64::INFINITY);
        let mut total_max = glam::DVec3::splat(f64::NEG_INFINITY);
        let mut fallback_min = glam::DVec3::splat(f64::INFINITY);
        let mut fallback_max = glam::DVec3::splat(f64::NEG_INFINITY);
        let mut has_point_only_root = false;
        for root in roots {
            let visited = self.collect_selection_hierarchy(&[root])?;
            let mut sub_vol_min = glam::DVec3::splat(f64::INFINITY);
            let mut sub_vol_max = glam::DVec3::splat(f64::NEG_INFINITY);
            let mut sub_pt_min = glam::DVec3::splat(f64::INFINITY);
            let mut sub_pt_max = glam::DVec3::splat(f64::NEG_INFINITY);
            for node in visited {
                let origin =
                    self.accumulate_node_bounds(node, &mut sub_vol_min, &mut sub_vol_max)?;
                sub_pt_min = sub_pt_min.min(origin);
                sub_pt_max = sub_pt_max.max(origin);
            }
            if sub_vol_min.is_finite() {
                total_min = total_min.min(sub_vol_min);
                total_max = total_max.max(sub_vol_max);
            } else if sub_pt_min.is_finite() {
                has_point_only_root = true;
                fallback_min = fallback_min.min(sub_pt_min);
                fallback_max = fallback_max.max(sub_pt_max);
            }
        }
        if total_min.is_finite() {
            if has_point_only_root && fallback_min.is_finite() {
                total_min = total_min.min(fallback_min);
                total_max = total_max.max(fallback_max);
            }
            return Ok(Some((total_min, total_max)));
        }
        if fallback_min.is_finite() && (fallback_max - fallback_min).length_squared() > 1e-9 {
            return Ok(Some((fallback_min, fallback_max)));
        }
        Ok(None)
    }

    pub(crate) fn focus_selected_bounds(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let Some(primary) = self.instances.get(self.selected).copied() else {
            return Ok(());
        };
        let bounds = self.compute_selection_world_bounds()?;
        let mut camera = self.camera.clone();
        if let Some((min, max)) = bounds {
            let viewport = self.window.as_ref().map_or([800, 600], |window| {
                let size = window.inner_size();
                [size.width, size.height]
            });
            let regions = view_layout::regions(viewport, self.split_views());
            let region = regions
                .get(usize::from(self.active_view))
                .or_else(|| regions.first())
                .ok_or("empty focus viewport")?;
            let size = Vec2::new(region[2] as f32, region[3] as f32);
            if self.panels.is_some() {
                let scale = self
                    .window
                    .as_ref()
                    .map_or(1., |window| window.scale_factor() as f32);
                let safe = panels::safe_view_rect(viewport, *region, scale)
                    .ok_or("focus viewport is obscured by panels")?;
                camera.fit_bounds_in_rect(min, max, size, safe)?;
            } else {
                camera.fit_bounds(min, max, size)?;
            }
        } else {
            let world = self.scene.world_matrix(primary)?;
            camera.target = world.w_axis.truncate();
            camera.legacy = false;
            if !camera.valid() {
                return Err("displayed model center exceeds camera limits".into());
            }
        }
        self.camera = camera;
        Ok(())
    }
}
