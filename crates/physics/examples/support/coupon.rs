use physics::{
    cohesive,
    plasticity::{Material, mesh::Body},
};
#[derive(Debug)]
pub struct Coupon {
    pub body: Body,
    pub prescribed: Vec<[Option<f64>; 3]>,
    pub end: Vec<usize>,
    pub interface: cohesive::Material,
}
pub fn coupon(interface: cohesive::Material) -> Result<Coupon, &'static str> {
    let mut points = Vec::new();
    let mut prescribed = Vec::new();
    let mut end = Vec::new();
    let id = |side: usize, i: usize, j: usize, k: usize| side * 8 + i * 4 + j * 2 + k;
    for side in 0..2 {
        for i in 0..2 {
            for j in 0..2 {
                for k in 0..2 {
                    points.push([
                        if i == 0 { 0. } else { 0.1 } - if side == 0 { 0.1 } else { 0. },
                        if j == 0 { 0. } else { 0.1 },
                        if k == 0 { 0. } else { 0.1 },
                    ]);
                    let at_end = side == 1 && i == 1;
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
    let bulk = Material::new(1e9, 0., 1e8, 0.)?;
    for side in 0..2 {
        let v = [
            id(side, 0, 0, 0),
            id(side, 1, 0, 0),
            id(side, 0, 1, 0),
            id(side, 1, 1, 0),
            id(side, 0, 0, 1),
            id(side, 1, 0, 1),
            id(side, 0, 1, 1),
            id(side, 1, 1, 1),
        ];
        for t in [
            [0, 1, 3, 7],
            [0, 3, 2, 7],
            [0, 2, 6, 7],
            [0, 6, 4, 7],
            [0, 4, 5, 7],
            [0, 5, 1, 7],
        ] {
            cells.push((t.map(|a| v[a]), bulk));
        }
    }
    let mut body = Body::new(points, cells)?;
    let minus = [
        id(0, 1, 0, 0),
        id(0, 1, 1, 0),
        id(0, 1, 1, 1),
        id(0, 1, 0, 1),
    ];
    let plus = [
        id(1, 0, 0, 0),
        id(1, 0, 1, 0),
        id(1, 0, 1, 1),
        id(1, 0, 0, 1),
    ];
    for t in [[0, 1, 2], [0, 2, 3]] {
        body.add_interface(t.map(|a| minus[a]), t.map(|a| plus[a]), interface)?;
    }
    Ok(Coupon {
        body,
        prescribed,
        end,
        interface,
    })
}
