//! Benchmark-only crossover; one completed render per simulation step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefineCrossover {
    Aa,
    Ab,
    Ba,
    Bb,
}
impl RefineCrossover {
    pub fn parse(value: &str) -> Result<Self, crate::ComputeError> {
        match value {
            "aa" => Ok(Self::Aa),
            "ab" => Ok(Self::Ab),
            "ba" => Ok(Self::Ba),
            "bb" => Ok(Self::Bb),
            _ => Err(crate::ComputeError::Validation(
                "refinement crossover must be aa, ab, ba or bb".into(),
            )),
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Self::Aa => "aa",
            Self::Ab => "ab",
            Self::Ba => "ba",
            Self::Bb => "bb",
        }
    }
    pub fn label_b(self, frame: u64) -> bool {
        if frame == 0 {
            return false;
        }
        if self == Self::Bb {
            return true;
        }
        // ABBA BAAB balances both phases of every-other-frame GPU sampling.
        let b = [false, true, true, false, true, false, false, true][((frame - 1) % 8) as usize];
        b ^ (self == Self::Ba)
    }
    pub fn cache_b(self) -> bool {
        self != Self::Aa
    }
    pub fn cache_for_frame(self, frame: u64) -> bool {
        self.cache_b() && self.label_b(frame)
    }
    pub fn mixed(self) -> bool {
        self != Self::Bb
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_gpu_sampling_phase_is_balanced() {
        for start in 0..8 {
            let labels: Vec<_> = (0..4)
                .map(|i| RefineCrossover::Ab.label_b(181 + start + 2 * i))
                .collect();
            assert_eq!(labels.iter().filter(|x| **x).count(), 2);
        }
    }
    #[test]
    fn complementary_routes_cover_identical_states() {
        for frame in 1..=2580 {
            assert_ne!(
                RefineCrossover::Ab.label_b(frame),
                RefineCrossover::Ba.label_b(frame)
            );
            assert_eq!(
                RefineCrossover::Aa.label_b(frame),
                RefineCrossover::Ab.label_b(frame)
            );
            assert!(!RefineCrossover::Aa.cache_for_frame(frame));
            assert!(RefineCrossover::Bb.cache_for_frame(frame));
        }
    }
    #[test]
    fn probe_bake_never_uses_an_alternate() {
        for mode in [
            RefineCrossover::Aa,
            RefineCrossover::Ab,
            RefineCrossover::Ba,
            RefineCrossover::Bb,
        ] {
            assert!(!mode.label_b(0));
            assert!(!mode.cache_for_frame(0));
        }
    }
    #[test]
    fn only_single_candidate_mode_is_an_absolute_run() {
        assert!(!RefineCrossover::Bb.mixed());
        for mode in [
            RefineCrossover::Aa,
            RefineCrossover::Ab,
            RefineCrossover::Ba,
        ] {
            assert!(mode.mixed());
        }
        assert!(RefineCrossover::parse("auto").is_err());
    }
}
