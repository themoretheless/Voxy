//! Sample a bounded single-floor navigation region from the scene's static collision geometry.
use crate::{
    BoxCollider, CharacterBody, CharacterPhysics, KinematicBody, PhysicsError, RayQuery,
    SceneQueries, SupportQueryBudget,
};
use glam::{DVec2, DVec3, Vec3};
use std::collections::HashSet;
use voxy_input::InputMap;
use voxy_navigation::{Cell, HeightfieldNavGrid, NavigationError};
use voxy_scene::{NodeId, SceneGraph, SceneId};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneNavigationSettings {
    /// Lower X/Z corner of the sampled rectangle.
    pub origin: DVec2,
    pub width: usize,
    pub depth: usize,
    pub cell_size: f32,
    /// Search for upward-facing floor surfaces between these world Y coordinates.
    pub floor_y: [f64; 2],
    pub max_step_height: f32,
    pub max_slope_tan: f32,
    pub min_clearance: f32,
}

#[derive(Debug, thiserror::Error)]
pub enum SceneNavigationError {
    #[error("invalid scene navigation region")]
    Region,
    #[error(transparent)]
    Navigation(#[from] NavigationError),
    #[error(transparent)]
    Physics(#[from] PhysicsError),
}

#[derive(Debug)]
pub struct SceneNavigationGrid {
    scene: SceneId,
    settings: SceneNavigationSettings,
    grid: HeightfieldNavGrid,
}

impl SceneNavigationGrid {
    /// Samples the floor at each cell centre, then checks headroom against static boxes.
    /// Call again after changing geometry. The returned grid owns no scene objects.
    /// # Errors
    /// Rejects invalid bounds and insufficient query/cell budgets before publishing a grid.
    pub fn build(
        scene: &SceneGraph,
        settings: SceneNavigationSettings,
        max_cells: usize,
        collider_capacity: usize,
        budget: &mut SupportQueryBudget,
    ) -> Result<Self, SceneNavigationError> {
        let [bottom, top] = settings.floor_y;
        let scan_height = top - bottom;
        let far = settings.origin
            + DVec2::new(settings.width as f64, settings.depth as f64)
                * f64::from(settings.cell_size);
        if !settings.origin.is_finite()
            || !far.is_finite()
            || !bottom.is_finite()
            || !top.is_finite()
            || !scan_height.is_finite()
            || scan_height <= 0.
            || bottom.abs() > f64::from(f32::MAX)
            || top.abs() > f64::from(f32::MAX)
        {
            return Err(SceneNavigationError::Region);
        }
        let count = settings
            .width
            .checked_mul(settings.depth)
            .ok_or(SceneNavigationError::Region)?;
        let mut staged = *budget;
        staged.charge_work(count)?;
        let grid = HeightfieldNavGrid::new(
            settings.width,
            settings.depth,
            max_cells,
            settings.cell_size,
            settings.max_step_height,
            settings.max_slope_tan,
            settings.min_clearance,
        )?;
        let mut candidate = Self {
            scene: scene.identity(),
            settings,
            grid,
        };
        let queries = SceneQueries::from_scene(scene, collider_capacity)?;
        let static_box = |owner| is_static_box(scene, owner);
        let slope = f64::from(settings.max_slope_tan);
        let min_normal_y = 1. / (1. + slope * slope).sqrt();
        for z in 0..settings.depth {
            for x in 0..settings.width {
                let cell = Cell { x, y: z };
                let xz = candidate.center(cell);
                let floor = queries.cast_ray(
                    RayQuery {
                        origin: DVec3::new(xz.x, top, xz.y),
                        direction: DVec3::NEG_Y,
                        max_distance: scan_height,
                        solid: true,
                    },
                    &mut staged,
                    static_box,
                )?;
                let Some(floor) =
                    floor.filter(|hit| hit.normal.is_some_and(|n| n.y >= min_normal_y))
                else {
                    candidate.grid.set_span(cell, 0., 0., true)?;
                    continue;
                };
                let ceiling = queries.cast_ray(
                    RayQuery {
                        origin: floor.position,
                        direction: DVec3::Y,
                        max_distance: f64::from(settings.min_clearance),
                        solid: true,
                    },
                    &mut staged,
                    |owner| owner != floor.owner && static_box(owner),
                )?;
                let clearance = ceiling.map_or(f64::INFINITY, |hit| hit.distance);
                candidate.grid.set_span(
                    cell,
                    floor.position.y as f32,
                    clearance as f32,
                    clearance < f64::from(settings.min_clearance),
                )?;
            }
        }
        *budget = staged;
        Ok(candidate)
    }

    fn center(&self, cell: Cell) -> DVec2 {
        self.settings.origin
            + DVec2::new(cell.x as f64 + 0.5, cell.y as f64 + 0.5)
                * f64::from(self.settings.cell_size)
    }

    /// Returns world-space floor waypoints for the grid path, including both endpoints.
    /// # Errors
    /// Rejects invalid/blocked endpoints, unreachable goals or search budget exhaustion.
    pub fn waypoints(
        &self,
        start: Cell,
        goal: Cell,
        max_expanded: usize,
    ) -> Result<Vec<DVec3>, NavigationError> {
        self.grid
            .find_path_3d(start, goal, max_expanded)?
            .cells
            .into_iter()
            .map(|cell| {
                let xz = self.center(cell);
                let y = self
                    .grid
                    .walkable_elevation(cell)?
                    .ok_or(NavigationError::BlockedEndpoint)?;
                Ok(DVec3::new(xz.x, f64::from(y), xz.y))
            })
            .collect()
    }
}

/// A scene-authored goal within a single navigation layer.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpcDestination {
    /// Lower X/Z corner of the region.
    pub origin: [f64; 2],
    /// Number of cells along X and Z.
    pub cells: [usize; 2],
    pub cell_size: f32,
    pub floor_y: [f64; 2],
    pub max_step_height: f32,
    pub max_slope_tan: f32,
    pub min_clearance: f32,
    /// World X/Z within the destination cell. The route ends at its centre.
    pub goal: [f64; 2],
}

impl NpcDestination {
    pub(crate) fn validate(self) -> Result<(), PhysicsError> {
        let [min_y, max_y] = self.floor_y;
        let [width, depth] = self.cells;
        if width == 0
            || depth == 0
            || width
                .checked_mul(depth)
                .is_none_or(|count| count > NpcRoute::MAX_WAYPOINTS)
            || self
                .origin
                .iter()
                .chain(&self.goal)
                .chain(&self.floor_y)
                .any(|value| !value.is_finite() || value.abs() > 1e6)
            || max_y <= min_y
            || !self.cell_size.is_finite()
            || self.cell_size <= 0.
            || !self.max_step_height.is_finite()
            || self.max_step_height < 0.
            || !self.max_slope_tan.is_finite()
            || self.max_slope_tan <= 0.
            || !self.min_clearance.is_finite()
            || self.min_clearance <= 0.
        {
            return Err(PhysicsError::InvalidMotion);
        }
        Ok(())
    }

