//! Experimental transport boundary. Original physical admission owns publication.
use super::super::rescale;
use super::{
    EXPERIMENTAL_TRIPLE_CONTACT_ARITHMETIC, ExperimentalTripleContactInput, JointDotError,
};
#[derive(Debug)]
struct Layout {
    immutable: usize,
    state: usize,
    reactions: usize,
    status: usize,
    total: usize,
}
impl Layout {
    fn new(rows: usize, width: usize) -> Result<Self, JointDotError> {
        if rows == 0 || width == 0 {
            return Err(JointDotError::Shape);
        }
        if rows > 1024 {
            return Err(JointDotError::Capacity);
        }
        let checked = || {
            let matrix = rows.checked_mul(width)?.checked_mul(3)?;
            let square = rows.checked_mul(rows)?.checked_mul(3)?;
            let vector = width.checked_mul(3)?;
            let row_words = rows.checked_mul(3)?;
            let immutable = 4usize
                .checked_add(matrix)?
                .checked_add(row_words)?
                .checked_add(3)?;
            let state = immutable
                .checked_add(matrix)?
                .checked_add(square)?
                .checked_add(vector)?
                .checked_add(row_words)?;
            let reactions = state.checked_add(vector)?;
            let status = reactions.checked_add(row_words)?;
            let total = status.checked_add(1)?;
            (total <= 16 * 1024 * 1024).then_some(Self {
                immutable,
                state,
                reactions,
                status,
                total,
            })
        };
        checked().ok_or(JointDotError::Capacity)
    }
}
/// One complete original operator. A decoded result remains a candidate only.
#[derive(Debug)]
pub struct ExperimentalTripleActiveContactInput {
    words: Vec<u32>,
    layout: Layout,
    rows: usize,
    width: usize,
    state_scale: f64,
    inverse_column_scale: f64,
}
/// Owned normalized GPU expansion. Its exact value is the sum of all three
/// f32 components multiplied by both positive finite binary scale factors.
/// Consumers must not first collapse it to f64 when precision is required.
#[derive(Debug, Clone, PartialEq)]
pub struct ExperimentalTripleExpansion {
    words: Vec<[u32; 3]>,
    scales: [f64; 2],
}
impl ExperimentalTripleExpansion {
    pub fn normalized_words(&self) -> &[[u32; 3]] {
        &self.words
    }
    pub fn scale_factors(&self) -> [f64; 2] {
        self.scales
    }
    pub fn into_parts(self) -> (Vec<[u32; 3]>, [f64; 2]) {
        (self.words, self.scales)
    }
}
/// Checked transport candidate retaining every GPU component. This does not
/// certify original physical admission or promise a representable f64 preview.
#[derive(Debug, Clone, PartialEq)]
pub struct ExperimentalTripleExpandedCandidate {
    coordinates: ExperimentalTripleExpansion,
    reactions: ExperimentalTripleExpansion,
}
impl ExperimentalTripleExpandedCandidate {
    pub fn coordinates(&self) -> &ExperimentalTripleExpansion {
        &self.coordinates
    }
    pub fn reactions(&self) -> &ExperimentalTripleExpansion {
        &self.reactions
    }
    pub fn into_parts(self) -> (ExperimentalTripleExpansion, ExperimentalTripleExpansion) {
        (self.coordinates, self.reactions)
    }
    /// Variable-size retained data, excluding fixed struct/Vec overhead.
    pub fn retained_bytes(&self) -> usize {
        (self.coordinates.words.len() + self.reactions.words.len()) * 12
    }
}
// Error-free f64 grow expansion of three finite f32 values. Their sum cannot
// overflow or underflow f64; retain cancellation before checking the sign.
fn expansion_sign(words: [u32; 3]) -> Result<std::cmp::Ordering, JointDotError> {
    let mut terms = [0f64; 3];
    let mut count = 0;
    for bits in words {
        let mut value = f32::from_bits(bits) as f64;
        if !value.is_finite() {
            return Err(JointDotError::Output);
        }
        let mut next = 0;
        for index in 0..count {
            let prior = terms[index];
            let sum = value + prior;
            let virtual_prior = sum - value;
            let error = (value - (sum - virtual_prior)) + (prior - virtual_prior);
            if error != 0. {
                terms[next] = error;
                next += 1;
            }
            value = sum;
        }
        if value != 0. || next == 0 {
            terms[next] = value;
            next += 1;
        }
        count = next;
    }
    for &value in terms[..count].iter().rev() {
        if value < 0. {
            return Ok(std::cmp::Ordering::Less);
        }
        if value > 0. {
            return Ok(std::cmp::Ordering::Greater);
        }
    }
    Ok(std::cmp::Ordering::Equal)
}
/// Private checked QR scratch, valid only for the parallel experimental kernel.
#[derive(Debug)]
pub struct ExperimentalTripleActiveFactors {
    width: usize,
    columns: Vec<u32>,
    qr: Vec<u32>,
    selected: usize,
    column_scale_bits: u64,
}
/// Selected QR gather plan derived from a fully checked completed candidate.
/// Keep the producing GPU job exclusively owned and unchanged until decoding.
/// This retains warm-start scratch only; original physical admission is mandatory.
#[derive(Debug)]
pub struct ExperimentalTripleCompletedFactorReadback {
    ranges: Vec<std::ops::Range<u64>>,
    qr_words: usize,
    tail: Vec<u32>,
    width: usize,
    columns: Vec<u32>,
    selected: usize,
    column_scale_bits: u64,
}
impl ExperimentalTripleCompletedFactorReadback {
    pub fn ranges(&self) -> &[std::ops::Range<u64>] {
        &self.ranges
    }
    pub fn factors(self, bytes: &[u8]) -> Result<ExperimentalTripleActiveFactors, JointDotError> {
        if bytes.len() != (self.qr_words + self.tail.len()) * 4 {
            return Err(JointDotError::Output);
        }
        let words = bytes
            .chunks_exact(4)
            .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
            .collect::<Vec<_>>();
        if words[self.qr_words..] != self.tail
            || words[..self.qr_words]
                .iter()
                .any(|v| !f32::from_bits(*v).is_finite())
        {
            return Err(JointDotError::Output);
        }
        Ok(ExperimentalTripleActiveFactors {
            width: self.width,
            columns: self.columns,
            qr: words[..self.qr_words].to_vec(),
            selected: self.selected,
            column_scale_bits: self.column_scale_bits,
        })
    }
}
impl ExperimentalTripleActiveFactors {
    /// Retained payload size, excluding the small fixed struct overhead.
    pub fn retained_bytes(&self) -> usize {
        (self.columns.len() + self.qr.len()) * 4
    }
    /// Exact ordered active-vector identity; inactive rows and bounds do not define QR.
    pub fn same_basis(&self, other: &Self) -> bool {
        self.width == other.width
            && self.selected == other.selected
            && self.column_scale_bits == other.column_scale_bits
            && self.columns == other.columns
    }
}
/// One workgroup per complete original operator in a single dispatch.
/// No prefix result is returned if any candidate or immutable word fails.
#[derive(Debug)]
pub struct ExperimentalTripleActiveContactBatch {
    inputs: Vec<ExperimentalTripleActiveContactInput>,
    words: Vec<u32>,
    offsets: Vec<usize>,
}
impl ExperimentalTripleActiveContactBatch {
    pub fn shader_source() -> String {
        format!(
            "{}\nconst TRIPLE_BATCHED:bool=true;\n{}",
            EXPERIMENTAL_TRIPLE_CONTACT_ARITHMETIC,
            include_str!("joint_contact_triple_parallel.wgsl")
        )
    }
    pub fn new(inputs: Vec<ExperimentalTripleActiveContactInput>) -> Result<Self, JointDotError> {
        if inputs.is_empty() {
            return Err(JointDotError::Shape);
        }
        if inputs.len() > 65535 {
            return Err(JointDotError::Capacity);
        }
        let prefix = inputs.len() + 2;
        let total = inputs
            .iter()
            .try_fold(prefix, |sum, input| {
                sum.checked_add(input.words.len())
                    .filter(|v| *v <= 16 * 1024 * 1024)
            })
            .ok_or(JointDotError::Capacity)?;
        let mut words = Vec::new();
        words
            .try_reserve_exact(total)
            .map_err(|_| JointDotError::Capacity)?;
        words.resize(prefix, 0);
        words[0] = inputs.len() as u32;
        let mut offsets = Vec::with_capacity(inputs.len() + 1);
        for (i, input) in inputs.iter().enumerate() {
            offsets.push(words.len());
            words[i + 1] = words.len() as u32;
            words.extend_from_slice(&input.words);
        }
        offsets.push(words.len());
        words[prefix - 1] = words.len() as u32;
        Ok(Self {
            inputs,
            words,
            offsets,
        })
    }
    pub fn bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.words)
    }
    pub fn workgroups(&self) -> [u32; 3] {
        [self.inputs.len() as u32, 1, 1]
    }
    pub fn operator_offset_bytes(&self, index: usize) -> Option<u64> {
        self.inputs
            .get(index)
            .map(|_| (self.offsets[index] * 4) as u64)
    }
    pub fn inputs(&self) -> &[ExperimentalTripleActiveContactInput] {
        &self.inputs
    }
    /// Immutable table followed by each operator's independently audited ranges.
    pub fn compact_readback_ranges(&self) -> Vec<std::ops::Range<u64>> {
        let mut ranges = Vec::with_capacity(4);
        ranges.push(0..((self.inputs.len() + 2) * 4) as u64);
        for (i, input) in self.inputs.iter().enumerate() {
            let base = (self.offsets[i] * 4) as u64;
            ranges.extend(
                input
                    .compact_readback_ranges()
                    .into_iter()
                    .map(|r| r.start + base..r.end + base),
            );
        }
        ranges
    }
    /// Validate the complete table and length before exposing per-operator bytes.
    /// Each slice still requires its original input's candidate gate.
    pub fn operator_readbacks<'a>(
        &self,
        bytes: &'a [u8],
        compact: bool,
    ) -> Result<Vec<&'a [u8]>, JointDotError> {
        let prefix = (self.inputs.len() + 2) * 4;
        let lengths = self
            .inputs
            .iter()
            .map(|input| {
                if compact {
                    input
                        .compact_readback_ranges()
                        .iter()
                        .map(|r| (r.end - r.start) as usize)
                        .sum()
                } else {
                    input.bytes().len()
                }
            })
            .collect::<Vec<usize>>();
        if bytes.len() != prefix + lengths.iter().sum::<usize>()
            || bytes[..prefix] != self.bytes()[..prefix]
        {
            return Err(JointDotError::Output);
        }
        let mut offset = prefix;
        Ok(lengths
            .into_iter()
            .map(|len| {
                let start = offset;
                offset += len;
                &bytes[start..offset]
            })
            .collect())
    }

    /// Full or compact batch decode is atomic, including exact reaction signs.
    pub fn expanded_candidates(
        &self,
        bytes: &[u8],
        compact: bool,
    ) -> Result<Vec<ExperimentalTripleExpandedCandidate>, JointDotError> {
        self.expanded_candidate_results(bytes, compact)?
            .into_iter()
            .collect()
    }
    /// Validate batch framing first, then each immutable operator independently.
    /// A local rejection never certifies or invalidates a sibling candidate.
    pub fn expanded_candidate_results(
        &self,
        bytes: &[u8],
        compact: bool,
    ) -> Result<Vec<Result<ExperimentalTripleExpandedCandidate, JointDotError>>, JointDotError>
    {
        let slices = self.operator_readbacks(bytes, compact)?;
        Ok(self
            .inputs
            .iter()
            .zip(slices)
            .map(|(input, bytes)| {
                if compact {
                    input.expanded_candidate_from_compact_readback(bytes)
                } else {
                    input.expanded_candidate(bytes)
                }
            })
            .collect())
    }
    pub fn candidate(&self, bytes: &[u8]) -> Result<Vec<(Vec<f64>, Vec<f64>)>, JointDotError> {
        let prefix = (self.inputs.len() + 2) * 4;
        if bytes.len() != self.words.len() * 4 || bytes[..prefix] != self.bytes()[..prefix] {
            return Err(JointDotError::Output);
        }
        self.inputs
            .iter()
            .enumerate()
            .map(|(i, input)| input.candidate(&bytes[4 * self.offsets[i]..4 * self.offsets[i + 1]]))
            .collect()
    }
}
impl ExperimentalTripleActiveContactInput {
    pub fn shader_source() -> String {
        format!(
            "{}\n{}",
            EXPERIMENTAL_TRIPLE_CONTACT_ARITHMETIC,
            include_str!("joint_contact_triple_active.wgsl")
        )
    }
    /// Qualification-only 256-lane alternative using the identical checked payload.
    pub fn parallel_shader_source() -> String {
        format!(
            "{}\nconst TRIPLE_BATCHED:bool=false;\n{}",
            EXPERIMENTAL_TRIPLE_CONTACT_ARITHMETIC,
            include_str!("joint_contact_triple_parallel.wgsl")
        )
    }
    /// Full checked payload size; dimensions only, before coefficient packing.
    pub fn required_bytes(rows: usize, width: usize) -> Result<usize, JointDotError> {
        Ok(Layout::new(rows, width)?.total * 4)
    }
    pub fn new(
        columns: &[Vec<f64>],
        bounds: &[f64],
        tolerance: f64,
    ) -> Result<Self, JointDotError> {
        let rows = columns.len();
        let width = columns.first().map_or(0, Vec::len);
        // Account for input, Q, R and all candidate/live output buffers before packing.
        let layout = Layout::new(rows, width)?;
        let packed = ExperimentalTripleContactInput::new(columns, bounds, tolerance)?;
        let state_scale = packed.bound_scale / packed.column_scale;
        let inverse_column_scale = packed.column_scale.recip();
        if !state_scale.is_finite() || state_scale <= 0. || !inverse_column_scale.is_finite() {
            return Err(JointDotError::NumericRange);
        }
        let mut words = Vec::new();
        words
            .try_reserve_exact(layout.total)
            .map_err(|_| JointDotError::Capacity)?;
        words.extend([rows as u32, width as u32, 512u32.max(rows as u32 + 1), 0]);
        words.extend(packed.columns.iter().flatten().copied());
        words.extend(packed.bounds.iter().flatten().copied());
        words.extend(packed.tolerance);
        debug_assert_eq!(words.len(), layout.immutable);
        words.resize(layout.total, 0);
        words[layout.status] = u32::MAX;
        Ok(Self {
            words,
            layout,
            rows,
            width,
            state_scale,
            inverse_column_scale,
        })
    }
    /// Retain both coefficient components while building the existing checked ABI.
    pub fn new_expanded(
        columns: &[Vec<[f64; 2]>],
        bounds: &[f64],
        tolerance: f64,
    ) -> Result<Self, JointDotError> {
        let packed = ExperimentalTripleContactInput::new_expanded(columns, bounds, tolerance)?;
        let high: Vec<Vec<f64>> = columns
            .iter()
            .map(|c| c.iter().map(|v| v[0]).collect())
            .collect();
        let mut input = Self::new(&high, bounds, tolerance)?;
        input.state_scale = packed.bound_scale / packed.column_scale;
        input.inverse_column_scale = packed.column_scale.recip();
        if !input.state_scale.is_finite()
            || input.state_scale <= 0.
            || !input.inverse_column_scale.is_finite()
        {
            return Err(JointDotError::NumericRange);
        }
        // Coefficient precision can require additional binary scaling.
        let end = 4 + packed.columns.len() * 3;
        for (dst, word) in input.words[4..end]
            .iter_mut()
            .zip(packed.columns.iter().flatten())
        {
            *dst = *word;
        }
        Ok(input)
    }
    /// Experimental active-row hints only; magnitudes never enter physical loads.
    /// A rejected warm candidate requires a separate cold solve by the caller.
    pub fn new_seeded(
        columns: &[Vec<f64>],
        bounds: &[f64],
        tolerance: f64,
        seeds: &[f64],
    ) -> Result<Self, JointDotError> {
        let mut input = Self::new(columns, bounds, tolerance)?;
        if !seeds.is_empty() && seeds.len() != input.rows {
            return Err(JointDotError::Shape);
        }
        if seeds.iter().any(|v| !v.is_finite() || *v < 0.) {
            return Err(JointDotError::NumericRange);
        }
        let offset = input.layout.state - 3 * input.rows;
        for (i, &value) in seeds.iter().enumerate() {
            if value > 0. {
                input.words[offset + 3 * i] = 1f32.to_bits();
            }
        }
        Ok(input)
    }
    /// Retain only a completed one-iteration seeded basis; no physical admission implied.
    pub fn parallel_factors(
        &self,
        bytes: &[u8],
    ) -> Result<ExperimentalTripleActiveFactors, JointDotError> {
        if bytes.len() != self.words.len() * 4 {
            return Err(JointDotError::Output);
        }
        self.parallel_factors_words(|i| {
            u32::from_le_bytes(bytes[4 * i..4 * i + 4].try_into().unwrap())
        })
    }
    fn seeded_rows(&self) -> Vec<usize> {
        let flags = &self.words[self.layout.state - 3 * self.rows..self.layout.state];
        let mut rows = flags
            .chunks_exact(3)
            .enumerate()
            .filter_map(|(row, v)| (v[0] == 1f32.to_bits()).then_some(row))
            .collect::<Vec<_>>();
        rows.sort_by_key(|&row| {
            let rank = f32::from_bits(flags[3 * row + 1]);
            if rank > 0. { rank as usize - 1 } else { row }
        });
        rows
    }
    fn parallel_factors_words(
        &self,
        word: impl Fn(usize) -> u32,
    ) -> Result<ExperimentalTripleActiveFactors, JointDotError> {
        if word(3) != 1 {
            return Err(JointDotError::Output);
        }
        self.candidate_words(&word)?;
        self.factor_rows_words(&word, &self.seeded_rows())
    }
    /// Full-readback snapshot of the final basis after any successful iteration.
    /// Row order is explicit; no order is inferred from sorted positive reactions.
    pub fn completed_parallel_factors(
        &self,
        bytes: &[u8],
    ) -> Result<ExperimentalTripleActiveFactors, JointDotError> {
        if bytes.len() != self.words.len() * 4 {
            return Err(JointDotError::Output);
        }
        let word = |i| u32::from_le_bytes(bytes[4 * i..4 * i + 4].try_into().unwrap());
        let rows = self.completed_rows_words(&word)?;
        self.factor_rows_words(word, &rows)
    }
    fn completed_rows_words(
        &self,
        word: &impl Fn(usize) -> u32,
    ) -> Result<Vec<usize>, JointDotError> {
        let (_, reactions) = self.candidate_words(word)?;
        let metadata = self.layout.state - 3 * self.rows;
        if word(metadata) != 0x56414631 {
            return Err(JointDotError::Output);
        }
        let selected = word(metadata + 1) as usize;
        if selected == 0 || selected > self.rows.min(self.width) {
            return Err(JointDotError::Output);
        }
        let rows = (0..selected)
            .map(|i| word(metadata + 2 + i) as usize)
            .collect::<Vec<_>>();
        let mut seen = vec![false; self.rows];
        for &row in &rows {
            if row >= self.rows || seen[row] {
                return Err(JointDotError::Output);
            }
            seen[row] = true;
        }
        for (row, &present) in seen.iter().enumerate() {
            if reactions[row] > 0. && !present {
                return Err(JointDotError::Output);
            }
        }
        Ok(rows)
    }
    /// Plan a second, selected-only QR gather after compact candidate validation.
    /// It must read the same completed job before that allocation can be reused.
    pub fn completed_factor_readback_plan(
        &self,
        compact: &[u8],
    ) -> Result<ExperimentalTripleCompletedFactorReadback, JointDotError> {
        let map = self.compact_map(compact)?;
        let word = |i| Self::compact_word(compact, &map, i);
        let rows = self.completed_rows_words(&word)?;
        let selected = rows.len();
        let q = self.layout.immutable;
        let r = q + 3 * self.rows * self.width;
        let metadata = self.layout.state - 3 * self.rows;
        let mut ranges = Vec::with_capacity(4);
        ranges.push((q * 4) as u64..((q + 3 * selected * self.width) * 4) as u64);
        for column in 0..selected {
            let start = r + 3 * column * self.rows;
            ranges.push((start * 4) as u64..((start + 3 * selected) * 4) as u64);
        }
        ranges.push((metadata * 4) as u64..((metadata + 2 + selected) * 4) as u64);
        ranges.push(12..16);
        let mut tail = (metadata..metadata + 2 + selected)
            .map(&word)
            .collect::<Vec<_>>();
        tail.push(word(3));
        let columns = rows
            .iter()
            .flat_map(|&row| {
                self.words[4 + 3 * row * self.width..4 + 3 * (row + 1) * self.width]
                    .iter()
                    .copied()
            })
            .collect();
        Ok(ExperimentalTripleCompletedFactorReadback {
            ranges,
            qr_words: 3 * selected * self.width + 3 * selected * selected,
            tail,
            width: self.width,
            columns,
            selected,
            column_scale_bits: self.inverse_column_scale.to_bits(),
        })
    }
    fn factor_rows_words(
        &self,
        word: impl Fn(usize) -> u32,
        rows: &[usize],
    ) -> Result<ExperimentalTripleActiveFactors, JointDotError> {
        let selected = rows.len();
        if selected == 0 {
            return Err(JointDotError::Output);
        }
        // Both private callers have already validated this same immutable
        // candidate snapshot (completed_rows_words or parallel_factors_words).
        let read = |start: usize, end: usize| (start..end).map(&word);
        let q_words = 3 * selected * self.width;
        let r = self.layout.immutable + 3 * self.rows * self.width;
        let mut qr =
            read(self.layout.immutable, self.layout.immutable + q_words).collect::<Vec<_>>();
        for column in 0..selected {
            qr.extend(read(
                r + 3 * column * self.rows,
                r + 3 * column * self.rows + 3 * selected,
            ));
        }
        if qr.iter().any(|v| !f32::from_bits(*v).is_finite()) {
            return Err(JointDotError::Output);
        }
        let columns = rows
            .iter()
            .flat_map(|&row| {
                self.words[4 + 3 * row * self.width..4 + 3 * (row + 1) * self.width]
                    .iter()
                    .copied()
            })
            .collect();
        Ok(ExperimentalTripleActiveFactors {
            width: self.width,
            columns,
            qr,
            selected,
            column_scale_bits: self.inverse_column_scale.to_bits(),
        })
    }
    /// Apply bit-exact matching ordered active vectors; inactive rows/bounds may change.
    pub fn reuse_parallel_factors(
        &mut self,
        factors: &ExperimentalTripleActiveFactors,
    ) -> Result<(), JointDotError> {
        self.apply_parallel_factors(factors, false)
    }
    /// Restore a final basis in its original order using row-rank hints only.
    /// Magnitudes never enter loads. Native physical admission remains mandatory.
    /// Seed a fresh operator solely by exact cached active-vector identity.
    /// Every uploaded component and normalization scale must match. Magnitudes
    /// are never supplied as reactions; the next result still needs physical admission.
    pub fn seed_completed_parallel_factors(
        &mut self,
        factors: &ExperimentalTripleActiveFactors,
    ) -> Result<(), JointDotError> {
        if self.words[3] != 0
            || !self.seeded_rows().is_empty()
            || self.width != factors.width
            || self.inverse_column_scale.to_bits() != factors.column_scale_bits
            || factors.selected == 0
            || factors.selected > self.rows
        {
            return Err(JointDotError::Shape);
        }
        let row_words = 3 * self.width;
        let mut rows = Vec::with_capacity(factors.selected);
        for vector in factors.columns.chunks_exact(row_words) {
            // Identical directions can have different bounds. Seed the strongest
            // duplicate so a weaker equality does not create a dependent append.
            let bounds = 4 + self.rows * row_words;
            let bound = |row: usize| {
                (0..3)
                    .map(|part| f64::from(f32::from_bits(self.words[bounds + 3 * row + part])))
                    .sum::<f64>()
            };
            let row = (0..self.rows)
                .filter(|row| {
                    !rows.contains(row)
                        && &self.words[4 + row * row_words..4 + (row + 1) * row_words] == vector
                })
                .max_by(|&a, &b| bound(a).total_cmp(&bound(b)).then_with(|| b.cmp(&a)))
                .ok_or(JointDotError::Shape)?;
            rows.push(row);
        }
        let flags = self.layout.state - 3 * self.rows;
        for &row in &rows {
            self.words[flags + 3 * row] = 1f32.to_bits();
        }
        let result = self.reuse_completed_parallel_factors(factors);
        if result.is_err() {
            for row in rows {
                self.words[flags + 3 * row..flags + 3 * row + 3].fill(0);
            }
        }
        result
    }
    pub fn reuse_completed_parallel_factors(
        &mut self,
        factors: &ExperimentalTripleActiveFactors,
    ) -> Result<(), JointDotError> {
        self.apply_parallel_factors(factors, true)
    }
    fn apply_parallel_factors(
        &mut self,
        factors: &ExperimentalTripleActiveFactors,
        reorder: bool,
    ) -> Result<(), JointDotError> {
        if self.width != factors.width
            || self.inverse_column_scale.to_bits() != factors.column_scale_bits
        {
            return Err(JointDotError::Shape);
        }
        let selected = self.seeded_rows();
        if selected.len() != factors.selected {
            return Err(JointDotError::Shape);
        }
        let row_words = 3 * self.width;
        let columns = |row: usize| &self.words[4 + row_words * row..4 + row_words * (row + 1)];
        let mut order = Vec::with_capacity(selected.len());
        if reorder {
            for vector in factors.columns.chunks_exact(row_words) {
                let row = selected
                    .iter()
                    .copied()
                    .find(|row| !order.contains(row) && columns(*row) == vector)
                    .ok_or(JointDotError::Shape)?;
                order.push(row);
            }
        } else {
            if !selected
                .iter()
                .flat_map(|&row| columns(row).iter().copied())
                .eq(factors.columns.iter().copied())
            {
                return Err(JointDotError::Shape);
            }
        }
        let q_words = 3 * factors.selected * self.width;
        let r = self.layout.immutable + 3 * self.rows * self.width;
        self.words[self.layout.immutable..r + 3 * self.rows * self.rows].fill(0);
        self.words[self.layout.immutable..self.layout.immutable + q_words]
            .copy_from_slice(&factors.qr[..q_words]);
        for column in 0..factors.selected {
            let start = q_words + 3 * column * factors.selected;
            self.words
                [r + 3 * column * self.rows..r + 3 * column * self.rows + 3 * factors.selected]
                .copy_from_slice(&factors.qr[start..start + 3 * factors.selected]);
        }
        if reorder {
            let flags = self.layout.state - 3 * self.rows;
            for (rank, row) in order.into_iter().enumerate() {
                self.words[flags + 3 * row + 1] = ((rank + 1) as f32).to_bits();
            }
        }
        self.words[3] = factors.selected as u32;
        Ok(())
    }
    /// Every immutable word, live result, final-order metadata and seeded QR prefix; omitted mutable
    /// scratch is never read by either decoder. R ranges use the current stride.
    pub fn compact_readback_ranges(&self) -> Vec<std::ops::Range<u64>> {
        let selected = self.words[self.layout.state - 3 * self.rows..self.layout.state]
            .chunks_exact(3)
            .filter(|v| v[0] == 1f32.to_bits())
            .count();
        let mut ranges = Vec::with_capacity(4);
        ranges.push(0..((self.layout.immutable + 3 * selected * self.width) * 4) as u64);
        let r = self.layout.immutable + 3 * self.rows * self.width;
        for column in 0..selected {
            let range = ((r + 3 * column * self.rows) * 4) as u64
                ..((r + 3 * column * self.rows + 3 * selected) * 4) as u64;
            if ranges.last().unwrap().end == range.start {
                ranges.last_mut().unwrap().end = range.end;
            } else {
                ranges.push(range);
            }
        }
        ranges
            .push(((self.layout.state - 3 * self.rows) * 4) as u64..(self.layout.total * 4) as u64);
        ranges
    }
    fn compact_map(
        &self,
        bytes: &[u8],
    ) -> Result<Vec<(std::ops::Range<usize>, usize)>, JointDotError> {
        let mut offset = 0;
        let ranges = self
            .compact_readback_ranges()
            .into_iter()
            .map(|range| {
                let words = range.start as usize / 4..range.end as usize / 4;
                let start = offset;
                offset += words.len();
                (words, start)
            })
            .collect::<Vec<_>>();
        if bytes.len() != offset * 4 {
            return Err(JointDotError::Output);
        }
        Ok(ranges)
    }
    fn compact_word(bytes: &[u8], map: &[(std::ops::Range<usize>, usize)], index: usize) -> u32 {
        let (range, start) = map
            .iter()
            .find(|(range, _)| range.contains(&index))
            .expect("decoder reads only audited compact ranges");
        let offset = 4 * (start + index - range.start);
        u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
    }
    /// Same transport gate as full readback; no omitted word is reconstructed.
    pub fn candidate_from_compact_readback(
        &self,
        bytes: &[u8],
    ) -> Result<(Vec<f64>, Vec<f64>), JointDotError> {
        let map = self.compact_map(bytes)?;
        self.candidate_words(|i| Self::compact_word(bytes, &map, i))
    }
    pub fn parallel_factors_from_compact_readback(
        &self,
        bytes: &[u8],
    ) -> Result<ExperimentalTripleActiveFactors, JointDotError> {
        let map = self.compact_map(bytes)?;
        self.parallel_factors_words(|i| Self::compact_word(bytes, &map, i))
    }
    pub fn bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.words)
    }
    /// Transport validation only. The original native owner must admit this candidate.
    pub fn candidate(&self, bytes: &[u8]) -> Result<(Vec<f64>, Vec<f64>), JointDotError> {
        if bytes.len() != self.words.len() * 4 {
            return Err(JointDotError::Output);
        }
        self.candidate_words(|i| u32::from_le_bytes(bytes[i * 4..i * 4 + 4].try_into().unwrap()))
    }
    /// Preserve raw expansions without an intermediate f64 conversion.
    pub fn expanded_candidate(
        &self,
        bytes: &[u8],
    ) -> Result<ExperimentalTripleExpandedCandidate, JointDotError> {
        if bytes.len() != self.words.len() * 4 {
            return Err(JointDotError::Output);
        }
        self.expanded_candidate_words(|i| {
            u32::from_le_bytes(bytes[4 * i..4 * i + 4].try_into().unwrap())
        })
    }
    pub fn expanded_candidate_from_compact_readback(
        &self,
        bytes: &[u8],
    ) -> Result<ExperimentalTripleExpandedCandidate, JointDotError> {
        let map = self.compact_map(bytes)?;
        self.expanded_candidate_words(|i| Self::compact_word(bytes, &map, i))
    }
    fn validate_candidate_header(&self, word: &impl Fn(usize) -> u32) -> Result<(), JointDotError> {
        if word(self.layout.status) != 0
            || word(3) == 0
            || word(3) > self.words[2]
            || (0..self.layout.immutable).any(|i| i != 3 && word(i) != self.words[i])
        {
            return Err(JointDotError::Output);
        }
        Ok(())
    }
    fn expanded_candidate_words(
        &self,
        word: impl Fn(usize) -> u32,
    ) -> Result<ExperimentalTripleExpandedCandidate, JointDotError> {
        self.validate_candidate_header(&word)?;
        let read = |offset: usize,
                    count: usize,
                    nonnegative: bool|
         -> Result<Vec<[u32; 3]>, JointDotError> {
            let mut values = Vec::new();
            values
                .try_reserve_exact(count)
                .map_err(|_| JointDotError::Capacity)?;
            for i in 0..count {
                let parts = [
                    word(offset + 3 * i),
                    word(offset + 3 * i + 1),
                    word(offset + 3 * i + 2),
                ];
                let sign = expansion_sign(parts)?;
                if nonnegative && sign == std::cmp::Ordering::Less {
                    return Err(JointDotError::Output);
                }
                values.push(parts);
            }
            Ok(values)
        };
        Ok(ExperimentalTripleExpandedCandidate {
            coordinates: ExperimentalTripleExpansion {
                words: read(self.layout.state, self.width, false)?,
                scales: [self.state_scale, 1.],
            },
            reactions: ExperimentalTripleExpansion {
                words: read(self.layout.reactions, self.rows, true)?,
                scales: [self.state_scale, self.inverse_column_scale],
            },
        })
    }
    fn candidate_words(
        &self,
        word: impl Fn(usize) -> u32,
    ) -> Result<(Vec<f64>, Vec<f64>), JointDotError> {
        self.validate_candidate_header(&word)?;
        let scalar = |offset| {
            let a = f32::from_bits(word(offset)) as f64;
            let b = f32::from_bits(word(offset + 1)) as f64;
            let c = f32::from_bits(word(offset + 2)) as f64;
            if !a.is_finite() || !b.is_finite() || !c.is_finite() {
                return Err(JointDotError::Output);
            }
            let sum = a + b;
            let virtual_b = sum - a;
            Ok(sum + ((a - (sum - virtual_b)) + (b - virtual_b) + c))
        };
        let mut state = Vec::new();
        state
            .try_reserve_exact(self.width)
            .map_err(|_| JointDotError::Capacity)?;
        for i in 0..self.width {
            let normalized = scalar(self.layout.state + 3 * i)?;
            let value = rescale(normalized, self.state_scale, 1.);
            if !value.is_finite() || (normalized != 0. && value == 0.) {
                return Err(JointDotError::Output);
            }
            state.push(value);
        }
        let mut reactions = Vec::new();
        reactions
            .try_reserve_exact(self.rows)
            .map_err(|_| JointDotError::Capacity)?;
        for i in 0..self.rows {
            let normalized = scalar(self.layout.reactions + 3 * i)?;
            let value = rescale(normalized, self.state_scale, self.inverse_column_scale);
            if !value.is_finite() || value < 0. || (normalized != 0. && value == 0.) {
                return Err(JointDotError::Output);
            }
            reactions.push(value);
        }
        Ok((state, reactions))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn triple_expanded_candidate_keeps_terms_below_f64_rounding() {
        let input = ExperimentalTripleActiveContactInput::new(&[vec![1.]], &[1.], 1e-14).unwrap();
        let mut output = input.words.clone();
        output[3] = 1;
        output[input.layout.status] = 0;
        let coordinate = [
            1f32.to_bits(),
            2f32.powi(-54).to_bits(),
            2f32.powi(-100).to_bits(),
        ];
        let reaction = [
            1f32.to_bits(),
            2f32.powi(-55).to_bits(),
            2f32.powi(-110).to_bits(),
        ];
        output[input.layout.state..input.layout.state + 3].copy_from_slice(&coordinate);
        output[input.layout.reactions..input.layout.reactions + 3].copy_from_slice(&reaction);
        let full = bytemuck::cast_slice::<u32, u8>(&output);
        assert_eq!(input.candidate(full).unwrap(), (vec![1.], vec![1.]));
        let expanded = input.expanded_candidate(full).unwrap();
        assert_eq!(expanded.coordinates().normalized_words(), &[coordinate]);
        assert_eq!(expanded.reactions().normalized_words(), &[reaction]);
        assert_eq!(expanded.coordinates().scale_factors(), [1., 1.]);
        assert_eq!(expanded.reactions().scale_factors(), [1., 1.]);
        assert_eq!(expanded.retained_bytes(), 24);
        let compact = input
            .compact_readback_ranges()
            .iter()
            .flat_map(|r| full[r.start as usize..r.end as usize].iter().copied())
            .collect::<Vec<_>>();
        assert_eq!(
            input
                .expanded_candidate_from_compact_readback(&compact)
                .unwrap(),
            expanded
        );
        let mut bad = output.clone();
        bad[4] ^= 1;
        assert!(
            input
                .expanded_candidate(bytemuck::cast_slice(&bad))
                .is_err()
        );
        let mut bad = output.clone();
        bad[input.layout.state] = f32::NAN.to_bits();
        assert!(
            input
                .expanded_candidate(bytemuck::cast_slice(&bad))
                .is_err()
        );
        drop(input);
        let (coordinates, reactions) = expanded.into_parts();
        assert_eq!(coordinates.into_parts(), (vec![coordinate], [1., 1.]));
        assert_eq!(reactions.normalized_words(), &[reaction]);
    }
    #[test]
    fn triple_expanded_candidate_sign_survives_wide_cancellation() {
        let maximum = f32::MAX.to_bits();
        let minimum = f32::from_bits(1).to_bits();
        let negative = (-f32::MAX).to_bits();
        assert_eq!(
            expansion_sign([maximum, minimum, negative]).unwrap(),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            expansion_sign([maximum, (-f32::from_bits(1)).to_bits(), negative]).unwrap(),
            std::cmp::Ordering::Less
        );
        let input = ExperimentalTripleActiveContactInput::new(&[vec![1.]], &[1.], 1e-14).unwrap();
        let mut output = input.words.clone();
        output[3] = 1;
        output[input.layout.status] = 0;
        output[input.layout.reactions..input.layout.reactions + 3].copy_from_slice(&[
            maximum,
            (-f32::from_bits(1)).to_bits(),
            negative,
        ]);
        assert!(
            input
                .expanded_candidate(bytemuck::cast_slice(&output))
                .is_err()
        );
    }
    #[test]
    fn triple_expanded_candidate_does_not_require_representable_f64_projection() {
        let scale = 2f64.powi(1023);
        let input =
            ExperimentalTripleActiveContactInput::new(&[vec![1.]], &[scale], scale).unwrap();
        let mut output = input.words.clone();
        output[3] = 1;
        output[input.layout.status] = 0;
        output[input.layout.state] = 2f32.to_bits();
        let full = bytemuck::cast_slice::<u32, u8>(&output);
        assert!(input.candidate(full).is_err());
        let expanded = input.expanded_candidate(full).unwrap();
        assert_eq!(
            expanded.coordinates().normalized_words(),
            &[[2f32.to_bits(), 0, 0]]
        );
        assert_eq!(expanded.coordinates().scale_factors(), [scale, 1.]);
    }
    #[test]
    #[ignore = "requires VOXY_TRIPLE_DECODE_AUDIT_FIXTURES with eight opaque GPU captures"]
    fn triple_expanded_candidate_replays_opaque_gpu_captures() {
        let directory = std::path::PathBuf::from(
            std::env::var_os("VOXY_TRIPLE_DECODE_AUDIT_FIXTURES").unwrap(),
        );
        let mut files = std::fs::read_dir(&directory)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("json"))
            .collect::<Vec<_>>();
        files.sort();
        assert_eq!(files.len(), 8);
        let mut rounded_gate_failures = 0;
        for path in files {
            let m: serde_json::Value =
                serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
            assert_eq!(m["schema"], "voxy-triple-opaque-decode-audit-v1");
            let rows = m["rows"].as_u64().unwrap() as usize;
            let width = m["coordinates"].as_u64().unwrap() as usize;
            let original =
                std::fs::read(directory.join(m["original_columns_file"].as_str().unwrap()))
                    .unwrap();
            let scalars = original
                .chunks_exact(8)
                .map(|v| f64::from_bits(u64::from_le_bytes(v.try_into().unwrap())))
                .collect::<Vec<_>>();
            assert_eq!(scalars.len(), rows * width);
            let columns = scalars
                .chunks_exact(width)
                .map(|v| v.to_vec())
                .collect::<Vec<_>>();
            let bounds = m["effective_bounds_bits"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| f64::from_bits(v.as_u64().unwrap()))
                .collect::<Vec<_>>();
            let input = ExperimentalTripleActiveContactInput::new(
                &columns,
                &bounds,
                f64::from_bits(m["tolerance_bits"].as_u64().unwrap()),
            )
            .unwrap();
            assert_eq!(
                input.bytes(),
                std::fs::read(directory.join(m["input_file"].as_str().unwrap())).unwrap()
            );
            let compact =
                std::fs::read(directory.join(m["output_file"].as_str().unwrap())).unwrap();
            let expected = input.compact_readback_ranges();
            let ranges = m["readback_ranges_bytes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r[0].as_u64().unwrap()..r[1].as_u64().unwrap())
                .collect::<Vec<_>>();
            assert_eq!(ranges, expected);
            // Pad omitted mutable QR scratch only. Every word read by this decoder
            // comes from the actual captured gather, not the padding.
            let mut full = input.bytes().to_vec();
            let mut offset = 0;
            for range in ranges {
                let count = (range.end - range.start) as usize;
                full[range.start as usize..range.end as usize]
                    .copy_from_slice(&compact[offset..offset + count]);
                offset += count;
            }
            assert_eq!(offset, compact.len());
            let expanded = input
                .expanded_candidate_from_compact_readback(&compact)
                .unwrap();
            assert_eq!(input.expanded_candidate(&full).unwrap(), expanded);
            let raw = |base: usize, count: usize| {
                (0..count)
                    .map(|i| {
                        std::array::from_fn(|p| {
                            u32::from_le_bytes(
                                full[4 * (base + 3 * i + p)..4 * (base + 3 * i + p + 1)]
                                    .try_into()
                                    .unwrap(),
                            )
                        })
                    })
                    .collect::<Vec<[u32; 3]>>()
            };
            assert_eq!(
                expanded.coordinates().normalized_words(),
                raw(input.layout.state, width)
            );
            assert_eq!(
                expanded.reactions().normalized_words(),
                raw(input.layout.reactions, rows)
            );
            assert_eq!(
                expanded.coordinates().scale_factors(),
                [input.state_scale, 1.]
            );
            assert_eq!(
                expanded.reactions().scale_factors(),
                [input.state_scale, input.inverse_column_scale]
            );
            let (rounded, reactions) = input.candidate_from_compact_readback(&compact).unwrap();
            assert_eq!(
                rounded.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                m["decoded_coordinates_bits"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_u64().unwrap())
                    .collect::<Vec<_>>()
            );
            let scales = expanded.coordinates().scale_factors();
            let components = expanded
                .coordinates()
                .normalized_words()
                .iter()
                .flat_map(|parts| {
                    parts
                        .iter()
                        .map(|v| (f32::from_bits(*v) as f64) * scales[0] * scales[1])
                })
                .collect::<Vec<_>>();
            let mut rounded_failed = false;
            for row in 0..rows {
                let repeated = columns[row]
                    .iter()
                    .flat_map(|v| [*v; 3])
                    .collect::<Vec<_>>();
                let exact_components_gap =
                    super::super::qr_tests::compensated_gap(&repeated, &components, bounds[row]);
                let rounded_gap =
                    super::super::qr_tests::compensated_gap(&columns[row], &rounded, bounds[row]);
                let tol = f64::from_bits(m["tolerance_bits"].as_u64().unwrap());
                assert!(
                    if reactions[row] > 0. {
                        exact_components_gap.abs() <= tol
                    } else {
                        exact_components_gap >= -tol
                    },
                    "retained components failed row={row}"
                );
                rounded_failed |= if reactions[row] > 0. {
                    rounded_gap.abs() > tol
                } else {
                    rounded_gap < -tol
                };
            }
            rounded_gate_failures += usize::from(rounded_failed);
        }
        assert_eq!(
            rounded_gate_failures, 4,
            "known f64-only boundary regression was not exercised"
        );
        eprintln!(
            "TRIPLE EXPANDED REPLAY snapshots=8 expanded_coordinate_gate_passes=8 rounded_coordinate_gate_failures=4 all_coordinate_and_reaction_bits_preserved=true scales_preserved=true full_and_compact_equal=true scope=transport_not_original_physical_admission"
        );
    }
    #[test]
    fn triple_batch_checks_metadata_and_withholds_valid_prefix_on_late_failure() {
        let inputs = (1..=2)
            .map(|n| {
                ExperimentalTripleActiveContactInput::new(&[vec![1.]], &[n as f64], 1e-14).unwrap()
            })
            .collect();
        let batch = ExperimentalTripleActiveContactBatch::new(inputs).unwrap();
        let mut output = batch.words.clone();
        for (i, input) in batch.inputs.iter().enumerate() {
            let base = batch.offsets[i];
            output[base + 3] = 1;
            output[base + input.layout.status] = 0;
            let value = (i + 1) as f64;
            output[base + input.layout.state] = ((value / input.state_scale) as f32).to_bits();
            output[base + input.layout.reactions] =
                ((value / (input.state_scale * input.inverse_column_scale)) as f32).to_bits();
        }
        assert_eq!(
            batch.candidate(bytemuck::cast_slice(&output)).unwrap(),
            vec![(vec![1.], vec![1.]), (vec![2.], vec![2.])]
        );
        let full = bytemuck::cast_slice::<u32, u8>(&output);
        let compact = batch
            .compact_readback_ranges()
            .iter()
            .flat_map(|range| {
                full[range.start as usize..range.end as usize]
                    .iter()
                    .copied()
            })
            .collect::<Vec<_>>();
        for (bytes, is_compact) in [(full, false), (compact.as_slice(), true)] {
            let slices = batch.operator_readbacks(bytes, is_compact).unwrap();
            let decoded = batch
                .inputs
                .iter()
                .zip(slices)
                .map(|(input, bytes)| {
                    if is_compact {
                        input.candidate_from_compact_readback(bytes)
                    } else {
                        input.candidate(bytes)
                    }
                })
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert_eq!(decoded, batch.candidate(full).unwrap());
            let expanded = batch.expanded_candidates(bytes, is_compact).unwrap();
            assert_eq!(expanded.len(), 2);
            assert_eq!(
                expanded[0].coordinates().normalized_words(),
                &[[1f32.to_bits(), 0, 0]]
            );
            assert_eq!(expanded[1].coordinates().scale_factors(), [2., 1.]);
            assert!(
                batch
                    .operator_readbacks(&bytes[..bytes.len() - 4], is_compact)
                    .is_err()
            );
            let mut extra = bytes.to_vec();
            extra.extend([0; 4]);
            assert!(batch.operator_readbacks(&extra, is_compact).is_err());
            for index in 0..batch.inputs.len() + 2 {
                let mut bad = bytes.to_vec();
                bad[index * 4] ^= 1;
                assert!(batch.operator_readbacks(&bad, is_compact).is_err());
            }
        }
        for index in 0..batch.inputs.len() + 2 {
            let mut bad = output.clone();
            bad[index] ^= 1;
            assert!(batch.candidate(bytemuck::cast_slice(&bad)).is_err());
        }
        output[batch.offsets[1] + batch.inputs[1].layout.status] = 1;
        assert!(batch.candidate(bytemuck::cast_slice(&output)).is_err());
        assert!(
            batch
                .expanded_candidates(bytemuck::cast_slice(&output), false)
                .is_err()
        );
        let partial = batch
            .expanded_candidate_results(bytemuck::cast_slice(&output), false)
            .unwrap();
        assert!(partial[0].is_ok());
        assert!(partial[1].is_err());
        let compact = batch
            .compact_readback_ranges()
            .iter()
            .flat_map(|range| {
                bytemuck::cast_slice::<u32, u8>(&output)[range.start as usize..range.end as usize]
                    .iter()
                    .copied()
            })
            .collect::<Vec<_>>();
        let partial = batch.expanded_candidate_results(&compact, true).unwrap();
        assert!(partial[0].is_ok());
        assert!(partial[1].is_err());
        output[batch.offsets[1] + batch.inputs[1].layout.status] = 0;
        output[batch.offsets[0] + 4] ^= 1;
        let partial = batch
            .expanded_candidate_results(bytemuck::cast_slice(&output), false)
            .unwrap();
        assert!(partial[0].is_err(), "mutated operator input was admitted");
        assert!(partial[1].is_ok());
        output[0] ^= 1;
        assert!(
            batch
                .expanded_candidate_results(bytemuck::cast_slice(&output), false)
                .is_err(),
            "mutated batch framing was admitted"
        );
        assert!(ExperimentalTripleActiveContactBatch::new(vec![]).is_err());
    }
    #[test]
    fn triple_completed_basis_requires_checked_unique_final_order_and_atomic_match() {
        let columns = vec![vec![1., 0.], vec![0., 1.]];
        let input = ExperimentalTripleActiveContactInput::new(&columns, &[1., 2.], 1e-14).unwrap();
        let mut output = input.words.clone();
        output[3] = 3;
        output[input.layout.status] = 0;
        let metadata = input.layout.state - 3 * input.rows;
        output[metadata] = 0x56414631;
        output[metadata + 1] = 2;
        output[metadata + 2] = 1;
        output[metadata + 3] = 0;
        output[input.layout.reactions] = 1f32.to_bits();
        output[input.layout.reactions + 3] = 1f32.to_bits();
        let factors = input
            .completed_parallel_factors(bytemuck::cast_slice(&output))
            .unwrap();
        let full = bytemuck::cast_slice::<u32, u8>(&output);
        let compact = input
            .compact_readback_ranges()
            .iter()
            .flat_map(|range| {
                full[range.start as usize..range.end as usize]
                    .iter()
                    .copied()
            })
            .collect::<Vec<_>>();
        let plan = input.completed_factor_readback_plan(&compact).unwrap();
        let gathered = plan
            .ranges()
            .iter()
            .flat_map(|range| {
                full[range.start as usize..range.end as usize]
                    .iter()
                    .copied()
            })
            .collect::<Vec<_>>();
        let qr_words = plan.qr_words;
        let staged = plan.factors(&gathered).unwrap();
        assert!(staged.same_basis(&factors));
        assert_eq!(staged.qr, factors.qr);
        for index in qr_words..gathered.len() / 4 {
            let mut bad = gathered.clone();
            bad[index * 4] ^= 1;
            assert!(
                input
                    .completed_factor_readback_plan(&compact)
                    .unwrap()
                    .factors(&bad)
                    .is_err()
            );
        }
        let mut bad = gathered.clone();
        bad[..4].copy_from_slice(&f32::NAN.to_bits().to_le_bytes());
        assert!(
            input
                .completed_factor_readback_plan(&compact)
                .unwrap()
                .factors(&bad)
                .is_err()
        );
        assert!(
            input
                .completed_factor_readback_plan(&compact)
                .unwrap()
                .factors(&gathered[..gathered.len() - 4])
                .is_err()
        );
        let mut extra = gathered.clone();
        extra.extend([0; 4]);
        assert!(
            input
                .completed_factor_readback_plan(&compact)
                .unwrap()
                .factors(&extra)
                .is_err()
        );

        let mut warm =
            ExperimentalTripleActiveContactInput::new_seeded(&columns, &[2., 3.], 1e-14, &[1., 1.])
                .unwrap();
        let before = warm.bytes().to_vec();
        assert!(warm.reuse_parallel_factors(&factors).is_err());
        assert_eq!(warm.bytes(), before);
        warm.reuse_completed_parallel_factors(&factors).unwrap();
        assert_eq!(warm.seeded_rows(), [1, 0]);
        assert_eq!(warm.words[3], 2);
        for (offset, value) in [
            (metadata, 0),
            (metadata + 1, 0),
            (metadata + 1, 3),
            (metadata + 2, 2),
            (metadata + 3, 1),
        ] {
            let mut bad = output.clone();
            bad[offset] = value;
            assert!(
                input
                    .completed_parallel_factors(bytemuck::cast_slice(&bad))
                    .is_err()
            );
        }
        let mut bad = output.clone();
        bad[metadata + 1] = 1;
        assert!(
            input
                .completed_parallel_factors(bytemuck::cast_slice(&bad))
                .is_err(),
            "missing positive reaction row accepted"
        );
        let mut changed = columns.clone();
        changed[0][0] = f64::from_bits(1f64.to_bits() + 1);
        let mut mismatch =
            ExperimentalTripleActiveContactInput::new_seeded(&changed, &[2., 3.], 1e-14, &[1., 1.])
                .unwrap();
        let before = mismatch.bytes().to_vec();
        assert!(mismatch.reuse_completed_parallel_factors(&factors).is_err());
        assert_eq!(mismatch.bytes(), before);
    }
    #[test]
    fn triple_active_layout_checks_total_workspace_before_allocation() {
        assert!(Layout::new(91, 882).is_ok());
        assert!(Layout::new(1024, 1024).is_ok());
        assert!(matches!(Layout::new(1025, 1), Err(JointDotError::Capacity)));
        assert!(matches!(
            Layout::new(1024, 10000),
            Err(JointDotError::Capacity)
        ));
        assert!(matches!(
            Layout::new(2, usize::MAX),
            Err(JointDotError::Capacity)
        ));
    }
    #[test]
    fn triple_active_seed_hints_preserve_original_operator_and_validate_shape() {
        let columns = vec![vec![1., 0.], vec![0., 1.]];
        let bounds = [1., 2.];
        let cold = ExperimentalTripleActiveContactInput::new(&columns, &bounds, 1e-14).unwrap();
        let warm = ExperimentalTripleActiveContactInput::new_seeded(
            &columns,
            &bounds,
            1e-14,
            &[0., f64::MAX],
        )
        .unwrap();
        assert_eq!(
            &cold.words[..cold.layout.immutable],
            &warm.words[..warm.layout.immutable]
        );
        assert_eq!(cold.bytes().len(), warm.bytes().len());
        let offset = warm.layout.state - 3 * warm.rows;
        assert_eq!(warm.words[offset], 0);
        assert_eq!(warm.words[offset + 3], 1f32.to_bits());
        for seeds in [&[1.][..], &[1., f64::NAN], &[-1., 1.]] {
            assert!(
                ExperimentalTripleActiveContactInput::new_seeded(&columns, &bounds, 1e-14, seeds)
                    .is_err()
            );
        }
        assert_eq!(
            ExperimentalTripleActiveContactInput::new_seeded(&columns, &bounds, 1e-14, &[])
                .unwrap()
                .bytes(),
            cold.bytes()
        );
    }
    #[test]
    fn triple_active_factor_reuse_requires_exact_columns_and_flags() {
        let columns = vec![vec![1., 0.], vec![0., 1.]];
        let input =
            ExperimentalTripleActiveContactInput::new_seeded(&columns, &[1., 2.], 1e-14, &[1., 1.])
                .unwrap();
        let mut output = input.words.clone();
        output[3] = 1;
        output[input.layout.status] = 0;
        let factors = input
            .parallel_factors(bytemuck::cast_slice(&output))
            .unwrap();
        let mut changed =
            ExperimentalTripleActiveContactInput::new_seeded(&columns, &[2., 3.], 1e-13, &[1., 1.])
                .unwrap();
        changed.reuse_parallel_factors(&factors).unwrap();
        assert_eq!(changed.words[3], 2);
        for bad_columns in [
            vec![vec![f64::from_bits(1f64.to_bits() + 1), 0.], vec![0., 1.]],
            vec![vec![1., -0.], vec![0., 1.]],
            vec![vec![2., 0.], vec![0., 2.]],
        ] {
            let mut bad = ExperimentalTripleActiveContactInput::new_seeded(
                &bad_columns,
                &[1., 2.],
                1e-14,
                &[1., 1.],
            )
            .unwrap();
            assert!(bad.reuse_parallel_factors(&factors).is_err());
        }
        let mut bad =
            ExperimentalTripleActiveContactInput::new_seeded(&columns, &[1., 2.], 1e-14, &[1., 0.])
                .unwrap();
        assert!(bad.reuse_parallel_factors(&factors).is_err());
    }
    #[test]
    fn triple_active_factor_snapshot_retains_only_selected_qr_and_restores_stride() {
        let columns = vec![vec![1., 0.], vec![0., 1.], vec![1., 1.]];
        let input = ExperimentalTripleActiveContactInput::new_seeded(
            &columns,
            &[1., 2., 3.],
            1e-14,
            &[1., 0., 1.],
        )
        .unwrap();
        let q = input.layout.immutable;
        let r = q + 3 * input.rows * input.width;
        let mut output = input.words.clone();
        output[3] = 1;
        output[input.layout.status] = 0;
        for (i, word) in output[q..r + 3 * input.rows * input.rows]
            .iter_mut()
            .enumerate()
        {
            *word = ((i + 1) as f32).to_bits();
        }
        let factors = input
            .parallel_factors(bytemuck::cast_slice(&output))
            .unwrap();
        assert_eq!(factors.qr.len(), 3 * 2 * 2 + 3 * 2 * 2);
        assert!(
            factors.retained_bytes()
                < (input.rows * input.width * 6 + input.rows * input.rows * 3 + input.rows * 3) * 4
        );
        let mut restored = ExperimentalTripleActiveContactInput::new_seeded(
            &columns,
            &[2., 3., 4.],
            1e-14,
            &[1., 0., 1.],
        )
        .unwrap();
        restored.reuse_parallel_factors(&factors).unwrap();
        assert_eq!(&restored.words[q..q + 12], &output[q..q + 12]);
        assert!(restored.words[q + 12..r].iter().all(|v| *v == 0));
        for column in 0..2 {
            assert_eq!(
                &restored.words[r + 9 * column..r + 9 * column + 6],
                &output[r + 9 * column..r + 9 * column + 6]
            );
            assert!(
                restored.words[r + 9 * column + 6..r + 9 * column + 9]
                    .iter()
                    .all(|v| *v == 0)
            );
        }
        let same = restored
            .parallel_factors(bytemuck::cast_slice(&output))
            .err();
        assert!(
            same.is_some(),
            "mutated immutable bounds must reject output"
        );
        let duplicate = input
            .parallel_factors(bytemuck::cast_slice(&output))
            .unwrap();
        assert!(factors.same_basis(&duplicate));
    }
    #[test]
    fn triple_active_basis_key_ignores_only_inactive_rows_and_rejects_order_changes() {
        let columns = vec![vec![1., 0.], vec![0., 1.], vec![0.25, 0.25]];
        let input = ExperimentalTripleActiveContactInput::new_seeded(
            &columns,
            &[1., 2., 0.],
            1e-14,
            &[1., 1., 0.],
        )
        .unwrap();
        let mut output = input.words.clone();
        output[3] = 1;
        output[input.layout.status] = 0;
        let factors = input
            .parallel_factors(bytemuck::cast_slice(&output))
            .unwrap();
        assert_eq!(factors.columns.len(), 2 * 2 * 3);
        let mut extended = columns.clone();
        extended[2] = vec![0.5, -0.5];
        extended.push(vec![0., 0.]);
        let mut changed = ExperimentalTripleActiveContactInput::new_seeded(
            &extended,
            &[1., 2., 0., -1.],
            1e-14,
            &[1., 1., 0., 0.],
        )
        .unwrap();
        changed.reuse_parallel_factors(&factors).unwrap();
        extended.swap(0, 1);
        let mut reordered = ExperimentalTripleActiveContactInput::new_seeded(
            &extended,
            &[2., 1., 0., -1.],
            1e-14,
            &[1., 1., 0., 0.],
        )
        .unwrap();
        assert!(reordered.reuse_parallel_factors(&factors).is_err());
    }
    #[test]
    fn triple_compact_readback_matches_full_and_rejects_missing_or_mutated_original_words() {
        let input = ExperimentalTripleActiveContactInput::new_seeded(
            &[vec![1., 0.], vec![0., 1.], vec![0., 0.]],
            &[1., 2., -1.],
            1e-14,
            &[1., 1., 0.],
        )
        .unwrap();
        let mut output = input.words.clone();
        output[3] = 1;
        output[input.layout.status] = 0;
        output[input.layout.state] = 1f32.to_bits();
        output[input.layout.state + 3] = 2f32.to_bits();
        let full = bytemuck::cast_slice::<u32, u8>(&output);
        let compact = input
            .compact_readback_ranges()
            .iter()
            .flat_map(|range| {
                full[range.start as usize..range.end as usize]
                    .iter()
                    .copied()
            })
            .collect::<Vec<_>>();
        assert!(compact.len() < full.len());
        assert_eq!(
            input.candidate_from_compact_readback(&compact).unwrap(),
            input.candidate(full).unwrap()
        );
        let a = input.parallel_factors(full).unwrap();
        let b = input
            .parallel_factors_from_compact_readback(&compact)
            .unwrap();
        assert!(a.same_basis(&b));
        assert_eq!(a.qr, b.qr);
        for index in [
            0,
            4 * 4,
            4 * (input.layout.immutable - 1),
            compact.len() - 4,
        ] {
            let mut bad = compact.clone();
            bad[index] ^= 1;
            assert!(input.candidate_from_compact_readback(&bad).is_err());
        }
        assert!(
            input
                .candidate_from_compact_readback(&compact[..compact.len() - 4])
                .is_err()
        );
        let mut extra = compact.clone();
        extra.extend([0; 4]);
        assert!(input.candidate_from_compact_readback(&extra).is_err());
    }
    #[test]
    fn triple_active_decoder_rejects_partial_mutated_and_nonfinite_results() {
        let input = ExperimentalTripleActiveContactInput::new(&[vec![1.]], &[1.], 1e-14).unwrap();
        assert!(input.candidate(input.bytes()).is_err());
        let mut valid = input.words.clone();
        valid[3] = 2;
        valid[input.layout.status] = 0;
        valid[input.layout.state] = 1f32.to_bits();
        valid[input.layout.reactions] = 1f32.to_bits();
        assert_eq!(
            input.candidate(bytemuck::cast_slice(&valid)).unwrap(),
            (vec![1.], vec![1.])
        );
        for index in [0, 1, 2, 4, input.layout.immutable - 1] {
            let mut bad = valid.clone();
            bad[index] ^= 1;
            assert!(input.candidate(bytemuck::cast_slice(&bad)).is_err());
        }
        for status in [1, 2, 8, 13, 14, u32::MAX] {
            let mut bad = valid.clone();
            bad[input.layout.status] = status;
            assert!(input.candidate(bytemuck::cast_slice(&bad)).is_err());
        }
        for iterations in [0, input.words[2] + 1] {
            let mut bad = valid.clone();
            bad[3] = iterations;
            assert!(input.candidate(bytemuck::cast_slice(&bad)).is_err());
        }
        for index in [
            input.layout.state,
            input.layout.state + 1,
            input.layout.state + 2,
            input.layout.reactions,
        ] {
            let mut bad = valid.clone();
            bad[index] = f32::NAN.to_bits();
            assert!(input.candidate(bytemuck::cast_slice(&bad)).is_err());
        }
        let mut bad = valid.clone();
        bad[input.layout.reactions] = (-1f32).to_bits();
        assert!(input.candidate(bytemuck::cast_slice(&bad)).is_err());
        assert!(
            input
                .candidate(&input.bytes()[..input.bytes().len() - 4])
                .is_err()
        );
    }
}

