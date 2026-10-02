use physics::{
    cohesive,
    plasticity::{Material, mesh::Body},
};
pub fn fixture(
    n: usize,
    interface: cohesive::Material,
) -> (Body, Vec<[Option<f64>; 3]>, Vec<usize>) {
    let width = n + 1;
    let count = width.pow(3);
    let id = |side: usize, i: usize, j: usize, k: usize| side * count + (i * width + j) * width + k;
    let spacing = 0.1 / f64::from(u32::try_from(n).unwrap());
    let mut points = Vec::new();
    let mut prescribed = Vec::new();
    let mut end = Vec::new();
    for side in 0..2 {
        for i in 0..width {
            for j in 0..width {
                for k in 0..width {
                    points.push([
                        f64::from(u32::try_from(i).unwrap()) * spacing
                            - if side == 0 { 0.1 } else { 0. },
                        f64::from(u32::try_from(j).unwrap()) * spacing,
                        f64::from(u32::try_from(k).unwrap()) * spacing,
                    ]);
                    let at_end = side == 1 && i == n;
                    prescribed.push([
                        if (side == 0 && i == 0) || at_end {
                            Some(0.)
                        } else {
                            None
                        },
                        Some(0.),
                        Some(0.),
                    ]);
                    if at_end {
                        end.push(id(side, i, j, k));
                    }
                }
            }
        }
    }
    let mut cells = Vec::new();
    let material = Material::new(1e9, 0., 1e8, 0.).unwrap();
    for side in 0..2 {
        for i in 0..n {
            for j in 0..n {
                for k in 0..n {
                    let v = [
                        id(side, i, j, k),
                        id(side, i + 1, j, k),
                        id(side, i, j + 1, k),
                        id(side, i + 1, j + 1, k),
                        id(side, i, j, k + 1),
                        id(side, i + 1, j, k + 1),
                        id(side, i, j + 1, k + 1),
                        id(side, i + 1, j + 1, k + 1),
                    ];
                    for t in [
                        [0, 1, 3, 7],
                        [0, 3, 2, 7],
                        [0, 2, 6, 7],
                        [0, 6, 4, 7],
                        [0, 4, 5, 7],
                        [0, 5, 1, 7],
                    ] {
                        cells.push((t.map(|a| v[a]), material));
                    }
                }
            }
        }
    }
    let mut body = Body::new(points, cells).unwrap();
    for j in 0..n {
        for k in 0..n {
            let face = |side: usize| {
                let i = if side == 0 { n } else { 0 };
                [
                    id(side, i, j, k),
                    id(side, i, j + 1, k),
                    id(side, i, j + 1, k + 1),
                    id(side, i, j, k + 1),
                ]
            };
            for t in [[0, 1, 2], [0, 2, 3]] {
                body.add_interface(t.map(|a| face(0)[a]), t.map(|a| face(1)[a]), interface)
                    .unwrap();
            }
        }
    }
    (body, prescribed, end)
}