    fn settings(self) -> SceneNavigationSettings {
        SceneNavigationSettings {
            origin: DVec2::from_array(self.origin),
            width: self.cells[0],
            depth: self.cells[1],
            cell_size: self.cell_size,
            floor_y: self.floor_y,
            max_step_height: self.max_step_height,
            max_slope_tan: self.max_slope_tan,
            min_clearance: self.min_clearance,
        }
    }

    fn cell(self, position: DVec2) -> Result<Cell, SceneNavigationError> {
        let offset = (position - DVec2::from_array(self.origin)) / f64::from(self.cell_size);
        let [x, z] = offset.to_array();
        if !offset.is_finite()
            || x < 0.
            || z < 0.
            || x >= self.cells[0] as f64
            || z >= self.cells[1] as f64
        {
            return Err(SceneNavigationError::Region);
        }
        Ok(Cell {
            x: x as usize,
            y: z as usize,
        })
    }
}

/// Plans routes for active destinations without changing the scene. The caller
/// installs the returned components on the detached runtime scene before ticking.
/// # Errors
/// Rejects conflicting authored routes, unreachable goals and exhausted budgets.
pub fn prepare_npc_routes(
    scene: &SceneGraph,
    collider_capacity: usize,
    budget: &mut SupportQueryBudget,
) -> Result<Vec<(NodeId, NpcRoute)>, SceneNavigationError> {
    let mut staged = *budget;
    let mut routes = Vec::new();
    let mut grids: Vec<SceneNavigationGrid> = Vec::new();
    for (owner, destination) in scene.active_components::<NpcDestination>() {
        destination.validate()?;
        if scene
            .component::<NpcRoute>(owner)
            .map_err(PhysicsError::from)?
            .is_some()
            || scene
                .component::<CharacterBody>(owner)
                .map_err(PhysicsError::from)?
                .is_none()
        {
            return Err(PhysicsError::InvalidMotion.into());
        }
        let position = crate::character_world::translation(scene, owner)?.as_dvec3();
        let start = destination.cell(DVec2::new(position.x, position.z))?;
        let goal = destination.cell(DVec2::from_array(destination.goal))?;
        let settings = destination.settings();
        let grid = if let Some(grid) = grids.iter().find(|grid| grid.settings == settings) {
            grid
        } else {
            grids.push(SceneNavigationGrid::build(
                scene,
                settings,
                NpcRoute::MAX_WAYPOINTS,
                collider_capacity,
                &mut staged,
            )?);
            grids.last().expect("new navigation grid")
        };
        let waypoints = grid.waypoints(start, goal, NpcRoute::MAX_WAYPOINTS)?;
        routes.push((
            owner,
            NpcRoute::new(
                waypoints
                    .into_iter()
                    .map(|point| point.to_array())
                    .collect(),
            )?,
        ));
    }
    *budget = staged;
    Ok(routes)
}

/// Authored world-space route for the ordinary character tick.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "NpcRouteWire")]
pub struct NpcRoute {
    waypoints: Vec<[f64; 3]>,
    #[serde(skip)]
    signature: [u8; 32],
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct NpcRouteWire {
    waypoints: Vec<[f64; 3]>,
}

impl TryFrom<NpcRouteWire> for NpcRoute {
    type Error = PhysicsError;

