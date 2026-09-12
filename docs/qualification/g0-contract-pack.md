# G0 Contract Pack

**Status:** in progress — not closed.  
**Charter:** `docs/design/rust-brep-nurbs-kernel.md` §21 and
`docs/design/brep-nurbs-14-stage-master-plan.md` §G0.  
**Machine status:** [`g0-contract-pack-status-v1.json`](./g0-contract-pack-status-v1.json).

G0 produces contracts, IDL, baselines, and qualification schema only. It must
not change product routing, geometry results, or create a production B-rep
provider. Parallel G1–G3 implementation remains NO-GO until every G0 exit row
below is green **and** organizational closure criteria are met.

ADRs **0005–0010** are human-accepted by `repository-owner` with solo dual-role
reviewer attestation (explicitly **not** organizationally independent).

## Blocking ADR dependency graph

```text
0001 language-compatibility          (accepted)
0002 permanent-geometry-engine-routing (accepted) ──► G0.3 partial
0003 official-openscad-mcp-runtime   (superseded oracle; not a B-rep blocker)
0004 independent-openscad-2021-engine (accepted; language path)
0005 semantic-program-v1             (accepted) ──► G0.4 / G0.5 / G1
0006 tolerance-predicates-evidence   (accepted) ──► G0.7 / G2a
0007 brep-topology-idl               (accepted) ──► G0.6 / G2a
0008 protocol-v6-geometry-scene-v2   (accepted) ──► G0.8 / G4a
0009 resource-security-supervisor    (accepted) ──► G0.12
0010 independent-reimplementation    (accepted) ──► G0.13
```

## Backlog map

| ID | Deliverable | Status |
| --- | --- | --- |
| G0.1 | Review charter + ADR graph | partial (solo dual-role) |
| G0.2 | Toolchain fingerprints + SPDX SBOM | partial (G1 plan hash drift) |
| G0.3 | Engine/language/error precedence | partial (refusal matrix appendix) |
| G0.4 | SemanticProgram IDL | done |
| G0.5 | Hash / identity ADR | done |
| G0.6 | Topology IDL | done |
| G0.7 | Tolerance / predicates / evidence | done |
| G0.8 | Protocol v6 / GeometrySceneV2 IDL | done (activation = G4a) |
| G0.9 | Legacy golden corpus | done |
| G0.10 | Differential comparator | done |
| G0.11 | QualificationPlan schema | done |
| G0.12 | Resource/security/supervisor ADR | done |
| G0.13 | Independent-reimplementation policy | done (license activation pending) |
| G0.14 | G1 qualification plan | partial |

## Definition of Done (unchanged)

G0 closes only when all blocking ADRs are accepted, IDL validators are
independent and green, the legacy corpus/comparator are versioned and mutation-
sensitive, identities are domain-separated, supervisor/package/licensing
policies are measurable, and the G1 plan is frozen **and** approved for
execution — with no unresolved P0/P1 in G0 scope. Solo dual-role acceptance
does not by itself satisfy organizational independence claims.
