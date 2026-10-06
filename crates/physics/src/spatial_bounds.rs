//! Immutable median-split bounds shared by mesh admission and embedding.
pub(crate) type Bounds = (usize, [f64; 3], [f64; 3]);
#[derive(Debug)]
pub(crate) struct BoundsIndex {
    low: [f64; 3],
    high: [f64; 3],
    max_index: usize,
    children: Option<(Box<Self>, Box<Self>)>,
    cells: Vec<Bounds>,
}
impl BoundsIndex {
    fn build(cells: &mut [Bounds]) -> Self {
        let low: [f64; 3] = std::array::from_fn(|axis| {
            cells
                .iter()
                .map(|v| v.1[axis])
                .fold(f64::INFINITY, f64::min)
        });
        let high: [f64; 3] = std::array::from_fn(|axis| {
            cells
                .iter()
                .map(|v| v.2[axis])
                .fold(f64::NEG_INFINITY, f64::max)
        });
        let max_index = cells.iter().map(|v| v.0).max().unwrap_or(0);
        let (children, leaf) = if cells.len() <= 4 {
            (None, cells.to_vec())
        } else {
            let axis = (0..3)
                .max_by(|&a, &b| (high[a] - low[a]).total_cmp(&(high[b] - low[b])))
                .unwrap();
            let mid = cells.len() / 2;
            cells.select_nth_unstable_by(mid, |a, b| {
                (0.5 * a.1[axis] + 0.5 * a.2[axis])
                    .total_cmp(&(0.5 * b.1[axis] + 0.5 * b.2[axis]))
                    .then(a.0.cmp(&b.0))
            });
            let (left, right) = cells.split_at_mut(mid);
            (
                Some((Box::new(Self::build(left)), Box::new(Self::build(right)))),
                vec![],
            )
        };
        Self {
            low,
            high,
            max_index,
            children,
            cells: leaf,
        }
    }
    pub(crate) fn new(mut cells: Vec<Bounds>) -> Self {
        Self::build(&mut cells)
    }
    pub(crate) fn query_point(&self, point: [f64; 3], output: &mut Vec<usize>) {
        self.visit(None, point, point, true, output);
    }
    pub(crate) fn query_overlapping_after(
        &self,
        index: usize,
        low: [f64; 3],
        high: [f64; 3],
        output: &mut Vec<usize>,
    ) {
        self.visit(Some(index), low, high, false, output);
    }
    fn visit(
        &self,
        after: Option<usize>,
        low: [f64; 3],
        high: [f64; 3],
        inclusive: bool,
        output: &mut Vec<usize>,
    ) {
        let overlaps = |a: [f64; 3], b: [f64; 3]| {
            !(0..3).any(|axis| {
                if inclusive {
                    low[axis] > b[axis] || a[axis] > high[axis]
                } else {
                    low[axis] >= b[axis] || a[axis] >= high[axis]
                }
            })
        };
        if after.is_some_and(|index| self.max_index <= index) || !overlaps(self.low, self.high) {
            return;
        }
        if let Some((left, right)) = &self.children {
            left.visit(after, low, high, inclusive, output);
            right.visit(after, low, high, inclusive, output);
        } else {
            for &(index, a, b) in &self.cells {
                if after.is_none_or(|previous| index > previous) && overlaps(a, b) {
                    output.push(index);
                }
            }
        }
    }
}
