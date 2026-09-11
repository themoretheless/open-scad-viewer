# ADR 0006: Tolerance, predicates, constructions, and evidence (G2a inventory)

- Status: proposed; blocking G0.7 / G2a
- Date: 2026-09-12
- Contract: `tolerance-predicates-evidence-v1`
- Inventory: `docs/qualification/g2a-predicate-inventory-v1.json`

## Context

Exact B-rep Boolean, fillet, sewing, and certified solids are meaningless
without a numeric contract that separates:

1. exact-sign predicates over authentic input leaves;
2. certified constructions that may not be re-ingested as exact leaves;
3. tolerance-aware model classification that can return `Indeterminate`.

Design prose already exists in `docs/design/rust-brep-nurbs-kernel.md` §5 and
the master-plan mathematical foundation stage. G0.7 must freeze a **finite**
inventory for the first walking slice (G2a: plane / axis-aligned box) before
any production predicate crate is introduced.

## Decision

### Scope

This ADR freezes the G2a numeric contract only. Later gates (G2b+, G5a–e)
extend the inventory with a new versioned JSON artifact and an ADR amendment;
they do not silently widen v1.

Authoritative machine inventory:

[`docs/qualification/g2a-predicate-inventory-v1.json`](../qualification/g2a-predicate-inventory-v1.json)

### Exact input leaves

An `ExactInputLeaf` is either:

- an uninterpreted IEEE-754 binary64 bit pattern from authored geometry or a
  previously certified boundary that preserved bits; or
- a canonical finite rational constant with explicit numerator/denominator.

Division results, normalized vector components, trig/libm outputs, Newton
iterates, and rounded construction centers are **never** new exact leaves.

### Decision pipeline

Every G2a predicate evaluates in order:

1. fast binary64 filter;
2. outward rounding interval filter;
3. exact expansion / adaptive exact-sign arithmetic.

A filter may only accelerate a conclusive sign. It may not invent a sign when
the exact stage is inconclusive.

### G2a predicates

Only these predicates are admitted in inventory v1:

| ID | Role |
| --- | --- |
| `orient2d` | planar loop / triangle orientation |
| `orient3d` | plane-vs-point side / tetrahedral sign for box solids |
| `compare_squared_distance` | optional residual checks; never a substitute for orient |

Outputs are `Negative | Zero | Positive | Indeterminate`. `Zero` means exact
sign zero under the leaf algebra, not “close enough.”

`inCircle`, `inSphere`, general CC/CS/SS certificates, and NURBS root isolation
are out of scope for this ADR.

### ToleranceContext

One immutable context enters every operation and cache key. Required fields:

`linear_abs`, `linear_rel`, `on_tol`, `clear_tol`, `angular`, `param_floor`,
`ulp_guard`, `max_entity_error`, `policy`.

Validation: `0 < on_tol < clear_tol`, and derived local bounds must respect
`max_entity_error`. Invalid profiles fail closed before geometry work.

Numeric error budget is separate from user acceptance tolerance. There is no
global `EPSILON`, no silent clamp/weld, and no automatic entity-tolerance
inflation along operation chains.

### Model classifier

With proven residual/enclosure bounds:

- below `on_tol` → `Coincident`;
- above `clear_tol` → `Separate`;
- gray band, exhausted precision, or missing proof → `Indeterminate`.

`Indeterminate` must not be coerced to `false`, `WithinTolerance` success, or
a topology mutation. UI snapping cannot alter kernel topology.

### Certified constructions

```text
CertifiedConstruction { enclosure, recipe, residual, uniqueness, provenance }
```

A subsequent predicate must evaluate against `recipe` / `enclosure` or return
`Indeterminate`. Publishing a rounded center as a new exact point is forbidden.

### Evidence ledger

Geometry records carry a typed `EvidenceLedger`. Applicable fields for G2a:

- positional enclosure or residual;
- directional or two-sided Hausdorff bound when claimed;
- parameter correspondence when claimed;
- derivative/normal bound when claimed;
- topology-preservation evidence when claimed.

Unused fields stay absent. Bounds do not shrink under composition. Merge is
allowed only with compatible evidence and proven topology invariants.

### Shewchuk / robust-arithmetic policy

Selected mode for adaptive exact-sign work: **`original_from_paper`**.

`derived_port` and `vendored` require a new ADR, exact upstream artifact hash,
notice, and approved `LicenseRef`, and must not be labeled independent
reimplementation.

### Oracle isolation

Predicate qualification requires an independent BigRational / interval oracle
under `tests/support` that imports no production predicate module. Visual mesh
similarity is not an oracle.

### Failure taxonomy (numeric)

At minimum the numeric layer must be able to surface:

`InvalidInput`, `UnsupportedCapability`, `UnsupportedDegeneracy`,
`PrecisionExhausted`, `NoConvergence`, `ResourceLimit`, `Cancelled`,
`DeadlineExceeded`, `KernelFault`.

### Implementation timing

No production `cad-predicates` (or equivalent) crate is created under this ADR.
Implementation begins only after G0 closes and G1 differential admission is
green, as required by the master plan. This ADR is contract freeze, not code
authorization for G2a.

## Consequences

- G2a math work has a finite, reviewable inventory and kill criteria.
- G5 Boolean / fillet / STEP remain blocked on later gates; they cannot claim
  certification while predicate status is `Indeterminate` or `not_certified`.
- Expanding predicates requires inventory v2+ and ADR amendment before code.

## Acceptance gates

G0.7 is complete only when:

1. this ADR is accepted (not merely proposed);
2. `g2a-predicate-inventory-v1.json` is immutable except via version bump;
3. an independent oracle harness exists and mutation-tests the three admitted
   predicates plus classifier gray-band behavior;
4. no production path maps `Indeterminate` to success or false.
