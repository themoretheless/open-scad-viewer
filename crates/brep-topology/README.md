# brep-topology

Independent Rust boundary topology. Depends only on serde, not on either geometry kernel.

`Model<C,S,P>` stores vertices, edges, oriented edge uses (coedges), ordered loops, faces with outer/hole loops, oriented shell face uses, and bodies with outer/cavity shells. Array indices are model-local IDs, preserved across serialization and tessellation; callers must remap references when deleting or reordering entities. Geometry payload types C/S/P are supplied by adapters: rational curves/surfaces for NURBS; face meshes and unit edge/pcurve slots for polygons.

`validate_topology()` checks reference integrity, ownership, loop closure, shell connectivity, edge incidence/orientation, connected vertex fans and closed body boundaries. Open sheet shells are supported. It does not establish geometric containment, absence of self-intersection, outward orientation in physical space or printability. A topologically valid shell is not proof of a valid geometric solid.

Limits: 4096 entities, 256 faces, 8192 coedges; coordinate/tolerance bounds. Exact NURBS trimming, sewing arbitrary incompatible boundaries, geometric B-rep booleans and STEP are not implemented.

## Application-owned vertex geometry

`Vertex<V = [f64; 3]>` and `Model<C, S, P, V = [f64; 3]>` can retain opaque
constructed geometry without replacing it with rounded display coordinates.
`validate_topology_with_vertices` validates indexed incidence and calls the
owner's vertex admission callback. That callback must enforce the owning
source/context and geometric constraints. It is not a solid certificate.

The default `validate_topology` still checks finite coordinates within 1e6.
Vertex/model codecs accept owner payloads implementing value-codec traits;
the default binary64 representation is unchanged. ConstructedPoint3 has no
automatic serialization or rounded conversion. Owners explicitly export its
binary recipe as a payload and replay it against an admitted source on load.
Decoding alone does not validate geometry or indexed incidence.
A native integration test stores the six retained vertices of an exact triangle
intersection in a real indexed face/loop/shell, validates plane membership,
clones the graph, and checks invalid incidence and owner rejection.
`cad-predicates` is a dev-dependency only; production runtime routing is unchanged.

## Immutable admitted snapshots

`TopologySnapshot` retains a checked indexed model and its vertex admission
policy. Clones share the immutable model. `try_edit` edits a topology copy and
returns a new snapshot only after that same policy and incidence validation
succeed. It does not expose mutable access to the retained model or let an
edit substitute a weaker policy. Edit errors and invalid incidence leave the
original and sibling snapshots intact.

Geometry payloads and admission policies must be immutable; external callback
side effects are not rolled back. This is an in-memory incidence/admission
transaction boundary, not a geometric solid certificate, revision/CAS store,
persistent naming system or production UI transaction integration. Tests use
real exact vertices, verify sibling isolation, retained coordinates after clone
and drop, and reject a foreign-source vertex through the retained policy.
Five topology tests pass in debug/release; module Clippy with `--no-deps` passes.

## Revision-checked publication

`TopologyStore` owns a current admitted snapshot, a non-wrapping revision and
an opaque in-process store identity. A checkout prepares an edit against its
retained snapshot/policy; commit consumes the prepared transaction only if both
store identity and base revision match. Every commit, including a no-op, advances
the revision. Returning to old coordinates does not revive an old transaction.
Foreign/stale transactions and revision exhaustion leave current state unchanged.

This is a single-writer in-memory boundary enforced by exclusive mutable access,
not a distributed or persistent CAS protocol. Tests cover sibling commits, ABA
geometry restoration, foreign stores, rejected preparation and u64 exhaustion.
Six topology tests pass in debug/release; module-only Clippy passes.

## Bounded history

The revision store retains at most 32 historical snapshots across Undo/Redo.
Undo and Redo restore an admitted immutable snapshot and advance revision;
empty history is a no-op. A successful new commit clears Redo and evicts the
oldest undo snapshot when full. Failed commits and revision exhaustion preserve
both geometry and history. Exact vertex recipes remain shared and retained.
This bounds history entries, not arbitrary application-owned geometry memory
or snapshots retained externally.

Seven topology tests pass in debug/release, including 40 commits through the
history bound, full undo, branch/redo invalidation, exhaustion and stale edits
across Undo/Redo. Module-only Clippy passes. This history is not yet wired to
the production UI or persisted to disk.

## Retained whole-model admission

`TopologySnapshot::new_with_model_validation` retains an additional owner
validator for complete geometry. Every edit passes vertex admission, indexed
incidence and this same model validator before a new snapshot is created.
`new` remains the incidence/vertex-only constructor. Model admission does not
upgrade the strength of the owner's geometric evidence.

A brep-core integration test uses its actual Model::validate on a rational
cuboid, rejecting zero curve/surface weights, nonfinite surface coordinates
and detached curve endpoints without changing the revision or original model.
The validator remains attached after commit and Undo/Redo. This test pins
existing topology identity tables; it does not implement topology-changing
identity transactions or a certified solid validator.

## Final publication check

`commit_with_check` allows the caller to check cancellation, deadline or other
publication conditions against the prepared immutable model immediately before
replacement. Store identity, base revision and overflow are checked first. A
failed check leaves model, revision, Undo and Redo untouched. Plain `commit`
uses the same path with no additional caller condition. This is a cooperative
publication check, not interruption of a running geometry calculation.

The real-kernel transaction tests cover cancellation after preparation with
Redo present, rejection of stale edits before invoking the check, and successful
checked publication. Both integration tests pass in debug/release; seven
topology tests pass in debug and module-only Clippy passes.

## Shared revision engine

`RevisionStore<T>` operates on an owner-defined immutable `RevisionState`,
whose checked edit method must preserve admission. Existing TopologyStore,
TopologyCheckout and TopologyTransaction names remain aliases for the indexed
TopologySnapshot specialization. The same engine now also serves complete
brep-core models, avoiding separate histories for geometry and identity tables.
The trait is an owner contract, not an independent geometry certificate.

### Admission continuity during preparation

The shared checkout now checks that the edited state reports the same admission
policy before creating a transaction. A deliberately faulty owner implementation
that substitutes its policy is rejected without changing geometry, revision or
history. This is a consistency guard over the owner contract, not authentication
of arbitrary trait implementations. Nine topology tests pass in debug/release;
five real-kernel transaction tests pass in debug; module-only Clippy passes.
Production integration and full B-rep certification remain open.

### Exact vertex recipes through the indexed topology codec

The shared Vertex/Model codec now supports application-owned vertex payloads
through explicit value-codec trait bounds. The default binary64 wire shape is
unchanged; ConstructedPoint3 still has no implicit rounded serialization.
A native integration test exports all six exact intersection vertices to bounded
binary recipes, serializes the indexed contour as JSON, strictly decodes it,
replays against the admitted source, and revalidates incidence, exact equality
and membership in both source triangles. Curve/surface payloads in this test
are unit placeholders, so this demonstrates retained exact vertices and incidence,
not a certified trimmed surface or solid archive.

Validation: 121 tests across brep-core, brep-topology and value-codec pass in
debug; all nine topology tests pass in release. Module-only topology Clippy
with warnings denied and the native workspace check pass. Project UI/WASM
persistence integration and complete B-rep certification remain open.
