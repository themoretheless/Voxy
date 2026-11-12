//! Bounded deterministic navigation independent of rendering and scene ownership.
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT_GRID: AtomicU64 = AtomicU64::new(1);
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Cell {
    pub x: usize,
    pub y: usize,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NavigationError {
    Dimensions,
    Capacity,
    InvalidCell,
    BlockedEndpoint,
    BudgetExceeded,
    Unreachable,
    StaleSearch,
    SearchFinished,
}
impl std::fmt::Display for NavigationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "navigation error: {self:?}")
    }
}
impl std::error::Error for NavigationError {}
#[derive(Debug)]
pub struct NavigationGrid {
    id: u64,
    width: usize,
    height: usize,
    blocked: Vec<bool>,
    revision: u64,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GridPath {
    /// Includes both endpoints; four-connected shortest path with uniform cost.
    pub cells: Vec<Cell>,
    /// Revision is local to the grid, not a globally unique grid identity.
    pub revision: u64,
    pub expanded: usize,
}
impl NavigationGrid {
    /// # Errors
    /// Rejects zero/overflowing dimensions and configured cell limit overflow.
    pub fn new(width: usize, height: usize, max_cells: usize) -> Result<Self, NavigationError> {
        if width == 0 || height == 0 {
            return Err(NavigationError::Dimensions);
        }
        let count = width
            .checked_mul(height)
            .ok_or(NavigationError::Dimensions)?;
        if count > max_cells {
            return Err(NavigationError::Capacity);
        }
        let id = NEXT_GRID
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .map_err(|_| NavigationError::Capacity)?;
        Ok(Self {
            id,
            width,
            height,
            blocked: vec![false; count],
            revision: 0,
        })
    }
    fn index(&self, cell: Cell) -> Result<usize, NavigationError> {
        if cell.x >= self.width || cell.y >= self.height {
            return Err(NavigationError::InvalidCell);
        }
        Ok(cell.y * self.width + cell.x)
    }
    fn cell(&self, index: usize) -> Cell {
        Cell {
            x: index % self.width,
            y: index / self.width,
        }
    }
    /// # Errors
    /// Rejects invalid cells and revision exhaustion without changing obstacles.
    pub fn set_blocked(&mut self, cell: Cell, blocked: bool) -> Result<(), NavigationError> {
        let index = self.index(cell)?;
        if self.blocked[index] != blocked {
            let revision = self
                .revision
                .checked_add(1)
                .ok_or(NavigationError::Capacity)?;
            self.blocked[index] = blocked;
            self.revision = revision;
        }
        Ok(())
    }
    /// Rechecks a path against current obstacles/topology. A stale revision alone
    /// does not invalidate a route if changes occurred elsewhere.
    #[must_use]
    pub fn traversable(&self, path: &GridPath) -> bool {
        !path.cells.is_empty()
            && path
                .cells
                .iter()
                .all(|cell| self.index(*cell).is_ok_and(|index| !self.blocked[index]))
            && path
                .cells
                .windows(2)
                .all(|pair| pair[0].x.abs_diff(pair[1].x) + pair[0].y.abs_diff(pair[1].y) == 1)
    }

    /// Tests whether the straight segment between `a` and `b` is completely free of blocked cells
    /// using a 4-connected supercover Bresenham/Amanatides-Woo line traversal.
    #[must_use]
    pub fn has_line_of_sight(&self, a: Cell, b: Cell) -> bool {
        let Ok(idx_a) = self.index(a) else {
            return false;
        };
        let Ok(idx_b) = self.index(b) else {
            return false;
        };
        if self.blocked[idx_a] || self.blocked[idx_b] {
            return false;
        }
        let mut x0 = a.x as isize;
        let mut y0 = a.y as isize;
        let x1 = b.x as isize;
        let y1 = b.y as isize;
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx - dy;
        while x0 != x1 || y0 != y1 {
            let e2 = 2 * err;
            if e2 > -dy && e2 < dx {
                // Diagonal step crosses corner: require both adjacent cardinal cells to be free
                let cx1 = Cell {
                    x: (x0 + sx) as usize,
                    y: y0 as usize,
                };
                let cy1 = Cell {
                    x: x0 as usize,
                    y: (y0 + sy) as usize,
                };
                if self.index(cx1).map_or(true, |i| self.blocked[i])
                    || self.index(cy1).map_or(true, |i| self.blocked[i])
                {
                    return false;
                }
                err -= dy;
                err += dx;
                x0 += sx;
                y0 += sy;
            } else if e2 > -dy {
                err -= dy;
                x0 += sx;
            } else {
                err += dx;
                y0 += sy;
            }
            let current = Cell {
                x: x0 as usize,
                y: y0 as usize,
            };
            if self.index(current).map_or(true, |i| self.blocked[i]) {
                return false;
            }
        }
        true
    }

