//! Winner-only temporary object capacity. Integer precision stays 128 radix words.
#[derive(Debug, Clone, Copy)]
pub(super) struct Plan {
    pub slots: u32,
    pub workers: u32,
}
pub(super) fn plan(
    slots: Option<u32>,
    requested: Option<u32>,
    full_workers: u32,
    capacity: u32,
) -> Option<Plan> {
    let slots = slots.unwrap_or(512);
    if slots < 16 || slots > 512 || !slots.is_multiple_of(16) || full_workers == 0 || capacity == 0 {
        return None;
    }
    let available = u64::from(full_workers) * 512 / u64::from(slots);
    let workers = u64::from(requested.unwrap_or(full_workers))
        .min(available)
        .min(u64::from(capacity));
    if workers == 0 {
        return None;
    }
    Some(Plan {
        slots,
        workers: workers as u32,
    })
}
pub(super) fn source(original: &str, plan: Plan) -> String {
    let arena = "const EX_SLOTS:u32=512u;";
    let guard = "if id.x>=arrayLength(&exact_scratch)/EX_ARENA_WORDS {return;}";
    let winner = "fn nearest_winner_process(";
    assert_eq!(original.matches(arena).count(), 1);
    assert_eq!(original.matches(guard).count(), 6);
    assert_eq!(original.matches(winner).count(), 1);
    let source=original.replace(arena,&format!("const EX_SLOTS:u32={}u;",plan.slots))
        .replace(guard,"if id.x>=WORLD_COMPACT_WINNER_WORKERS||id.x>=arrayLength(&exact_scratch)/EX_ARENA_WORDS {return;}")
        .replace(winner,"fn nearest_winner_process_workspace(");
    // The existing full-source pass sees state 4 on the same retained queue.
    // It resets the cursor and uses the unchanged 512-slot module and owner.
    let retry = r"
fn nearest_winner_process(index:u32,stride:u32,worker:u32) {
 nearest_winner_process_workspace(index,stride,worker);
 if ex_peak>=EX_SLOTS||exact_output[index*stride].x!=1u {
  nearest_admission_result(index,stride,4u,0u);
 }
}
";
    format!(
        "{}\nconst WORLD_COMPACT_WINNER_WORKERS:u32={}u;\n{}",
        source, plan.workers, retry
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_same_allocation() {
        let p = plan(Some(128), Some(4096), 1024, 1_658_880).unwrap();
        assert_eq!((p.slots, p.workers), (128, 4096));
        assert_eq!(
            plan(Some(192), Some(4096), 1024, 1_658_880)
                .unwrap()
                .workers,
            2730
        );
    }
    #[test]
    fn respects_explicit_worker_limit_and_partial_capacity() {
        assert_eq!(plan(Some(128), Some(31), 1024, 100).unwrap().workers, 31);
        assert_eq!(plan(Some(128), Some(4096), 1024, 17).unwrap().workers, 17);
        assert_eq!(plan(Some(128), None, 1024, 10000).unwrap().workers, 1024);
    }
    #[test]
    fn rejects_invalid_or_empty_workspaces() {
        for n in [0, 1, 15, 17, 513] {
            assert!(plan(Some(n), Some(1), 1024, 1024).is_none());
        }
        assert!(plan(Some(128), Some(0), 1024, 1024).is_none());
        assert!(plan(None, None, 0, 1).is_none());
    }
}