#[cfg(test)]
mod expanded_factor_identity_tests {
    use super::*;
    #[test]
    fn fresh_factor_seed_distinguishes_coefficients_below_an_f64_ulp() {
        let low = 2f64.powi(-60);
        let source =
            ExperimentalTripleActiveContactInput::new_expanded(&[vec![[1., low]]], &[1.], 1e-14)
                .unwrap();
        let factors = ExperimentalTripleActiveFactors {
            width: 1,
            columns: source.words[4..7].to_vec(),
            qr: vec![
                1f32.to_bits(),
                0,
                0,
                1f32.to_bits(),
                (low as f32).to_bits(),
                0,
            ],
            selected: 1,
            column_scale_bits: source.inverse_column_scale.to_bits(),
        };
        let mut changed = ExperimentalTripleActiveContactInput::new_expanded(
            &[vec![[1., low * 0.5]]],
            &[1.],
            1e-14,
        )
        .unwrap();
        let before = changed.words.clone();
        assert!(changed.seed_completed_parallel_factors(&factors).is_err());
        assert_eq!(changed.words, before);
        let mut same =
            ExperimentalTripleActiveContactInput::new_expanded(&[vec![[1., low]]], &[2.], 1e-14)
                .unwrap();
        same.seed_completed_parallel_factors(&factors).unwrap();
        assert_eq!(same.words[3], 1);
        let mut duplicate = ExperimentalTripleActiveContactInput::new_expanded(
            &[vec![[1., low]], vec![[1., low]]],
            &[0., 1.],
            1e-14,
        )
        .unwrap();
        duplicate.seed_completed_parallel_factors(&factors).unwrap();
        assert_eq!(duplicate.seeded_rows(), vec![1]);
    }
}
