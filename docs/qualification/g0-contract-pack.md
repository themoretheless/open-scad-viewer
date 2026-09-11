# G0 Contract Pack

**Status:** in progress — not closed.  
**Charter:** `docs/design/rust-brep-nurbs-kernel.md` §21 and
`docs/design/brep-nurbs-14-stage-master-plan.md` §G0.  
**Machine status:** [`g0-contract-pack-status-v1.json`](./g0-contract-pack-status-v1.json).

G0 produces contracts, IDL, baselines, and qualification schema only. It must
not change product routing, geometry results, or create a production B-rep
provider. Parallel G1–G3 implementation remains NO-GO until every G0 exit row
below is green and blocking ADRs are **accepted**.

## Blocking ADR dependency graph

Edges mean “must be accepted before the dependent gate may start code.”

```text
0001 language-compatibility          (accepted)
0002 permanent-geometry-engine-routing (accepted) ──► G0.3 partial
0003 official-openscad-mcp-runtime   (superseded oracle; not a B-rep blocker)
0004 independent-openscad-2021-engine (accepted; language path)
0005 semantic-program-v1             (proposed) ──► G0.4 / G0.5 / G1
0006 tolerance-predicates-evidence   (proposed) ──► G0.7 / G2a math
0007 brep-topology-idl               (proposed) ──► G0.6 / G2a
(pending) protocol-v6-geometry-scene ──► G0.8 / G4a
(pending) resource-security-supervisor ──► G0.12
(pending) independent-reimplementation-policy ──► G0.13
```

## Backlog map

| ID | Deliverable | Status artifact |
| --- | --- | --- |
| G0.1 | Review charter + ADR graph | this document + status JSON |
| G0.2 | Toolchain fingerprints | `g0-toolchain-fingerprints-v1.json` |
| G0.3 | Engine/language/error precedence | ADR 0001 + 0002 + routing contract |
| G0.4 | SemanticProgram IDL | ADR 0005 (proposed) |
| G0.5 | Hash / identity ADR | embedded in ADR 0005 until split/accepted |
| G0.6 | Topology IDL | ADR 0007 + `topology-idl/` fixtures (proposed) |
| G0.7 | Tolerance / predicates / evidence | ADR 0006 + `g2a-predicate-inventory-v1.json` |
| G0.8 | Protocol v6 / GeometrySceneV2 | missing |
| G0.9 | Legacy golden corpus | partial (`manifold-plan-oracle-v1.json`) |
| G0.10 | Differential comparator | partial (inside G1 plan + tests) |
| G0.11 | QualificationPlan schema | `qualification-plan.schema.json` |
| G0.12 | Resource/security/supervisor ADR | missing |
| G0.13 | Independent-reimplementation policy | design prose only |
| G0.14 | G1 qualification plan | `semantic-manifold-g1-plan-v16.json` (not execution-approved) |

## Definition of Done (unchanged)

G0 closes only when all blocking ADRs are accepted, IDL validators are
independent and green, the legacy corpus/comparator are versioned and mutation-
sensitive, identities are domain-separated, supervisor/package/licensing
policies are measurable, and the G1 plan is frozen **and** approved for
execution — with no unresolved P0/P1 in G0 scope.
