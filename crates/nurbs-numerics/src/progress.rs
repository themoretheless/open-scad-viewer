//! Progress reporting, cooperative cancellation and bounded allocation
//! (roadmap N19, checklist items 920 and 921).
//!
//! The contract sits next to the unified [`Budget`](crate::guards::Budget)
//! guard and reuses the same error taxonomy:
//! - [`CancellationToken`] is a cheap-to-clone handle over one atomic flag.
//!   It needs no threads, clock or allocator beyond one `Arc`, so it works on
//!   `wasm32-unknown-unknown`; a host (worker message, UI button) sets it and
//!   long loops poll it through [`Control::step`]. A cancelled loop returns a
//!   typed [`Cancelled`] error (code [`CANCELLED`]) and never a partial result.
//! - [`Progress`] receives [`ProgressUpdate`]s: a static phase name,
//!   `done/total` units of work and an optional message. [`Control`] keeps
//!   `done` monotonic within a phase and reports the final `done == total`.
//! - [`AllocationBudget`] pre-checks element counts against a byte budget
//!   and uses `try_reserve`, so an oversized request becomes a typed
//!   `resource()` error instead of an abort on allocation failure.
//!
//! Existing entry points keep their signatures and run with
//! [`Control::none`]: never cancelled, no reporter, default allocation budget.

use crate::{Error, Result, resource};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// Error code of a cooperative cancellation.
pub const CANCELLED: &str = "NURBS_CANCELLED";

/// Typed payload: the stage that observed the cancellation request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cancelled {
    pub stage: &'static str,
}

impl std::fmt::Display for Cancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} was cancelled", self.stage)
    }
}

impl From<Cancelled> for Error {
    fn from(c: Cancelled) -> Self {
        Error::new(CANCELLED, c.to_string())
    }
}

/// True when `error` came from a cooperative cancellation.
pub fn is_cancelled(error: &Error) -> bool {
    error.code == CANCELLED
}

#[derive(Debug)]
struct Flag {
    cancelled: AtomicBool,
    /// Checks left before the token cancels itself; `usize::MAX` disables it.
    countdown: AtomicUsize,
}

/// Cooperative cancellation handle. Clones share one flag; the default token
/// holds no flag at all and is never cancelled.
#[derive(Clone, Debug, Default)]
pub struct CancellationToken(Option<Arc<Flag>>);

impl CancellationToken {
    /// A live token that a holder of any clone may cancel.
    pub fn new() -> Self {
        Self::with_countdown(usize::MAX)
    }
    /// A token that can never be cancelled (no allocation).
    pub fn never() -> Self {
        Self(None)
    }
    /// A token that cancels itself on its `checks + 1`-th poll. Deterministic
    /// stand-in for "the user pressed cancel mid-loop" in tests and drills.
    pub fn cancel_after(checks: usize) -> Self {
        Self::with_countdown(checks)
    }
    fn with_countdown(countdown: usize) -> Self {
        Self(Some(Arc::new(Flag {
            cancelled: AtomicBool::new(false),
            countdown: AtomicUsize::new(countdown),
        })))
    }
    /// Request cancellation; idempotent. No effect on a [`never`](Self::never) token.
    pub fn cancel(&self) {
        if let Some(flag) = &self.0 {
            flag.cancelled.store(true, Ordering::Release);
        }
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.as_ref().is_some_and(|f| f.cancelled.load(Ordering::Acquire))
    }
    /// Poll the token: `Err(Cancelled)` once cancellation was requested.
    pub fn check(&self, stage: &'static str) -> std::result::Result<(), Cancelled> {
        let Some(flag) = &self.0 else { return Ok(()) };
        let left = flag.countdown.load(Ordering::Relaxed);
        if left != usize::MAX {
            if left == 0 {
                flag.cancelled.store(true, Ordering::Release);
            } else {
                flag.countdown.store(left - 1, Ordering::Relaxed);
            }
        }
        if flag.cancelled.load(Ordering::Acquire) {
            Err(Cancelled { stage })
        } else {
            Ok(())
        }
    }
}

/// One progress sample.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProgressUpdate<'a> {
    pub phase: &'static str,
    pub done: u64,
    pub total: u64,
    pub message: Option<&'a str>,
}