    /// Simplifies a grid path into minimal line-of-sight waypoints (string pulling) so
    /// continuous steering agents do not zig-zag along staircase grid edges.
    #[must_use]
    pub fn smooth_path_waypoints(&self, path: &GridPath) -> Vec<Cell> {
        if path.cells.len() <= 2 {
            return path.cells.clone();
        }
        let mut waypoints = Vec::new();
        let mut anchor = 0_usize;
        waypoints.push(path.cells[anchor]);
        while anchor + 1 < path.cells.len() {
            let mut farthest = anchor + 1;
            for candidate in (anchor + 2..path.cells.len()).rev() {
                if self.has_line_of_sight(path.cells[anchor], path.cells[candidate]) {
                    farthest = candidate;
                    break;
                }
            }
            waypoints.push(path.cells[farthest]);
            anchor = farthest;
        }
        waypoints
    }
    /// Breadth-first search with deterministic left/right/up/down neighbor order.
    /// Allocates scratch proportional to grid cells, bounded by construction.
    /// # Errors
    /// Rejects invalid/blocked endpoints. Reports budget exhaustion separately
    /// from proven unreachability. `max_expanded` bounds popped non-goal cells.
    pub fn find_path(
        &self,
        start: Cell,
        goal: Cell,
        max_expanded: usize,
    ) -> Result<GridPath, NavigationError> {
        let mut search = GridSearch::new(self, start, goal)?;
        match search.advance(self, max_expanded)? {
            SearchProgress::Found(path) => Ok(path),
            SearchProgress::Pending { .. } => Err(NavigationError::BudgetExceeded),
            SearchProgress::Unreachable => Err(NavigationError::Unreachable),
        }
    }

