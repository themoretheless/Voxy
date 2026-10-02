//! Shared layout drives pointer targeting and keyboard traversal after resize.
use voxy_ui::{
    Axis, FocusRouter, KeyAction, LayoutItem, Length, PointerAction, PointerRouter, WidgetId,
    layout_linear,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let items = [
        LayoutItem {
            id: WidgetId(1),
            length: Length::Fixed(80.0),
            enabled: true,
        },
        LayoutItem {
            id: WidgetId(2),
            length: Length::Flex(1.0),
            enabled: true,
        },
    ];
    let mut pointer = PointerRouter::new(2);
    let mut keyboard = FocusRouter::new(2);
    for width in [320.0, 640.0, 1000.0] {
        let regions = layout_linear(
            [0.0; 2],
            [width, 60.0],
            Axis::Horizontal,
            8.0,
            8.0,
            &items,
            2,
        )?;
        pointer.set_regions(&regions)?;
        keyboard.set_order(
            &regions
                .iter()
                .filter(|r| r.enabled)
                .map(|r| r.id)
                .collect::<Vec<_>>(),
        )?;
        let target = regions[1];
        pointer.move_to(Some([target.origin[0] + target.size[0] * 0.5, 30.0]));
        assert_eq!(pointer.press().action, PointerAction::Press(target.id));
        keyboard.focus_to(Some(target.id))?;
        assert_eq!(
            pointer.release().action,
            PointerAction::Release {
                id: target.id,
                clicked: true
            }
        );
        keyboard.traverse(false);
        assert_eq!(keyboard.press(), KeyAction::Press(WidgetId(1)));
        assert_eq!(
            keyboard.release(),
            KeyAction::Release {
                id: WidgetId(1),
                clicked: true
            }
        );
    }
    println!(
        "UI MENU PASS: shared responsive geometry, pointer clicks and keyboard focus across three widths"
    );
    Ok(())
}
