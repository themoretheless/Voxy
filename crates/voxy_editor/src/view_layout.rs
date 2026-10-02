//! Physical-pixel view layout shared by rendering and pointer routing.
use glam::Vec2;

pub(super) fn regions(size: [u32; 2], split: bool) -> Vec<[u32; 4]> {
    let [width, height] = size;
    if width == 0 || height == 0 {
        return vec![];
    }
    if split && width >= 2 {
        let left = width / 2;
        vec![[0, 0, left, height], [left, 0, width - left, height]]
    } else {
        vec![[0, 0, width, height]]
    }
}
pub(super) fn hit(regions: &[[u32; 4]], cursor: Vec2) -> Option<(u8, Vec2, Vec2)> {
    if !cursor.is_finite() {
        return None;
    }
    regions
        .iter()
        .enumerate()
        .find_map(|(index, &[x, y, width, height])| {
            let origin = Vec2::new(x as f32, y as f32);
            let size = Vec2::new(width as f32, height as f32);
            let local = cursor - origin;
            (local.min_element() >= 0. && local.x < size.x && local.y < size.y).then_some((
                index as u8,
                local,
                size,
            ))
        })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn odd_width_boundary_and_scaled_pixels_share_one_layout() {
        let rects = regions([1001, 700], true);
        assert_eq!(rects, vec![[0, 0, 500, 700], [500, 0, 501, 700]]);
        assert_eq!(
            hit(&rects, Vec2::new(500., 100.)),
            Some((1, Vec2::new(0., 100.), Vec2::new(501., 700.)))
        );
        assert_eq!(hit(&rects, Vec2::new(499., 100.)).unwrap().0, 0);
        assert!(hit(&rects, Vec2::new(1001., 0.)).is_none());
        assert!(hit(&rects, Vec2::new(f32::NAN, 0.)).is_none());
        let scaled = regions([2002, 1400], true);
        assert_eq!(
            hit(&scaled, Vec2::new(1001., 200.)).unwrap().1,
            Vec2::new(0., 200.)
        );
        assert_eq!(hit(&scaled, Vec2::new(1000., 200.)).unwrap().0, 0);
        assert_eq!(regions([1, 700], true), vec![[0, 0, 1, 700]]);
        assert!(regions([0, 700], true).is_empty());
    }
}