    /// Computes a repulsive clearance potential vector `[rx, ry]` pushing away from blocked cells
    /// within Chebyshev `max_radius` around `cell` using inverse-square distance weighting.
    #[must_use]
    pub fn obstacle_clearance_repulsion(&self, cell: Cell, max_radius: usize) -> [f32; 2] {
        if self.index(cell).is_err() || max_radius == 0 {
            return [0.0, 0.0];
        }
        let r = max_radius.min(16) as isize;
        let cx = cell.x as isize;
        let cy = cell.y as isize;
        let mut rx = 0.0_f32;
        let mut ry = 0.0_f32;
        for dy in -r..=r {
            for dx in -r..=r {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = cx + dx;
                let ny = cy + dy;
                if nx < 0 || ny < 0 || nx >= self.width as isize || ny >= self.height as isize {
                    continue;
                }
                let idx = (ny as usize) * self.width + (nx as usize);
                if self.blocked[idx] {
                    let dist_sq = (dx * dx + dy * dy) as f32;
                    let dist = dist_sq.sqrt();
                    let weight = 1.0 / (dist_sq * dist);
                    rx -= (dx as f32) * weight;
                    ry -= (dy as f32) * weight;
                }
            }
        }
        [rx, ry]
    }
}

/// Evaluates the 8-connected octile distance heuristic $\Delta_{\max} + (\sqrt{2} - 1)\Delta_{\min}$
/// with a tiny deterministic cross-product tie-breaker relative to `start -> goal`.
#[must_use]
pub fn octile_distance_heuristic(current: Cell, goal: Cell, start: Cell) -> f32 {
    let dx = current.x.abs_diff(goal.x) as f32;
    let dy = current.y.abs_diff(goal.y) as f32;
    let d_max = dx.max(dy);
    let d_min = dx.min(dy);
    let octile = d_max + (std::f32::consts::SQRT_2 - 1.0) * d_min;
    let dx1 = current.x as f32 - goal.x as f32;
    let dy1 = current.y as f32 - goal.y as f32;
    let dx2 = start.x as f32 - goal.x as f32;
    let dy2 = start.y as f32 - goal.y as f32;
    let cross = (dx1 * dy2 - dx2 * dy1).abs();
    octile + cross * 1e-4
}

/// Evaluates the Tobler hiking function traversal cost multiplier (`>= 1.0` relative to optimal -5% grade)
/// for a terrain slope `dz_dx_slope = dh / dx`.
#[must_use]
pub fn tobler_hiking_cost_multiplier(dz_dx_slope: f32) -> f32 {
    if !dz_dx_slope.is_finite() {
        return 100.0;
    }
    let s = dz_dx_slope.clamp(-2.0, 2.0);
    // Tobler speed v(s) = 6 * exp(-3.5 * |s + 0.05|); cost multiplier relative to flat v(0):
    // cost(s) / cost(0) = exp(3.5 * (|s + 0.05| - 0.05)).
    (3.5 * ((s + 0.05).abs() - 0.05)).exp().clamp(0.5, 100.0)
}

#[derive(Debug, Eq, PartialEq)]
pub enum SearchProgress {
    Pending { expanded: usize },
    Found(GridPath),
    Unreachable,
}
/// Persistent BFS frontier/scratch, tied to one grid identity and obstacle revision.
/// Creation allocates grid-sized scratch; each advance bounds neighbor expansions.
/// Final route reconstruction is proportional to route length and is not separately
/// budgeted. Dropping the job cancels it and releases its memory.
#[derive(Debug)]
pub struct GridSearch {
    grid: u64,
    revision: u64,
    start: usize,
    goal: usize,
    parents: Vec<usize>,
    queue: VecDeque<usize>,
    expanded: usize,
    finished: bool,
}
impl GridSearch {
    /// # Errors
    /// Rejects invalid/blocked endpoints before allocating scratch.
    pub fn new(grid: &NavigationGrid, start: Cell, goal: Cell) -> Result<Self, NavigationError> {
        let start = grid.index(start)?;
        let goal = grid.index(goal)?;
        if grid.blocked[start] || grid.blocked[goal] {
            return Err(NavigationError::BlockedEndpoint);
        }
        let mut parents = vec![usize::MAX; grid.blocked.len()];
        parents[start] = start;
        Ok(Self {
            grid: grid.id,
            revision: grid.revision,
            start,
            goal,
            parents,
            queue: VecDeque::from([start]),
            expanded: 0,
            finished: false,
        })
    }
    /// Continues from the existing frontier without restarting the search.
    /// # Errors
    /// Rejects completed searches or a different/changed grid. Stale jobs become
    /// terminal; callers must create a new job against the new topology.
    pub fn advance(
        &mut self,
        grid: &NavigationGrid,
        budget: usize,
    ) -> Result<SearchProgress, NavigationError> {
        if self.finished {
            return Err(NavigationError::SearchFinished);
        }
        if self.grid != grid.id || self.revision != grid.revision {
            self.finished = true;
            return Err(NavigationError::StaleSearch);
        }
        let mut work = 0;
        while let Some(&index) = self.queue.front() {
            if index == self.goal {
                self.finished = true;
                let mut cells = vec![grid.cell(self.goal)];
                let mut current = self.goal;
                while current != self.start {
                    current = self.parents[current];
                    cells.push(grid.cell(current));
                }
                cells.reverse();
                return Ok(SearchProgress::Found(GridPath {
                    cells,
                    revision: self.revision,
                    expanded: self.expanded,
                }));
            }
            if work >= budget {
                return Ok(SearchProgress::Pending {
                    expanded: self.expanded,
                });
            }
            self.queue.pop_front();
            work += 1;
            self.expanded += 1;
            let cell = grid.cell(index);
            let candidates = [
                (cell.x > 0).then(|| index - 1),
                (cell.x + 1 < grid.width).then(|| index + 1),
                (cell.y > 0).then(|| index - grid.width),
                (cell.y + 1 < grid.height).then(|| index + grid.width),
            ];
            for next in candidates.into_iter().flatten() {
                if !grid.blocked[next] && self.parents[next] == usize::MAX {
                    self.parents[next] = index;
                    self.queue.push_back(next);
                }
            }
        }
        self.finished = true;
        Ok(SearchProgress::Unreachable)
    }
}

/// 2D circular agent state for local reciprocal collision avoidance (`ORCA` / `RVO`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrowdAgent2D {
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub preferred_velocity: [f32; 2],
    pub radius: f32,
    pub max_speed: f32,
}

