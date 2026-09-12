# ADR 0009: Resource, security, and supervisor policy (G0.12)

- Status: accepted
- Date: 2026-09-12
- Accepted-by: repository-owner (human authorization in session)
- Contract: `resource-security-supervisor-v1`
- Reference implementation (qualification-only): `src/mcp/manifoldPlanQualificationSupervisor.ts`

## Context

Geometry work runs in browser Workers and MCP child workers. G0 requires a
normative end-to-end memory, hard-kill, and queue policy before claiming
supervisor rows in any QualificationPlan. Existing code is discovery evidence
until this ADR is accepted and re-run under a frozen plan.

## Decision

### Queue

- At most **one active geometry job** per supervisor instance.
- Additional requests fail fast with a typed busy error
  (`E_MCP_MANIFOLD_PLAN_BUSY` or browser equivalent).
- Silent unbounded queues are forbidden.

### Cancellation vs hard-kill

| Mode | Meaning |
| --- | --- |
| Cooperative cancel | Requested; Worker may finish cleanup and emit `cancelled` |
| Hard-kill | Process/Worker termination after grace; no partial success publication |

Hard-kill is **not** proof that OS-level OOM or native crashes are contained in
the parent process failure domain. Qualification claims must not reinterpret
Worker termination as process isolation.

### Deadlines

Every supervised run binds finite:

- startup timeout;
- wall deadline;
- cancellation grace;
- join timeout.

Missing any bound is invalid configuration and must refuse before work.

### Memory admission (end-to-end)

Admission counts peak of:

- WASM committed + scratch + packed output + chunk scratch;
- JS destination / decoded buffers;
- retained previous scene;
- GPU old+new replacement assets simultaneously.

Descriptor lengths validate before JS allocation. A “128 MiB payload” limit is
not a “128 MiB peak” claim.

### Quarantine

Indeterminate ownership after cleanup failure, protocol violation, or
supervisor fault quarantines the instance: future begins refuse until explicit
reset. Quarantine cannot be cleared by retrying the same poisoned session.

### Browser vs MCP

Same policy semantics; different deployment backends. Browser and MCP must not
diverge on busy/cancel/deadline/quarantine meaning. Isolation strength differs
and must be stated in each QualificationPlan environment row.

### Explicit non-goals

- Claiming process-level OOM / hostile sandbox containment from Worker kill.
- Automatic cross-engine fallback on timeout.
- Product behavior change under this ADR alone.

## Acceptance gates

G0.12 completes only when:

1. this ADR is accepted;
2. matrix rows exist for busy, cancel, deadline, hard-kill, quarantine;
3. end-to-end memory formula is cited by package/budget ADRs;
4. organizational reviewer signs the supervisor claim boundary.
