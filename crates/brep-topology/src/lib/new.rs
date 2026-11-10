//! lib operations: new.
use super::*;
impl<T: RevisionState> RevisionStore<T> {
pub fn new(snapshot: T) -> Self {
        Self {
            current: snapshot,
            undo: Vec::new(),
            redo: Vec::new(),
            identity: std::sync::Arc::new(()),
            revision: 0,
        }
    }
/// Restore already admitted states into a fresh store identity/revision.
    /// Histories are ordered oldest-to-newest, with the next restoration last.
    pub fn from_history(current: T, undo: Vec<T>, redo: Vec<T>) -> Result<Self> {
        if undo.len().saturating_add(redo.len()) > MAX_TOPOLOGY_HISTORY {
            return Err(Error::new(
                "BREP_RESOURCE_LIMIT",
                "History exceeds 32 snapshots",
            ));
        }
        if undo
            .iter()
            .chain(&redo)
            .any(|snapshot| !current.same_admission_policy(snapshot))
        {
            return Err(Error::new(
                "BREP_HISTORY_POLICY_MISMATCH",
                "History snapshots use different admission policies",
            ));
        }
        let mut store = Self::new(current);
        store.undo = undo;
        store.redo = redo;
        Ok(store)
    }
pub fn history_snapshots(&self) -> (&[T], &[T]) {
        (&self.undo, &self.redo)
    }
pub fn revision(&self) -> u64 {
        self.revision
    }
pub fn snapshot(&self) -> &T {
        &self.current
    }
pub fn history_lengths(&self) -> (usize, usize) {
        (self.undo.len(), self.redo.len())
    }
/// Restoring geometry still advances revision; pending checkouts stay stale.
    pub fn undo(&mut self) -> Result<bool> {
        self.restore_history(false)
    }
pub fn redo(&mut self) -> Result<bool> {
        self.restore_history(true)
    }
pub(super) fn restore_history(&mut self, redo: bool) -> Result<bool> {
        let (source, target) = if redo {
            (&mut self.redo, &mut self.undo)
        } else {
            (&mut self.undo, &mut self.redo)
        };
        if source.is_empty() {
            return Ok(false);
        }
        let next = self.revision.checked_add(1).ok_or_else(|| {
            Error::new("BREP_REVISION_EXHAUSTED", "Topology revision cannot wrap")
        })?;
        let restored = source.pop().expect("History was checked nonempty");
        target.push(std::mem::replace(&mut self.current, restored));
        self.revision = next;
        Ok(true)
    }
pub fn checkout(&self) -> RevisionCheckout<T> {
        RevisionCheckout {
            snapshot: self.current.clone(),
            identity: self.identity.clone(),
            revision: self.revision,
        }
    }
/// Consumes an already validated edit. Stale/foreign/overflow failures leave
    /// current state unchanged. Every commit advances revision, even a no-op.
    pub fn commit(&mut self, transaction: RevisionTransaction<T>) -> Result<u64> {
        self.commit_with_check(transaction, |_| Ok(()))
    }
/// Run a final caller check (e.g. cancellation/deadline) after identity and
    /// revision validation, immediately before replacing state. Failure leaves
    /// the current snapshot, revision and both history stacks unchanged.
    pub fn commit_with_check(
        &mut self,
        transaction: RevisionTransaction<T>,
        check: impl FnOnce(&T::Model) -> Result<()>,
    ) -> Result<u64> {
        if !std::sync::Arc::ptr_eq(&self.identity, &transaction.identity) {
            return Err(Error::new(
                "BREP_FOREIGN_TRANSACTION",
                "Transaction belongs to another topology store",
            ));
        }
        if self.revision != transaction.base_revision {
            return Err(Error::new(
                "BREP_STALE_TRANSACTION",
                "Topology changed after checkout",
            ));
        }
        let next = self.revision.checked_add(1).ok_or_else(|| {
            Error::new("BREP_REVISION_EXHAUSTED", "Topology revision cannot wrap")
        })?;
        check(transaction.candidate.model())?;
        let previous = std::mem::replace(&mut self.current, transaction.candidate);
        if self.undo.len() == MAX_TOPOLOGY_HISTORY {
            self.undo.remove(0);
        }
        self.undo.push(previous);
        self.redo.clear();
        self.revision = next;
        Ok(next)
    }
}
