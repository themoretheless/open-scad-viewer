# ADR 0007: B-rep topology ownership, typestate, and lineage (G2a IDL)

- Status: accepted
- Date: 2026-09-12
- Accepted-by: repository-owner (human authorization in session)
- Reviewer: repository-owner (solo dual-role attestation; not organizationally independent)
- Contract: `brep-topology-idl-v1`
- Fixtures: `docs/qualification/topology-idl/`

## Context

G2a requires an approved topology schema before plane/box constructors and
snapshot transactions. Design ownership is already fixed in
`docs/design/rust-brep-nurbs-kernel.md` §7: role lives only on
`SolidShellUse`; reverse incidence is derived; handles are not serialized.

Existing `crates/brep-topology` and `src/core/topologyLineage.ts` are early
carriers. They do **not** close G0.6 until this IDL, fixtures, and
one-invariant mutations are frozen and independently validated.

## Decision

### Authoritative ownership

```text
Solid → ordered SolidShellUse { shell, role: Outer|Cavity }
     → Shell { face_uses }          // Shell has NO canonical role
     → FaceUse { face, sense }
     → Face → Loop → Coedge
```

Reverse maps (`coedge_owner_loop`, `loop_owner_face`, `face_shell_uses`,
`edge_uses`) are snapshot-owned derived indexes, never a second edit surface.

### Profiles for G2a

| Profile | G2a |
| --- | --- |
| `SolidManifold` | yes — closed oriented box |
| `SheetManifold` | fixture-only negative/positive later; not a G2a product claim |
| `Generalized` | diagnostics only; forbidden as Boolean/fillet operand |

### Identity

- Internal `ArenaKey<T>`: slot / generation / record token; never serialized.
- Public durable id: opaque 128-bit `TopoId` (32 lowercase hex).
- Sibling snapshot and stale keys fail closed.
- `programHash`, `TopologySnapshotId`, and `MeshAssetId` stay domain-separated
  per the master-plan preimages; this ADR does not redefine those hashes.

### Typestate

1. `RawOverlay` — transaction scratch, unpublished.
2. `LocallyValidated` — passed `LocalTopologyValidator` only.
3. `GloballyAudited` — requires later intersection/trim gates; **out of G2a**.

G2a may publish locally validated constructor-certified box topology. It must
not claim global solid certification.

### LocalTopologyValidator (G2a minimum)

Must reject or accept solely on:

- handle/generation reachability;
- authoritative ownership and unique owners;
- loop cycle integrity (`next`/`prev` reciprocal);
- derived regular-edge incidence (two opposite uses on closed solid);
- endpoint contract for non-degenerate edges;
- orientability of the outer shell for the box fixture.

Global self-intersection / cavity classification audits are deferred.

### Fixtures (checked in)

| Path | Purpose |
| --- | --- |
| `docs/qualification/topology-idl/brep-topology-idl-v1.schema.json` | Discriminated snapshot schema |
| `docs/qualification/topology-idl/box-solid-v1.json` | Canonical axis-aligned unit box |
| `docs/qualification/topology-idl/mutations-v1.json` | One-invariant negatives |

Independent validator rule: a future test oracle under `tests/support` must
import neither production `brep-topology` mutation APIs nor Worker protocol
codecs when checking these fixtures.

### Explicit non-goals

- Sewing, healing, Boolean, fillet, imprint.
- Protocol v6 scene packets (G0.8).
- Persistent naming across rebuild ambiguity (N1).
- Creating additional `cad-*` crates under this ADR.

## Consequences

- G2a box work has a frozen ownership and validator boundary.
- Competing shell-role models remain forbidden (review finding F01).
- Expanding profiles or adding global audit claims requires IDL version bump.

## Acceptance gates

G0.6 is complete only when:

1. this ADR is **human-accepted** (not merely proposed / engineering-ready);
2. schema + box fixture + mutation corpus + sibling-COW/stale-key fixture
   validate under an independent checker;
3. every mutation in `mutations-v1.json` fails for exactly the declared invariant;
4. no production path serializes raw arena keys or treats `Generalized` as a
   solid Boolean operand.
