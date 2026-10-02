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
            let neighbors = [
                cell.x.checked_sub(1).map(|x| Cell { x, ..cell }),
                (cell.x + 1 < grid.width).then_some(Cell {
                    x: cell.x + 1,
                    ..cell
                }),
                cell.y.checked_sub(1).map(|y| Cell { y, ..cell }),
                (cell.y + 1 < grid.height).then_some(Cell {
                    y: cell.y + 1,
                    ..cell
                }),
            ];
            for cell in neighbors.into_iter().flatten() {
                let next = grid.index(cell)?;
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
}
