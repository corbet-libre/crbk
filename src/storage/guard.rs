//! Explicit transient authority at an existing storage transaction boundary.
use crate::Result;
use std::future::Future;

/// The caller's actual retained authority; never a serialized verdict or input.
/// SQL implementations call this after staging all writes under their existing
/// writer. Owner reads must use a second pooled connection and never write or
/// reacquire this writer. Original session/deadline/clock floors must survive.
pub trait CommitGuard: Send {
    /// Recheck current authority through its real owners while the writer is held.
    fn check(&mut self) -> impl Future<Output = Result<()>> + Send;
    /// Synchronous final cut directly before commit and after acknowledgment.
    /// A failure after commit is uncertainty, never evidence of rollback.
    fn release(&mut self) -> Result<()>;
}

// Preserve the original caller-authorized append API. Checked operations never
// select this private adapter or fall back to it on refusal.
pub(super) struct CallerAuthorized;
impl CommitGuard for CallerAuthorized {
    async fn check(&mut self) -> Result<()> {
        Ok(())
    }
    fn release(&mut self) -> Result<()> {
        Ok(())
    }
}