/// 2.5D/3D heightfield navigation grid supporting step-height climbing, maximum walkable slope,
/// and overhead clearance checks on multi-elevation terrain.
#[derive(Debug)]
pub struct HeightfieldNavGrid {
    width: usize,
    height: usize,
    cell_size: f32,
    max_step_height: f32,
    max_slope_tan: f32,
    min_clearance: f32,
    elevations: Vec<f32>,
    ceiling_clearances: Vec<f32>,
    blocked: Vec<bool>,
}

impl HeightfieldNavGrid {
    /// Creates a flat heightfield navigation grid with uniform `cell_size`, `max_step_height`,
    /// `max_slope_tan` ($\tan\theta_{\max}$), and `min_clearance`.
    /// # Errors
    /// Rejects zero/overflowing dimensions, capacity overflow, or non-positive/non-finite parameters.
    pub fn new(
        width: usize,
        height: usize,
        max_cells: usize,
        cell_size: f32,
        max_step_height: f32,
        max_slope_tan: f32,
        min_clearance: f32,
    ) -> Result<Self, NavigationError> {
        if width == 0
            || height == 0
            || !cell_size.is_finite()
            || cell_size <= 0.0
            || !max_step_height.is_finite()
            || max_step_height < 0.0
            || !max_slope_tan.is_finite()
            || max_slope_tan <= 0.0
            || !min_clearance.is_finite()
            || min_clearance <= 0.0
        {
            return Err(NavigationError::Dimensions);
        }
        let count = width
            .checked_mul(height)
            .ok_or(NavigationError::Dimensions)?;
        if count > max_cells {
            return Err(NavigationError::Capacity);
        }
        Ok(Self {
            width,
            height,
            cell_size,
            max_step_height,
            max_slope_tan,
            min_clearance,
            elevations: vec![0.0; count],
            ceiling_clearances: vec![f32::INFINITY; count],
            blocked: vec![false; count],
        })
    }

    fn index(&self, cell: Cell) -> Result<usize, NavigationError> {
        if cell.x >= self.width || cell.y >= self.height {
            return Err(NavigationError::InvalidCell);
        }
        Ok(cell.y * self.width + cell.x)
    }

    fn cell(&self, index: usize) -> Cell {
        Cell {
            x: index % self.width,
            y: index / self.width,
        }
    }

    /// Sets the floor elevation, overhead clearance, and explicit blocked state for `cell`.
    /// # Errors
    /// Rejects out-of-bounds cells or non-finite floor elevation.
    pub fn set_span(
        &mut self,
        cell: Cell,
        elevation: f32,
        clearance: f32,
        blocked: bool,
    ) -> Result<(), NavigationError> {
        if !elevation.is_finite() || clearance.is_nan() || clearance < 0.0 {
            return Err(NavigationError::InvalidCell);
        }
        let idx = self.index(cell)?;
        self.elevations[idx] = elevation;
        self.ceiling_clearances[idx] = clearance;
        self.blocked[idx] = blocked;
        Ok(())
    }

