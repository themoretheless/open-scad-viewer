//! Unified start/step/cancel protocol for long kernel jobs.
//!
//! `start_*` admits a job and returns its handle without doing the heavy
//! work. `step(handle, budget)` polls the job's owned future at most `budget`
//! times; every poll ends at a cooperative checkpoint (one pairwise Boolean,
//! one render batch), so a step is bounded work. It returns progress while
//! pending and the result once complete; completion consumes the handle.
//! `cancel(handle)` trips the job's [`CancellationToken`] and drops all of its
//! scratch. Handles are never recycled, so cancelling a stale handle can never
//! drop a newer job. There are no threads: the host drives every step and
//! regains control between steps, which is what makes cancellation prompt.

use crate::Result;
use nurbs_numerics::progress::CancellationToken;
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use value_codec::{Value, json};

/// Monotonic `done/total` units of one job, shared with its future.
#[derive(Default)]
pub(crate) struct JobProgress {
    done: Cell<u64>,
    total: Cell<u64>,
}

impl JobProgress {
    pub(crate) fn set(&self, done: u64, total: u64) {
        self.total.set(total);
        self.done.set(done);
    }
}

/// Kind tag reported with every step so hosts can assert what they drive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum JobKind {
    MeshBoolean,
    RenderPrepare,
}

impl JobKind {
    fn name(self) -> &'static str {
        match self {
            Self::MeshBoolean => "mesh-boolean",
            Self::RenderPrepare => "render-prepare",
        }
    }
}

type JobFuture = Pin<Box<dyn Future<Output = Result<Value>>>>;

struct Job {
    id: u32,
    kind: JobKind,
    future: JobFuture,
    progress: Rc<JobProgress>,
    token: CancellationToken,
    steps: u64,
}

#[derive(Default)]
struct Jobs {
    next: u32,
    live: Vec<Job>,
}

thread_local! {
    static JOBS: RefCell<Jobs> = RefCell::new(Jobs::default());
}

/// Concurrent job admission. A worker drives one build at a time; the slack
/// covers a superseded job whose cancel has not been delivered yet.
pub(crate) const MAX_KERNEL_JOBS: usize = 8;
/// Upper bound on polls per step, so a hostile budget cannot pin the host.
pub(crate) const MAX_STEP_BUDGET: u32 = 1 << 16;

fn admit(
    kind: JobKind,
    build: impl FnOnce(CancellationToken, Rc<JobProgress>) -> Result<JobFuture>,
) -> Result<u32> {
    let id = JOBS.with(|jobs| {
        let jobs = jobs.borrow();
        if jobs.live.len() >= MAX_KERNEL_JOBS {
            return Err(crate::input("Kernel job budget exceeded"));
        }
        jobs.next
            .checked_add(1)
            .ok_or_else(|| crate::input("Kernel job IDs exhausted"))
    })?;
    let token = CancellationToken::new();
    let progress = Rc::new(JobProgress::default());
    // Build outside the registry borrow: preparing a job reads the solid store.
    let future = build(token.clone(), progress.clone())?;
    JOBS.with(|jobs| {
        let mut jobs = jobs.borrow_mut();
        jobs.next = id;
        jobs.live.push(Job { id, kind, future, progress, token, steps: 0 });
    });
    Ok(id)
}

/// Start a cooperative n-ary mesh Boolean over retained solid handles.
pub(crate) fn start_boolean(ids: &[u32], op: crate::mesh::BooleanJobOp) -> Result<u32> {
    if ids.is_empty() || ids.len() > 20_000 {
        return Err(crate::input("Invalid Boolean operand count"));
    }
    admit(JobKind::MeshBoolean, |token, progress| {
        let future = crate::mesh::boolean_job(ids, op, token, progress)?;
        Ok(Box::pin(future) as JobFuture)
    })
}

/// Start cooperative render preparation (face ids, crease-split normals,
/// merge pairs) of a retained solid. Completion yields an array-result handle
/// with the same layout as `abi_render_mesh`.
pub(crate) fn start_render(id: u32, crease_cosine: f64) -> Result<u32> {
    if !crease_cosine.is_finite() {
        return Err(crate::input("Invalid crease cosine"));
    }
    admit(JobKind::RenderPrepare, |_token, progress| {
        let future = crate::mesh::render_buffers_job(id, crease_cosine)?;
        progress.set(0, 1);
        Ok(Box::pin(async move {
            let mesh = future.await?;
            progress.set(1, 1);
            Ok(json!(crate::mesh_analysis::store(
                crate::mesh_analysis::AnalysisBuffers::Render(mesh)
            )))
        }) as JobFuture)
    })
}

/// Advance a job by at most `budget` cooperative units. Pending:
/// `{state:"pending",kind,done,total,steps}`. Complete:
/// `{state:"complete",kind,result,steps}` and the handle is consumed. A failed
/// job is consumed too and returns its error. Unknown handles refuse.
pub(crate) fn step(id: u32, budget: u32) -> Result<Value> {
    let budget = budget.clamp(1, MAX_STEP_BUDGET);
    // Take the job out so its future may touch other kernel state (and even
    // admit nothing reentrantly) without a live registry borrow.
    let mut job = JOBS.with(|jobs| {
        let mut jobs = jobs.borrow_mut();
        let index = jobs
            .live
            .iter()
            .position(|job| job.id == id)
            .ok_or_else(|| crate::input("Unknown, completed or cancelled kernel job"))?;
        Ok::<_, crate::Error>(jobs.live.remove(index))
    })?;
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    for _ in 0..budget {
        job.steps += 1;
        if let std::task::Poll::Ready(result) = job.future.as_mut().poll(&mut cx) {
            let value = result?;
            return Ok(json!({
                "state": "complete",
                "kind": job.kind.name(),
                "result": value,
                "steps": job.steps,
            }));
        }
    }
    let pending = json!({
        "state": "pending",
        "kind": job.kind.name(),
        "done": job.progress.done.get(),
        "total": job.progress.total.get(),
        "steps": job.steps,
    });
    JOBS.with(|jobs| {
        let mut jobs = jobs.borrow_mut();
        let index = jobs.live.partition_point(|other| other.id < job.id);
        jobs.live.insert(index, job);
    });
    Ok(pending)
}

/// Idempotent: trips the token and drops every unfinished allocation.
pub(crate) fn cancel(id: u32) {
    let dropped = JOBS.with(|jobs| {
        let mut jobs = jobs.borrow_mut();
        jobs.live
            .iter()
            .position(|job| job.id == id)
            .map(|index| jobs.live.remove(index))
    });
    if let Some(job) = dropped {
        job.token.cancel();
        drop(job);
    }
}

/// Live jobs, for leak checks.
#[cfg(test)]
pub(crate) fn live_jobs() -> usize {
    JOBS.with(|jobs| jobs.borrow().live.len())
}

#[cfg(test)]
#[path = "tests/kernel_jobs.rs"]
mod tests;
