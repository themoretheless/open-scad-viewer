# G0 Contract Pack

**Status:** engineering rows largely done — gate **not closed** (G0.14).  
**Charter:** `docs/design/rust-brep-nurbs-kernel.md` §21 and
`docs/design/brep-nurbs-14-stage-master-plan.md` §G0.  
**Machine status:** [`g0-contract-pack-status-v1.json`](./g0-contract-pack-status-v1.json).

Acceptance mode: **solo dual-role** (`organizationalIndependence: false`).
Active G1 plan: latest `semantic-manifold-g1-plan-v1*.json` with
`executionAdmission: ready-clean-rerun` still needs **u07 clean post-freeze evidence**.

## Blocking ADR dependency graph

```text
0001–0002, 0004–0010  (accepted)
0003                  (superseded oracle)
```

## Backlog map

| ID | Status | Notes |
| --- | --- | --- |
| G0.1–G0.13 | done | contracts, oracles, protocol-v6, supervisor, provenance |
| G0.14 | **partial** | clean post-freeze execution evidence (u07) still open |

## Definition of Done

G0 closes when G1 plan clean runs publish append-only results with no
blocking unresolvedRows, under the frozen comparator/oracle. Solo dual-role
acceptance does not claim organizational independence.