    fn is_standable(&self, idx: usize) -> bool {
        !self.blocked[idx] && self.ceiling_clearances[idx] >= self.min_clearance
    }

    fn can_step(&self, from: usize, to: usize) -> bool {
        if !self.is_standable(from) || !self.is_standable(to) {
            return false;
        }
        let dh = (self.elevations[to] - self.elevations[from]).abs();
        let slope = dh / self.cell_size;
        dh <= self.max_step_height && slope <= self.max_slope_tan
    }

    /// Finds a 4-connected path from `start` to `goal` respecting step height, slope, and headroom.
    /// # Errors
    /// Rejects invalid/blocked endpoints, budget exhaustion, or unreachable goals.
    pub fn find_path_3d(
        &self,
        start: Cell,
        goal: Cell,
        max_expanded: usize,
    ) -> Result<GridPath, NavigationError> {
        let start_idx = self.index(start)?;
        let goal_idx = self.index(goal)?;
        if !self.is_standable(start_idx) || !self.is_standable(goal_idx) {
            return Err(NavigationError::BlockedEndpoint);
        }
        let mut parents = vec![usize::MAX; self.blocked.len()];
        parents[start_idx] = start_idx;
        let mut queue = VecDeque::from([start_idx]);
        let mut expanded = 0_usize;
        while let Some(&current) = queue.front() {
            if current == goal_idx {
                let mut cells = vec![self.cell(goal_idx)];
                let mut trace = goal_idx;
                while trace != start_idx {
                    trace = parents[trace];
                    cells.push(self.cell(trace));
                }
                cells.reverse();
                return Ok(GridPath {
                    cells,
                    revision: 0,
                    expanded,
                });
            }
            if expanded >= max_expanded {
                return Err(NavigationError::BudgetExceeded);
            }
            queue.pop_front();
            expanded += 1;
            let c = self.cell(current);
            let neighbors = [
                (c.x > 0).then(|| current - 1),
                (c.x + 1 < self.width).then(|| current + 1),
                (c.y > 0).then(|| current - self.width),
                (c.y + 1 < self.height).then(|| current + self.width),
            ];
            for next in neighbors.into_iter().flatten() {
                if parents[next] == usize::MAX && self.can_step(current, next) {
                    parents[next] = current;
                    queue.push_back(next);
                }
            }
        }
        Err(NavigationError::Unreachable)
    }
}