/// Receiver of progress samples. Implementations must be cheap; they run
/// inside the hot loops at the [`Control`] check interval.
pub trait Progress {
    fn report(&mut self, update: ProgressUpdate<'_>);
}

/// Reporter that drops every sample.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoProgress;

impl Progress for NoProgress {
    fn report(&mut self, _: ProgressUpdate<'_>) {}
}

/// Reporter that records owned samples (tests, logs).
#[derive(Clone, Debug, Default)]
pub struct RecordedProgress(pub Vec<(&'static str, u64, u64, Option<String>)>);

impl Progress for RecordedProgress {
    fn report(&mut self, u: ProgressUpdate<'_>) {
        self.0.push((u.phase, u.done, u.total, u.message.map(str::to_owned)));
    }
}

/// Byte budget for the buffers a loop grows. Requests are checked before any
/// allocation is attempted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AllocationBudget {
    pub max_bytes: usize,
}

impl Default for AllocationBudget {
    /// 1 GiB: far above every bounded kernel workload, below the point where
    /// a WASM linear memory (4 GiB) or a desktop process would fail hard.
    fn default() -> Self {
        Self { max_bytes: 1 << 30 }
    }
}

impl AllocationBudget {
    /// Reject `count` elements of `T` above the byte budget (or overflowing).
    pub fn admit<T>(&self, count: usize, stage: &'static str) -> Result<()> {
        match count.checked_mul(std::mem::size_of::<T>().max(1)) {
            Some(bytes) if bytes <= self.max_bytes => Ok(()),
            _ => Err(resource(format!(
                "{stage} would allocate {count} elements of {} bytes, above the {} byte budget",
                std::mem::size_of::<T>(),
                self.max_bytes
            ))),
        }
    }
    /// Budget-checked `Vec::with_capacity` that reports allocator failure as
    /// a typed `resource()` error.
    pub fn vec<T>(&self, count: usize, stage: &'static str) -> Result<Vec<T>> {
        self.admit::<T>(count, stage)?;
        let mut v = Vec::new();
        v.try_reserve_exact(count)
            .map_err(|_| resource(format!("{stage} could not reserve {count} elements")))?;
        Ok(v)
    }
}

/// Per-call control block: token, reporter, check interval, byte budget.
pub struct Control<'a> {
    pub token: CancellationToken,
    pub progress: &'a mut dyn Progress,
    /// Poll the token and report at most every `interval` steps (>= 1).
    pub interval: u64,
    pub memory: AllocationBudget,
    phase: &'static str,
    last_done: u64,
    since_check: u64,
}

impl<'a> Control<'a> {
    pub fn new(token: CancellationToken, progress: &'a mut dyn Progress) -> Self {
        Self {
            token,
            progress,
            interval: 1,
            memory: AllocationBudget::default(),
            phase: "",
            last_done: 0,
            since_check: 0,
        }
    }
    /// Never cancelled, no reporter, default byte budget.
    pub fn none() -> Control<'static> {
        // Leaking a zero-sized value allocates nothing.
        let progress: &'static mut NoProgress = Box::leak(Box::new(NoProgress));
        Control::new(CancellationToken::never(), progress)
    }
    pub fn with_interval(mut self, interval: u64) -> Self {
        self.interval = interval.max(1);
        self
    }
    pub fn with_memory(mut self, memory: AllocationBudget) -> Self {
        self.memory = memory;
        self
    }
    /// Start a phase: polls the token and reports `0/total`.
    pub fn begin(&mut self, phase: &'static str, total: u64) -> Result<()> {
        self.phase = phase;
        self.last_done = 0;
        self.since_check = 0;
        self.token.check(phase)?;
        self.progress.report(ProgressUpdate { phase, done: 0, total, message: None });
        Ok(())
    }
    /// One unit of loop work. Every `interval` steps: poll the token and
    /// report `done/total` (clamped so `done` never decreases or exceeds `total`).
    pub fn step(&mut self, done: u64, total: u64) -> Result<()> {
        self.since_check += 1;
        if self.since_check < self.interval {
            return Ok(());
        }
        self.since_check = 0;
        self.token.check(self.phase)?;
        let done = done.min(total).max(self.last_done);
        if done != self.last_done {
            self.last_done = done;
            self.progress.report(ProgressUpdate { phase: self.phase, done, total, message: None });
        }
        Ok(())
    }
    /// Close the phase: polls the token and reports `total/total`.
    pub fn finish(&mut self, total: u64, message: Option<&str>) -> Result<()> {
        self.token.check(self.phase)?;
        self.last_done = total;
        self.progress.report(ProgressUpdate { phase: self.phase, done: total, total, message });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_clones_share_the_flag_and_never_is_inert() {
        let t = CancellationToken::new();
        let c = t.clone();
        assert!(t.check("s").is_ok());
        c.cancel();
        assert_eq!(t.check("s"), Err(Cancelled { stage: "s" }));
        let n = CancellationToken::never();
        n.cancel();
        assert!(n.check("s").is_ok() && !n.is_cancelled());
        let e: Error = Cancelled { stage: "loop" }.into();
        assert!(is_cancelled(&e) && e.message.contains("loop"));
    }

    #[test]
    fn cancel_after_fires_on_the_requested_poll() {
        let t = CancellationToken::cancel_after(2);
        assert!(t.check("a").is_ok());
        assert!(t.check("a").is_ok());
        assert!(t.check("a").is_err());
        assert!(t.is_cancelled());
    }

    #[test]
    fn control_reports_monotonic_progress_to_total() {
        let mut rec = RecordedProgress::default();
        {
            let mut c = Control::new(CancellationToken::new(), &mut rec).with_interval(2);
            c.begin("p", 10).unwrap();
            for d in [1, 3, 2, 5, 4, 20] {
                c.step(d, 10).unwrap();
            }
            c.finish(10, Some("ok")).unwrap();
        }
        let done: Vec<u64> = rec.0.iter().map(|r| r.1).collect();
        assert!(done.windows(2).all(|w| w[0] <= w[1]), "{done:?}");
        assert_eq!(rec.0.last().unwrap().1, 10);
        assert_eq!(rec.0.last().unwrap().3.as_deref(), Some("ok"));
    }

    #[test]
    fn allocation_budget_refuses_before_allocating() {
        let b = AllocationBudget { max_bytes: 64 };
        assert!(b.vec::<u64>(8, "s").is_ok());
        let e = b.vec::<u64>(9, "stage-x").unwrap_err();
        assert_eq!(e.code, crate::RESOURCE_LIMIT);
        assert!(e.message.contains("stage-x"));
        assert!(AllocationBudget::default().admit::<u64>(usize::MAX, "s").is_err());
    }
}
