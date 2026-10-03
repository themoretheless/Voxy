//! Editor document/graph limits are independent of subsystem owner budgets.
pub(crate) const OBJECTS: usize = 4096;
pub(crate) const DOCUMENT_BYTES: usize = 4 * 1024 * 1024;
pub(crate) const HISTORY_BYTES: usize = 64 * 1024 * 1024;
