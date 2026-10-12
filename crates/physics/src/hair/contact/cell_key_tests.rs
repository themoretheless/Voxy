use super::cell_key;
#[test]
fn signed_cell_coordinates_round_trip_without_aliasing() {
    let values = [i32::MIN, -65_536, -1, 0, 1, 65_536, i32::MAX];
    let mut keys = std::collections::HashSet::new();
    for x in values {
        for y in values {
            for z in values {
                let key = cell_key([x, y, z]);
                assert!(keys.insert(key));
                assert_eq!(
                    [
                        (key.0 >> 32) as u32 as i32,
                        key.0 as u32 as i32,
                        key.1 as i32
                    ],
                    [x, y, z]
                );
            }
        }
    }
}
