//! Deterministic VR input-path fixture; no runtime, GPU or headset is simulated.
//! Hand states/locations replace only the boundary supplied by a real runtime.
use openxr::{ActionInput, ActionState, Posef, SpaceLocation, SpaceLocationFlags};
use voxy_ui::{HitRegion, PointerAction, PointerRouter, WidgetId};
use voxy_xr::{HandInput, XrPanelPointer, hit_test_quad};

fn state<T: ActionInput>(value: T) -> ActionState<T> {
    ActionState {
        current_state: value,
        changed_since_last_sync: false,
        last_change_time: openxr::Time::from_nanos(0),
        is_active: true,
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut pointer = PointerRouter::new(2);
    pointer.set_regions(&[
        HitRegion {
            id: WidgetId(1),
            origin: [0.0, 0.0],
            size: [400.0, 200.0],
            enabled: true,
        },
        HitRegion {
            id: WidgetId(2),
            origin: [150.0, 75.0],
            size: [100.0, 50.0],
            enabled: true,
        },
    ])?;
    let mut adapter = XrPanelPointer::default();
    let mut hand = HandInput {
        select: state(false),
        trigger: state(0.0),
        squeeze: state(0.0),
        stick: state(openxr::Vector2f { x: 0.0, y: 0.0 }),
        grip_active: true,
        aim_active: true,
    };
    let mut aim = SpaceLocation {
        pose: Posef::IDENTITY,
        location_flags: SpaceLocationFlags::POSITION_VALID | SpaceLocationFlags::ORIENTATION_VALID,
    };
    let mut panel = Posef::IDENTITY;
    panel.position.z = -2.0;
    let size = openxr::Extent2Df {
        width: 2.0,
        height: 1.0,
    };
    let extent = [400.0, 200.0];
    let mut dispatch = |hand: &HandInput,
                        aim: &SpaceLocation|
     -> Result<PointerAction, Box<dyn std::error::Error>> {
        let hit = hit_test_quad(hand.aim_active, aim, panel, size, 4.0)?;
        Ok(adapter
            .update_hand(&mut pointer, hit, hand, aim, extent)?
            .action)
    };
    assert_eq!(dispatch(&hand, &aim)?, PointerAction::None);
    hand.select.current_state = true;
    assert_eq!(dispatch(&hand, &aim)?, PointerAction::Press(WidgetId(2)));
    assert_eq!(dispatch(&hand, &aim)?, PointerAction::None);
    hand.select.current_state = false;
    assert_eq!(
        dispatch(&hand, &aim)?,
        PointerAction::Release {
            id: WidgetId(2),
            clicked: true
        }
    );
    hand.select.current_state = true;
    assert_eq!(dispatch(&hand, &aim)?, PointerAction::Press(WidgetId(2)));
    // A tracked ray outside the panel preserves capture until release, without click.
    aim.pose.position.x = 3.0;
    assert_eq!(dispatch(&hand, &aim)?, PointerAction::None);
    hand.select.current_state = false;
    assert_eq!(
        dispatch(&hand, &aim)?,
        PointerAction::Release {
            id: WidgetId(2),
            clicked: false
        }
    );
    aim.pose.position.x = 0.0;
    hand.select.current_state = true;
    assert_eq!(dispatch(&hand, &aim)?, PointerAction::Press(WidgetId(2)));
    // Invalid location data must not be read or turned into a click.
    aim.location_flags = SpaceLocationFlags::EMPTY;
    aim.pose.position.x = f32::NAN;
    assert_eq!(dispatch(&hand, &aim)?, PointerAction::Cancel(WidgetId(2)));
    aim.location_flags = SpaceLocationFlags::POSITION_VALID | SpaceLocationFlags::ORIENTATION_VALID;
    aim.pose.position.x = 0.0;
    assert_eq!(dispatch(&hand, &aim)?, PointerAction::None);
    hand.select.current_state = false;
    assert_eq!(dispatch(&hand, &aim)?, PointerAction::None);
    hand.select.current_state = true;
    assert_eq!(dispatch(&hand, &aim)?, PointerAction::Press(WidgetId(2)));
    hand.select.is_active = false;
    assert_eq!(dispatch(&hand, &aim)?, PointerAction::Cancel(WidgetId(2)));
    println!(
        "XR PANEL INPUT PASS: aim pose -> panel UV -> logical UI -> topmost click, drag-out, tracking/action cancellation, held recovery; no headset acceptance"
    );
    Ok(())
}
