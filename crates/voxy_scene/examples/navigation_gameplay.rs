//! Scene navigation with search work spread across ticks and topology invalidation.
use voxy_navigation::{Cell, GridSearch, NavigationError, NavigationGrid, SearchProgress};
use voxy_scene::{SceneGraph, Transform};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut grid = NavigationGrid::new(8, 5, 40)?;
    for y in 0..4 {
        grid.set_blocked(Cell { x: 3, y }, true)?;
    }
    let goal = Cell { x: 7, y: 0 };
    let mut position = Cell { x: 0, y: 0 };
    let mut scene = SceneGraph::new(1);
    let agent = scene.spawn(None, Transform::default())?;
    let mut path = None::<voxy_navigation::GridPath>;
    let mut job = None::<GridSearch>;
    let mut stale_jobs = 0;
    let mut pending_ticks = 0;
    for tick in 0..100 {
        if position == goal {
            break;
        }
        if tick == 1 {
            grid.set_blocked(Cell { x: 1, y: 0 }, true)?;
        }
        if path.as_ref().is_some_and(|path| !grid.traversable(path)) {
            path = None;
        }
        if path.is_none() {
            if job.is_none() {
                job = Some(GridSearch::new(&grid, position, goal)?);
            }
            match job.as_mut().unwrap().advance(&grid, 4) {
                Ok(SearchProgress::Pending { .. }) => {
                    pending_ticks += 1;
                    continue;
                }
                Ok(SearchProgress::Found(found)) => {
                    path = Some(found);
                    job = None;
                }
                Err(NavigationError::StaleSearch) => {
                    stale_jobs += 1;
                    job = None;
                    continue;
                }
                Ok(SearchProgress::Unreachable) => return Err("agent goal unreachable".into()),
                Err(error) => return Err(error.into()),
            }
        }
        let route = path.as_mut().unwrap();
        position = route.cells[1];
        route.cells.remove(0);
        #[allow(clippy::cast_precision_loss)]
        scene.set_local(
            agent,
            Transform {
                translation: glam::Vec3::new(position.x as f32, 0.0, position.y as f32),
                ..Transform::default()
            },
        )?;
    }
    assert_eq!(position, goal);
    assert_eq!(stale_jobs, 1);
    assert!(pending_ticks > 1);
    assert!(
        scene
            .world_matrix(agent)?
            .transform_point3(glam::Vec3::ZERO)
            .abs_diff_eq(glam::Vec3::new(7.0, 0.0, 0.0), 1e-6)
    );
    println!(
        "NAVIGATION GAMEPLAY PASS: agent reached goal, {pending_ticks} pending ticks, {stale_jobs} stale job rejected"
    );
    Ok(())
}