    fn try_from(value: NpcRouteWire) -> Result<Self, Self::Error> {
        Self::new(value.waypoints)
    }
}

impl NpcRoute {
    pub(crate) const MAX_WAYPOINTS: usize = 1024;
    const ARRIVAL_TOLERANCE: f64 = 1e-5;

    /// # Errors
    /// Rejects empty, oversized or nonfinite routes and coordinates outside the solver bounds.
    pub fn new(waypoints: Vec<[f64; 3]>) -> Result<Self, PhysicsError> {
        if waypoints.is_empty() || waypoints.len() > Self::MAX_WAYPOINTS {
            return Err(PhysicsError::InvalidMotion);
        }
        let mut hash = blake3::Hasher::new();
        for &waypoint in waypoints.iter().flatten() {
            if !waypoint.is_finite() || waypoint.abs() > 1e6 {
                return Err(PhysicsError::InvalidMotion);
            }
            hash.update(&waypoint.to_bits().to_le_bytes());
        }
        Ok(Self {
            waypoints,
            signature: *hash.finalize().as_bytes(),
        })
    }

    /// The final world-space waypoint of this validated route.
    #[must_use]
    pub fn goal(&self) -> DVec3 {
        DVec3::from_array(*self.waypoints.last().expect("nonempty route"))
    }

    pub(crate) fn completed(&self, cursor: RouteCursor) -> bool {
        cursor.signature == self.signature && cursor.next == self.waypoints.len()
    }

    pub(crate) fn advance(&self, cursor: &mut RouteCursor, position: DVec3) {
        while self.waypoints.get(cursor.next).is_some_and(|target| {
            planar_delta(DVec3::from_array(*target), position).length_squared()
                <= Self::ARRIVAL_TOLERANCE * Self::ARRIVAL_TOLERANCE
        }) {
            cursor.next += 1;
        }
    }