/// Computes a collision-free 2D velocity for `agent` against `neighbors` over `time_horizon`
/// using reciprocal velocity-obstacle half-plane projection.
#[must_use]
pub fn compute_reciprocal_avoidance_velocity(
    agent: CrowdAgent2D,
    neighbors: &[CrowdAgent2D],
    time_horizon: f32,
) -> [f32; 2] {
    let inv_tau = 1.0 / time_horizon.max(0.05);
    let mut vx = agent.preferred_velocity[0];
    let mut vy = agent.preferred_velocity[1];
    for other in neighbors {
        let rx = other.position[0] - agent.position[0];
        let ry = other.position[1] - agent.position[1];
        let combined_r = (agent.radius + other.radius).max(1e-4);
        let dist_sq = rx * rx + ry * ry;
        if dist_sq < 1e-10 {
            continue;
        }
        let dist = dist_sq.sqrt();
        let nx = rx / dist;
        let ny = ry / dist;
        let rel_vx = vx - other.velocity[0];
        let rel_vy = vy - other.velocity[1];
        let approach_speed = rel_vx * nx + rel_vy * ny;
        let min_clearance_rate = (dist - combined_r) * inv_tau;
        if approach_speed > min_clearance_rate {
            let excess = 0.5 * (approach_speed - min_clearance_rate);
            vx -= excess * nx;
            vy -= excess * ny;
        }
    }
    let speed_sq = vx * vx + vy * vy;
    let max_s = agent.max_speed.max(0.0);
    if speed_sq > max_s * max_s && speed_sq > 0.0 {
        let scale = max_s / speed_sq.sqrt();
        [vx * scale, vy * scale]
    } else {
        [vx, vy]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cell(x: usize, y: usize) -> Cell {
        Cell { x, y }
    }
    #[test]
    fn shortest_detour_and_dynamic_obstacles() {
        let mut grid = NavigationGrid::new(5, 3, 15).unwrap();
        grid.set_blocked(cell(2, 1), true).unwrap();
        let path = grid.find_path(cell(0, 1), cell(4, 1), 15).unwrap();
        assert_eq!(path.cells.len(), 7);
        assert!(grid.traversable(&path));
        grid.set_blocked(path.cells[1], true).unwrap();
        assert!(!grid.traversable(&path));
        let rerouted = grid.find_path(cell(0, 1), cell(4, 1), 15).unwrap();
        assert!(grid.traversable(&rerouted));
        assert_eq!(rerouted.cells.len(), 7);
    }
    #[test]
    fn budgets_endpoints_unreachable_and_dimensions() {
        assert!(NavigationGrid::new(usize::MAX, 2, usize::MAX).is_err());
        assert!(NavigationGrid::new(4, 4, 15).is_err());
        let mut grid = NavigationGrid::new(3, 1, 3).unwrap();
        assert_eq!(
            grid.find_path(cell(0, 0), cell(2, 0), 0),
            Err(NavigationError::BudgetExceeded)
        );
        assert_eq!(
            grid.find_path(cell(0, 0), cell(0, 0), 0).unwrap().cells,
            vec![cell(0, 0)]
        );
        grid.set_blocked(cell(1, 0), true).unwrap();
        assert_eq!(
            grid.find_path(cell(0, 0), cell(2, 0), 3),
            Err(NavigationError::Unreachable)
        );
        assert_eq!(
            grid.find_path(cell(1, 0), cell(2, 0), 3),
            Err(NavigationError::BlockedEndpoint)
        );
        assert_eq!(
            grid.find_path(cell(3, 0), cell(2, 0), 3),
            Err(NavigationError::InvalidCell)
        );
    }
    #[test]
    fn empty_grid_all_routes_match_manhattan_shortest_distance() {
        let grid = NavigationGrid::new(7, 6, 42).unwrap();
        for start_x in 0..7 {
            for start_y in 0..6 {
                for goal_x in 0..7 {
                    for goal_y in 0..6 {
                        let start = cell(start_x, start_y);
                        let goal = cell(goal_x, goal_y);
                        let path = grid.find_path(start, goal, 42).unwrap();
                        assert_eq!(
                            path.cells.len() - 1,
                            start_x.abs_diff(goal_x) + start_y.abs_diff(goal_y)
                        );
                        assert_eq!(path.cells.first(), Some(&start));
                        assert_eq!(path.cells.last(), Some(&goal));
                        assert!(grid.traversable(&path));
                    }
                }
            }
        }
    }
    #[test]
    fn sliced_search_matches_one_shot_and_never_exceeds_expansion_budget() {
        let grid = NavigationGrid::new(16, 16, 256).unwrap();
        let expected = grid.find_path(cell(0, 0), cell(15, 15), 256).unwrap();
        let mut search = GridSearch::new(&grid, cell(0, 0), cell(15, 15)).unwrap();
        let mut previous = 0;
        loop {
            match search.advance(&grid, 3).unwrap() {
                SearchProgress::Pending { expanded } => {
                    assert!(expanded - previous <= 3);
                    previous = expanded;
                }
                SearchProgress::Found(path) => {
                    assert_eq!(path, expected);
                    break;
                }
                SearchProgress::Unreachable => panic!("reachable grid"),
            }
        }
        assert_eq!(
            search.advance(&grid, 3),
            Err(NavigationError::SearchFinished)
        );
    }
    #[test]
    fn changed_or_foreign_grid_invalidates_job_and_zero_budget_preserves_frontier() {
        let mut grid = NavigationGrid::new(4, 4, 16).unwrap();
        let mut search = GridSearch::new(&grid, cell(0, 0), cell(3, 3)).unwrap();
        assert_eq!(
            search.advance(&grid, 0),
            Ok(SearchProgress::Pending { expanded: 0 })
        );
        search.advance(&grid, 1).unwrap();
        grid.set_blocked(cell(2, 2), true).unwrap();
        assert_eq!(search.advance(&grid, 16), Err(NavigationError::StaleSearch));
        assert_eq!(
            search.advance(&grid, 16),
            Err(NavigationError::SearchFinished)
        );
        let mut search = GridSearch::new(&grid, cell(0, 0), cell(3, 3)).unwrap();
        let foreign = NavigationGrid::new(4, 4, 16).unwrap();
        assert_eq!(
            search.advance(&foreign, 16),
            Err(NavigationError::StaleSearch)
        );
    }

    #[test]
    fn line_of_sight_string_pulling_collapses_open_grid_staircase() {
        let mut grid = NavigationGrid::new(6, 6, 36).unwrap();
        let path = grid.find_path(cell(0, 0), cell(5, 5), 36).unwrap();
        assert_eq!(path.cells.len(), 11);
        let smoothed = grid.smooth_path_waypoints(&path);
        assert_eq!(smoothed, vec![cell(0, 0), cell(5, 5)]);

        grid.set_blocked(cell(2, 2), true).unwrap();
        let detoured = grid.find_path(cell(0, 0), cell(5, 5), 36).unwrap();
        let smoothed_detour = grid.smooth_path_waypoints(&detoured);
        assert!(smoothed_detour.len() > 2 && smoothed_detour.len() < detoured.cells.len());
    }

    #[test]
    fn tobler_hiking_cost_penalizes_steep_inclines() {
        let flat = tobler_hiking_cost_multiplier(0.0);
        let gentle_down = tobler_hiking_cost_multiplier(-0.05);
        let steep_up = tobler_hiking_cost_multiplier(0.35);
        assert!((flat - 1.0).abs() < 1e-5);
        assert!(gentle_down < flat);
        assert!(steep_up > 3.0);
        let h_diag = octile_distance_heuristic(cell(0, 0), cell(3, 3), cell(0, 0));
        assert!((h_diag - 3.0 * std::f32::consts::SQRT_2).abs() < 1e-3);
        let mut grid = NavigationGrid::new(5, 5, 25).unwrap();
        grid.set_blocked(cell(1, 2), true).unwrap();
        let rep = grid.obstacle_clearance_repulsion(cell(2, 2), 2);
        assert!(rep[0] > 0.5 && rep[1].abs() < 1e-6);
    }

    #[test]
    fn heightfield_3d_grid_climbs_stairs_and_routes_around_cliffs_and_low_ceilings() {
        // 4x3 grid: direct middle row (y = 1) has a 2.0m cliff at (1, 1);
        // top row (y = 0) has gentle 0.3m steps up to 0.9m.
        let mut hf = HeightfieldNavGrid::new(4, 3, 16, 1.0, 0.45, 0.6, 1.8).unwrap();
        hf.set_span(cell(1, 1), 2.0, 3.0, false).unwrap(); // cliff in direct row
        hf.set_span(cell(1, 2), 0.0, 1.2, false).unwrap(); // low ceiling in bottom row
        hf.set_span(cell(1, 0), 0.3, 3.0, false).unwrap();
        hf.set_span(cell(2, 0), 0.6, 3.0, false).unwrap();
        hf.set_span(cell(3, 0), 0.6, 3.0, false).unwrap();
        hf.set_span(cell(3, 1), 0.3, 3.0, false).unwrap();

        let path = hf.find_path_3d(cell(0, 1), cell(3, 1), 16).unwrap();
        assert!(path.cells.contains(&cell(1, 0)));
        assert!(!path.cells.contains(&cell(1, 1)));
        assert!(!path.cells.contains(&cell(1, 2)));
    }
}
