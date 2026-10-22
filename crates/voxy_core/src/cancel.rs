use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

#[derive(Debug, Default)]
struct CancelState {
    cancelled: AtomicBool,
    children: Mutex<Vec<Arc<Self>>>,
}

impl CancelState {
    fn cancel(&self) {
        if !self.cancelled.swap(true, Ordering::AcqRel) {
            let children = self
                .children
                .lock()
                .map_or_else(|e| e.into_inner().clone(), |guard| guard.clone());
            for child in children {
                child.cancel();
            }
        }
    }
}

/// Cheap cooperative cancellation shared with finite worker jobs.
#[derive(Clone, Debug, Default)]
pub struct CancelToken(Arc<CancelState>);

impl CancelToken {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.cancel();
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.cancelled.load(Ordering::Acquire)
    }

    /// Creates a child token that is cancelled whenever `self` is cancelled, while
    /// cancelling the child does not cancel `self`.
    #[must_use]
    pub fn child_token(&self) -> Self {
        let child = Self::new();
        if self.is_cancelled() {
            child.cancel();
            return child;
        }
        if let Ok(mut guard) = self.0.children.lock() {
            if self.is_cancelled() {
                child.cancel();
            } else {
                guard.push(Arc::clone(&child.0));
            }
        } else {
            child.cancel();
        }
        child
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancellation_is_shared_across_clones() {
        let first = CancelToken::new();
        let second = first.clone();
        assert!(!second.is_cancelled());
        first.cancel();
        assert!(second.is_cancelled());
    }

    #[test]
    fn child_token_inherits_parent_cancellation_without_cancelling_parent() {
        let parent = CancelToken::new();
        let child_a = parent.child_token();
        let child_b = parent.child_token();
        child_a.cancel();
        assert!(child_a.is_cancelled());
        assert!(!parent.is_cancelled());
        assert!(!child_b.is_cancelled());
        parent.cancel();
        assert!(child_b.is_cancelled());
    }
}