    pub(crate) fn steering(
        &self,
        previous: Option<RouteCursor>,
        position: DVec3,
        max_speed: f64,
        dt: f64,
    ) -> Result<(RouteCursor, [f64; 2]), PhysicsError> {
        let signature = self.signature;
        let mut cursor = previous
            .filter(|cursor| cursor.signature == signature)
            .unwrap_or(RouteCursor { signature, next: 0 });
        if cursor.next > self.waypoints.len() {
            return Err(PhysicsError::InvalidMotion);
        }
        self.advance(&mut cursor, position);
        let speed = self
            .waypoints
            .get(cursor.next)
            .map_or(DVec3::ZERO, |target| {
                (planar_delta(DVec3::from_array(*target), position) / dt)
                    .clamp_length_max(max_speed)
            });
        Ok((cursor, [speed.x, speed.z]))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RouteCursor {
    pub(crate) signature: [u8; 32],
    pub(crate) next: usize,
}

fn is_static_box(scene: &SceneGraph, owner: NodeId) -> bool {
    scene
        .component::<BoxCollider>(owner)
        .is_ok_and(|body| body.is_some())
        && scene
            .component::<CharacterBody>(owner)
            .is_ok_and(|body| body.is_none())
        && scene
            .component::<KinematicBody>(owner)
            .is_ok_and(|body| body.is_none())
        && scene
            .component::<crate::LiquidBody>(owner)
            .is_ok_and(|body| body.is_none())
}

/// Runtime cursor over an immutable route. Collision rejection cannot advance it.
#[derive(Debug)]
pub struct NavigationFollower {
    scene: SceneId,
    owner: NodeId,
    waypoints: Vec<DVec3>,
    next: usize,
    tolerance: f64,
}

impl SceneNavigationGrid {
    /// Plans a route for an active character in the supplied scene.
    /// # Errors
    /// Rejects invalid character ownership, path endpoints and search budget exhaustion.
    pub fn follower(
        &self,
        scene: &SceneGraph,
        owner: NodeId,
        start: Cell,
        goal: Cell,
        max_expanded: usize,
    ) -> Result<NavigationFollower, SceneNavigationError> {
        if self.scene != scene.identity()
            || scene
                .component::<CharacterBody>(owner)
                .map_err(PhysicsError::from)?
                .is_none()
            || !scene
                .active_in_hierarchy(owner)
                .map_err(PhysicsError::from)?
        {
            return Err(PhysicsError::InvalidMotion.into());
        }
        Ok(NavigationFollower {
            scene: scene.identity(),
            owner,
            waypoints: self.waypoints(start, goal, max_expanded)?,
            next: 0,
            tolerance: f64::from(self.settings.cell_size) * 1e-5,
        })
    }
}

impl NavigationFollower {
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.next == self.waypoints.len()
    }

    fn cursor_at(&self, mut next: usize, position: DVec3) -> usize {
        while self.waypoints.get(next).is_some_and(|target| {
            planar_delta(*target, position).length_squared() <= self.tolerance * self.tolerance
        }) {
            next += 1;
        }
        next
    }
}

fn planar_delta(target: DVec3, position: DVec3) -> DVec3 {
    DVec3::new(target.x - position.x, 0., target.z - position.z)
}

/// Advances one shared physics tick and commits all route cursors after collision resolution.
/// Use a neutral input map for autonomous-only simulation. An input map with active
/// player actions applies the solver's normal input in addition to the route motion.
/// # Errors
/// Rejects duplicate or foreign followers, inactive or detached characters and physics
/// failures. Rejected physics ticks preserve route cursors.
pub fn step_navigation_followers(
    physics: &mut CharacterPhysics,
    scene: &mut SceneGraph,
    input: &mut InputMap,
    dt: f64,
    followers: &mut [NavigationFollower],
) -> Result<(), PhysicsError> {
    if !dt.is_finite() || dt <= 0. {
        return Err(PhysicsError::InvalidStep);
    }
    if followers.len() > physics.max_bodies {
        return Err(PhysicsError::InvalidMotion);
    }
    let mut owners = HashSet::with_capacity(followers.len());
    let mut motions = Vec::<(NodeId, Vec3)>::with_capacity(followers.len());
    let mut cursors = Vec::with_capacity(followers.len());
    for follower in followers.iter() {
        let owner = follower.owner;
        if follower.scene != scene.identity()
            || !owners.insert(owner)
            || !scene.active_in_hierarchy(owner)?
        {
            return Err(PhysicsError::InvalidMotion);
        }
        let body = scene
            .component::<CharacterBody>(owner)?
            .ok_or(PhysicsError::InvalidMotion)?;
        let position = match physics.accepted_pose(scene, owner)? {
            Some(pose) => pose.physical_center,
            None => crate::character_world::translation(scene, owner)?.as_dvec3(),
        };
        let next = follower.cursor_at(follower.next, position);
        if let Some(target) = follower.waypoints.get(next) {
            let max_distance = body.speed * dt;
            if !max_distance.is_finite() || max_distance < 0. {
                return Err(PhysicsError::InvalidMotion);
            }
            motions.push((
                owner,
                planar_delta(*target, position)
                    .clamp_length_max(max_distance)
                    .as_vec3(),
            ));
        }
        cursors.push(next);
    }

    physics.fixed_step_with_motion(scene, input, dt, &motions)?;
    for (follower, next) in followers.iter_mut().zip(cursors) {
        follower.next = physics.states.get(&follower.owner).map_or(next, |body| {
            follower.cursor_at(next, crate::precise_center(body.state.body))
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxy_scene::Transform;

    fn spawn_test_collider(
        scene: &mut SceneGraph,
        translation: Vec3,
        half_extents: [f32; 3],
    ) -> Result<NodeId, Box<dyn std::error::Error>> {
        let node = scene.spawn(
            None,
            Transform {
                translation,
                ..Default::default()
            },
        )?;
        scene.insert_component(node, BoxCollider { half_extents })?;
        Ok(node)
    }

    #[test]
    fn scene_path_routes_a_swept_character_around_a_wall_and_low_ceiling()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut scene = SceneGraph::new(8);
        for (translation, half_extents) in [
            (Vec3::new(1.5, -0.25, 1.5), [1.5, 0.25, 1.5]),
            (Vec3::new(1.5, 0.6, 0.5), [0.4, 0.6, 0.4]),
            (Vec3::new(1.5, 0.55, 1.5), [0.4, 0.3, 0.4]),
        ] {
            spawn_test_collider(&mut scene, translation, half_extents)?;
        }
        let player = scene.spawn(
            None,
            Transform {
                translation: Vec3::new(0.5, 0.21, 0.5),
                ..Default::default()
            },
        )?;
        scene.insert_component(
            player,
            CharacterBody {
                half_extents: [0.2; 3],
                ..Default::default()
            },
        )?;
        let settings = SceneNavigationSettings {
            origin: DVec2::ZERO,
            width: 3,
            depth: 3,
            cell_size: 1.,
            floor_y: [-0.1, 0.2],
            max_step_height: 0.02,
            max_slope_tan: 0.6,
            min_clearance: 0.4,
        };
        let mut short_budget = SupportQueryBudget::new(10)?;
        assert!(matches!(
            SceneNavigationGrid::build(&scene, settings, 9, 8, &mut short_budget),
            Err(SceneNavigationError::Physics(PhysicsError::SweepBudget))
        ));
        assert_eq!(short_budget.remaining(), 10);
        let grid =
            SceneNavigationGrid::build(&scene, settings, 9, 8, &mut SupportQueryBudget::new(128)?)?;
        let path = grid.waypoints(Cell { x: 0, y: 0 }, Cell { x: 2, y: 0 }, 9)?;
        assert_eq!(path.len(), 7);
        assert!(path.contains(&DVec3::new(1.5, 0., 2.5)));
        assert!(matches!(
            grid.waypoints(Cell { x: 0, y: 0 }, Cell { x: 1, y: 1 }, 9),
            Err(NavigationError::BlockedEndpoint)
        ));

        let mut physics = CharacterPhysics::new(&scene, 1, 8);
        let mut input = crate::player_input()?;
        let mut followers =
            [grid.follower(&scene, player, Cell { x: 0, y: 0 }, Cell { x: 2, y: 0 }, 9)?];
        let blocker = spawn_test_collider(&mut scene, Vec3::new(0.5, 0.6, 1.), [0.4, 0.6, 0.1])?;
        for _ in 0..120 {
            step_navigation_followers(
                &mut physics,
                &mut scene,
                &mut input,
                1. / 60.,
                &mut followers,
            )?;
        }
        assert!(!followers[0].is_finished());
        assert_eq!(followers[0].next, 1);
        assert!(scene.local(player)?.translation.z < 0.71);
        assert!(
            step_navigation_followers(
                &mut physics,
                &mut scene,
                &mut input,
                f64::NAN,
                &mut followers
            )
            .is_err()
        );
        assert_eq!(followers[0].next, 1);

        scene.remove_component::<BoxCollider>(blocker)?;
        for _ in 0..610 {
            step_navigation_followers(
                &mut physics,
                &mut scene,
                &mut input,
                1. / 60.,
                &mut followers,
            )?;
        }
        assert!(followers[0].is_finished());
        let position = scene.local(player)?.translation;
        assert!((position.x - 2.5).abs() < 1e-4);
        assert!((position.z - 0.5).abs() < 1e-4);
        Ok(())
    }
}
