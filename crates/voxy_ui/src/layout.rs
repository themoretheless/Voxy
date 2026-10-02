use crate::{HitRegion, UiError, WidgetId};
use std::collections::BTreeSet;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Length {
    Fixed(f32),
    Flex(f32),
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutItem {
    pub id: WidgetId,
    pub length: Length,
    pub enabled: bool,
}
/// Creates logical hit/render rectangles in item order. Fixed lengths consume
/// space first; flex weights divide the remainder. Cross-axis fills the padded
/// container. No implicit shrinking, scrolling or clipping occurs.
/// # Errors
/// Rejects invalid geometry/weights, duplicate IDs, item cap and insufficient space.
#[allow(clippy::cast_possible_truncation)] // Final f32 geometry is validated before returning.
pub fn layout_linear(
    origin: [f32; 2],
    size: [f32; 2],
    axis: Axis,
    padding: f32,
    gap: f32,
    items: &[LayoutItem],
    max_items: usize,
) -> Result<Vec<HitRegion>, UiError> {
    if items.len() > max_items {
        return Err(UiError::Capacity);
    }
    if !(HitRegion {
        id: WidgetId(0),
        origin,
        size,
        enabled: true,
    })
    .valid()
        || !padding.is_finite()
        || padding < 0.0
        || !gap.is_finite()
        || gap < 0.0
    {
        return Err(UiError::InvalidGeometry);
    }
    let main = usize::from(axis != Axis::Horizontal);
    let cross = 1 - main;
    let inner = f64::from(size[main]) - 2.0 * f64::from(padding);
    let cross_length = f64::from(size[cross]) - 2.0 * f64::from(padding);
    let mut ids = BTreeSet::new();
    let mut fixed = 0.0;
    let mut weights = 0.0;
    for item in items {
        if !ids.insert(item.id) {
            return Err(UiError::DuplicateId);
        }
        let (Length::Fixed(value) | Length::Flex(value)) = item.length;
        if !value.is_finite() || value <= 0.0 {
            return Err(UiError::InvalidGeometry);
        }
        match item.length {
            Length::Fixed(value) => fixed += f64::from(value),
            Length::Flex(value) => weights += f64::from(value),
        }
    }
    let gaps = u32::try_from(items.len().saturating_sub(1)).map_err(|_| UiError::Capacity)?;
    let available = inner - fixed - f64::from(gap) * f64::from(gaps);
    if cross_length <= 0.0 || available < 0.0 || (weights > 0.0 && available <= 0.0) {
        return Err(UiError::InsufficientSpace);
    }
    let mut cursor = f64::from(origin[main]) + f64::from(padding);
    let mut regions = Vec::with_capacity(items.len());
    for item in items {
        let length = match item.length {
            Length::Fixed(value) => f64::from(value),
            Length::Flex(value) => available * f64::from(value) / weights,
        };
        let mut position = origin;
        position[main] = cursor as f32;
        position[cross] = (f64::from(origin[cross]) + f64::from(padding)) as f32;
        let mut extent = size;
        extent[main] = length as f32;
        extent[cross] = cross_length as f32;
        let region = HitRegion {
            id: item.id,
            origin: position,
            size: extent,
            enabled: item.enabled,
        };
        if !region.valid() {
            return Err(UiError::InvalidGeometry);
        }
        regions.push(region);
        cursor += length + f64::from(gap);
    }
    Ok(regions)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resize_weights_and_vertical_geometry() {
        let items = [
            LayoutItem {
                id: WidgetId(1),
                length: Length::Fixed(20.0),
                enabled: true,
            },
            LayoutItem {
                id: WidgetId(2),
                length: Length::Flex(1.0),
                enabled: true,
            },
            LayoutItem {
                id: WidgetId(3),
                length: Length::Flex(3.0),
                enabled: false,
            },
        ];
        let regions = layout_linear(
            [10.0, 20.0],
            [100.0, 50.0],
            Axis::Horizontal,
            5.0,
            5.0,
            &items,
            3,
        )
        .unwrap();
        for (actual, expected) in regions.iter().zip([20.0, 15.0, 45.0]) {
            assert!((actual.size[0] - expected).abs() < f32::EPSILON);
        }
        assert!((regions[2].origin[0] - 60.0).abs() < f32::EPSILON);
        let vertical = layout_linear(
            [20.0, 10.0],
            [50.0, 100.0],
            Axis::Vertical,
            5.0,
            5.0,
            &items,
            3,
        )
        .unwrap();
        assert!((vertical[2].origin[1] - regions[2].origin[0]).abs() < f32::EPSILON);
        assert!(!regions[2].enabled);
        assert_eq!(
            layout_linear([0.0; 2], [20.0; 2], Axis::Horizontal, 5.0, 5.0, &items, 3),
            Err(UiError::InsufficientSpace)
        );
        assert_eq!(
            layout_linear([0.0; 2], [100.0; 2], Axis::Horizontal, 0.0, 0.0, &items, 2),
            Err(UiError::Capacity)
        );
    }
}
