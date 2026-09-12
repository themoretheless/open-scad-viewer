# ADR 0009: Resource, security, and supervisor policy

- Status: accepted
- Date: 2026-09-12
- Accepted-by: repository-owner (human authorization in session)
- Reviewer: repository-owner (solo dual-role attestation; not organizationally independent)
- Contract: `resource-security-supervisor-v1`
- Machine policy: `docs/qualification/resource-security-supervisor-v1.json`

## Context

Design §§16.4 / 19 require measurable HardLimits / RequestedLimits /
EffectiveLimits, cancel/watchdog, panic containment, and a future MCP
supervisor that keeps stdio admission in the main process while geometry runs
in isolated workers/subprocesses. Qualification supervisors already exist as
test harnesses; they do **not** close G0.12 without this ADR.

## Decision

### Limit lattice

```text
HardLimits (host-trusted)
RequestedLimits (document/request; may only shrink)
EffectiveLimits = fieldwise min(Hard, Requested)
```

`EffectiveLimits` enter execution cache keys and reports. Payloads cannot raise
a hard cap.

Required dimensions (machine inventory):

- input / decoded bytes and compression ratio
- arena entities, topology incidence, history size
- BVH / candidate / intersection queue sizes
- subdivision depth, solver iterations
- generated faces / edges / triangles
- diagnostics / witness bytes
- scratch / committed / WASM high-water memory
- queue depth and in-flight jobs per engine class

### Cancel and hard-kill

| Signal | Effect |
| --- | --- |
| cooperative cancel | observed at `JobMachine::step` boundaries; terminal `Cancelled` |
| watchdog deadline | terminal `DeadlineExceeded`; no late success publication |
| hard-kill / trap | replace Worker/subprocess; Coordinator synthesizes `KernelFault` |
| superseded | terminal cancel reason `superseded`; latest-only publication |

A hard-killed child never mutates App state after replacement begins. Same-
engine retry may reuse fingerprint/policy; cross-engine recovery is forbidden
(ADR 0002).

### Supervisor deployment (target)

Main process owns: stdio protocol, admission, DuckDB, quota tables.
Geometry child owns: evaluation only. Child has no network, no arbitrary
filesystem, no secrets. N-API requires a separate isolation review before use.

Queue policy: bounded per engine class; overload returns typed
`ResourceLimit` without starting work. Fairness is FIFO within a document
revision; cross-document starvation is mitigated by per-session caps in the
machine policy.

### Diagnostics hygiene

Release diagnostics must not include source text, control nets, backtraces, or
internal filesystem paths.

### Non-goals

- Shipping the MCP supervisor under this ADR
- Changing current in-process Manifold MCP behavior
- Performance budget numbers as product promises (remain G0 hypotheses)

## Consequences

- G0.12 becomes reviewable without pretending harness supervisors are the ADR
- G3/G7 supervisor rollout cites this policy verbatim

## Acceptance gates

1. this ADR is human-accepted;
2. `resource-security-supervisor-v1.json` is immutable except via version bump;
3. cancel / deadline / hard-kill / superseded rows exist as fail-closed fixtures
   in that JSON;
4. no production path maps hard-kill into a success terminal.
