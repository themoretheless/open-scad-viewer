# ADR 0003: SemanticProgram v1 identity, encoding, and execution boundary

- Status: proposed; blocking G0 contract review
- Date: 2026-08-01
- Contract: `semantic-program-contract-v1`
- Core identity: `semantic-program-core-v1`
- Capability graph: `semantic-capabilities-v1`
- Binary formats: `SPC1`, `SPE1`, `TSP1`

## Decision and scope

`SemanticProgram` is the sole executable, kernel-neutral meaning passed from
the language frontend to a geometry backend. A backend receives validated
typed nodes and their already validated inputs. It never receives source text,
does not bind language arguments, does not expand modules or loops, and does
not select an engine. Browser Worker and Node/MCP use the same lowering
implementation and the same canonical program bytes.

Four layers remain independent:

1. `SemanticProgramCoreV1` describes model meaning and is the only input to
   `programHash`.
2. `SemanticProgramEnvelopeV1` binds that core to exact source bytes, source
   provenance, display diagnostics, and authored tessellation intent.
3. execution controls describe cancellation, deadlines, work and memory
   limits; they are not model value identity.
4. publication controls describe current/stale selection, preview promotion,
   export admission, cache state, and queue state; they are not model value
   identity.

This ADR defines the candidate G0 IDL and identity boundary. It becomes frozen
only after the acceptance gates below pass and the status is changed to
accepted. It does not switch the production Manifold path, deploy a B-rep
provider, change Worker protocol v5, or claim Manifold differential
qualification.

## SPC1: canonical semantic core

`SPC1` encodes exactly one validated `SemanticProgramCoreV1`. Every core
object and union variant has an exact key set. Missing keys, extra keys,
`undefined`, sparse arrays, symbol keys, unknown discriminants, unknown
required features, and future schema versions fail closed.

The fixed v1 core contains:

- schema `1.2`, required feature IDs, and identity version;
- language contract, semantics revision, and capability-graph version;
- millimetre length units, degree angles, a right-handed Z-up frame, and the
  matrix convention defined below;
- ordered static operations and their structural identity evidence;
- ordered dynamic occurrence/output records;
- a typed, reachable, acyclic node DAG;
- exactly one discriminated `empty`, `single`, or `multi` result;
- authored capability declarations and their exact inferred closure;
- ordered stable diagnostic templates.

### Orthogonal geometry value contract

Every node has exactly `id`, its node-discriminant `kind`, a mandatory
`valueType`, and the variant fields defined below. `valueType` is part of SPC1
and is recomputed from the node transition rules; it is not a backend hint:

```text
SemanticValueType = {
  geometryKind:
    "curve" | "wire" | "region" | "sheet" | "solid" | "solid-set",
  space: "d2" | "d3",
  representation:
    "analytic-brep" | "rational-brep" | "certified-approx-brep" | "mesh",
  evidence:
    { tag: "representation-preserving" }
    | {
        tag: "certified-approximation",
        certificateProfile: string,
        certificatePolicyHash: Sha256Hex
      }
}
```

`SemanticValueType` and both evidence variants use exactly the keys shown. The
referenced canonical certificate policy, including semantic approximation
bounds, is part of the node/core and its hash is recomputed rather than trusted.
`certificateProfile` is 1..96 ASCII characters matching
`[A-Za-z0-9][A-Za-z0-9._:-]*`; this keeps its inferred capability within the
128-character capability limit. `certificatePolicyHash` is exactly 64 lowercase
hexadecimal characters and is recomputed as SHA-256 of
`length-prefix("semantic-approximation-policy-v1")` followed by
`length-prefix(canonical policy bytes)`. An unknown profile fails closed.

The axes have distinct meanings:

- `Curve` is one parameterized geometric curve in 2D or 3D.
- `Wire` is one ordered, oriented, connected chain of curve uses in 2D or 3D;
  open and closed are explicit wire state.
- `Region` is a regularized planar 2D set and may contain multiple islands and
  holes; v1 therefore needs no misleading `RegionSet` synonym.
- `Sheet` is one oriented 2-manifold carrier in 3D, possibly trimmed and with
  boundary, but with no claimed enclosed volume.
- `Solid` is one connected, validated regular-closed volumetric component.
- `SolidSet` is the conservative 3D volumetric-language carrier. A non-empty
  B-rep payload contains one or more validated regular-closed components; its
  zero-component state is typed runtime `empty`, never a payload. The legacy
  Mesh representation retains the pinned legacy validity behavior and makes no
  new topology claim; its executor-internal `materialized-empty` compatibility
  state is the sole payload-bearing exception and is not a B-rep component set.

Allowed `(geometryKind, space)` pairs are exact: `Curve` and `Wire` allow
`d2|d3`; `Region` allows only `d2`; `Sheet`, `Solid`, and `SolidSet` allow only
`d3`. Kind is neither ambient dimension nor connected-component metadata
inferred from a mesh at publication time.

Representation is independent of kind:

- `AnalyticBrep` uses versioned analytic carriers such as lines, planes,
  circles, cylinders, and other admitted exact analytic forms.
- `RationalBrep` uses rational Bézier/NURBS carriers, including exact rational
  embeddings of analytic geometry.
- `CertifiedApproxBrep` contains intentionally approximated B-rep geometry and
  is valid only with matching certified-approximation evidence.
- `Mesh` is a semantic piecewise-linear/polygonal value. A viewport/export mesh
  tessellated from B-rep by TSP1 is a derived `MeshAsset`, not an implicit
  mutation of the SPC1 value's representation.

Evidence is not an `exact: boolean` and does not certify unrelated properties.
`RepresentationPreserving` means no intentional representation approximation
relative to the declared language value; it does not by itself prove global
manifoldness, connectedness, or absence of floating-point roundoff.
`CertifiedApproximation` names the exact verifier/profile and policy required
of execution. At runtime it additionally carries a `CertificateId`; certificate
bytes and their verification result enter topology/execution evidence, not
`programHash`. `AnalyticBrep` and `RationalBrep` require representation-
preserving evidence. `CertifiedApproxBrep` requires certified evidence. `Mesh`
may be representation-preserving for frozen legacy polygonal semantics or
certified when created by a future explicit semantic approximation node.
Evidence cannot be upgraded from certified approximation to representation
preserving by assertion, serialization, or backend choice.

The v1.0 node table below constructs representation-preserving values only.
The certified-evidence discriminant is frozen now but is not caller-constructible
until a recognized required feature adds an explicit approximation node, its
canonical policy payload, certificate profile, transition capabilities, and
vectors. A bare certified `valueType` on an existing node is invalid.

For the current subset, `legacy/current` lowers every 2D value as
`Region/d2/Mesh/RepresentationPreserving` and every 3D value conservatively as
`SolidSet/d3/Mesh/RepresentationPreserving`. This preserves the pinned
piecewise-linear result, including a compatibility-only degenerate result of a
singular `multmatrix`, without falsely claiming B-rep validity or one connected
solid. Such a legacy mesh cannot be consumed as B-rep through an implicit
conversion.

For `openscad-viewer/brep-1`, admitted analytic primitives produce
`Region/d2/AnalyticBrep` or `Solid/d3/AnalyticBrep` with representation-
preserving evidence. Piecewise-linear planar boundaries and polyhedra may be
lifted to analytic line/plane B-rep only by a validated, representation-
preserving constructor; a polyhedron with no proven single connected component
is `SolidSet`. NURBS constructors will produce `Curve`, `Wire`, `Region`, or
`Sheet` with `RationalBrep`. A B-rep operation declares its exact output
`valueType`; if its qualified capability cannot produce that type/evidence it
returns a typed refusal and never silently emits mesh or certified
approximation.

The lowerer derives `valueType` solely from the language contract, typed inputs,
node parameters, and versioned semantic transition table. It never consults the
currently installed provider or its readiness. Provider admission happens only
after closure validation and either accepts the already fixed contract or
refuses it.

Every existing node replaces its conflated `dimension` field with `valueType`.
All references point to earlier nodes, and input/output transitions are exact:

| Node | Exact variant fields and geometry-kind transition | Required capability |
| --- | --- | --- |
| `box` | positive `size: Vec3`, `center`; legacy `SolidSet`, B-rep `Solid` | `construct.box` |
| `sphere-analytic` | positive `radius`; B-rep `Solid` | `construct.sphere.analytic` |
| `sphere-polygonal` | positive `radius`, `radialSegments >= 4`; legacy `SolidSet` | `construct.sphere.polygonal` |
| `cylinder-analytic` | positive `height`, non-negative radii not both zero, `center`; B-rep `Solid` | `construct.cylinder.analytic` |
| `cylinder-polygonal` | analytic fields plus `radialSegments >= 3`; legacy `SolidSet` | `construct.cylinder.polygonal` |
| `polyhedron` | at least four `Vec3` vertices and ordered triangles (distinct indices for B-rep); legacy/current additionally admits only the exact empty `vertices=[]`, `triangles=[]` kernel constructor, while B-rep remains strict; conservative `SolidSet` | `construct.polyhedron` |
| `rectangle` | positive `size: Vec2`, `center`; `Region` | `construct.rectangle` |
| `circle-analytic` | positive `radius`; B-rep `Region` | `construct.circle.analytic` |
| `circle-polygonal` | positive `radius`, `radialSegments >= 3`; legacy `Region` | `construct.circle.polygonal` |
| `polygon` | ordered rings of at least three `Vec2`, `fillRule = even-odd`; `Region` | `construct.polygon` |
| `transform` | `input`, affine `matrix`; preserves all four value axes under the contract-specific singular rule | `operation.transform` |
| `boolean` | `union/intersection/difference`, at least two ordered compatible inputs; `Region -> Region`, any `Solid|SolidSet -> SolidSet` | operation and operation-specific capability |
| `hull` | at least two ordered compatible inputs; `Region -> Region`, 3D conservatively `SolidSet` | `operation.hull` |
| `linear-extrude` | `Region -> SolidSet`; positive `height`, finite `twistDegrees`, bounded `slices`, `scale: Vec2`, `center` | `operation.linear-extrude` |
| `rotate-extrude-analytic` | `Region -> SolidSet`; contract-valid finite `angleDegrees` | `operation.rotate-extrude.analytic` |
| `rotate-extrude-polygonal` | analytic fields plus `radialSegments >= 3`; legacy only | `operation.rotate-extrude.polygonal` |
| `projection` | `Solid|SolidSet -> Region`; `cut` | `operation.projection` |
| `offset` | `Region -> Region`; finite `distance` | `operation.offset` |

The analytic sphere/circle/cylinder/revolve variants are invalid for
`legacy/current`; their polygonal counterparts are invalid for B-rep. Legacy
segment/slice materialization is semantic core state. Local primitive checks do
not certify winding, self-intersection, manifoldness, or global B-rep validity.

The table is closed over descriptor combinations reachable in v1.0. A reserved
kind, representation, or evidence discriminant does not automatically become
an admitted input to an existing generic-looking node when a future constructor
starts producing it. Every added producer feature must also name each newly
admitted `(node kind, ordered input valueTypes) -> output valueType` signature;
all other combinations remain invalid. In particular, future rational NURBS
regions do not silently acquire exact offset/projection support, because those
operations are not generally closed over rational parametric representation.

### DAG ordering and aliasing

Node IDs are zero-based array positions. Every edge points to a lower node ID.
Starting from result items in authored result order, a depth-first traversal
of each node's authored input order must produce exactly the node table in
children-before-parent postorder. Unreachable nodes, forward references,
dangling references, and cycles are invalid.

Operand order is semantic even for mathematically commutative operations.
Encoders, lowerers, caches, and backends must not reorder Boolean or hull
operands. Their order can affect diagnostics, provenance, original-surface
lineage, entity identity, and exact mesh bytes.

Structurally equal nodes are allowed to remain distinct when they have
different producer occurrences. No v1 encoder or validator performs implicit
hash-consing. Node-edge sharing is legal only when the closed production rule
below names that exact shared edge inside one logical producer group. The
current v1.0 OpenSCAD table admits no cross-branch semantic-value reuse; alias
occurrence rows may reference a descendant-owned node but do not add a DAG
edge or transfer its materializer. A future language construct that reuses a
value outside its runtime-descendant chain requires a recognized feature and
an explicit reuse/lineage field or node. Allocation or object identity must
not create, remove, or justify sharing.

### Cardinality

The root result is canonical:

- `empty` has bottom type `never`, has no result items, and retains no geometry
  nodes;
- `single` has exactly one typed output reference;
- `multi` has at least two typed output references in language evaluation
  order.

Zero-operand operations lower to `empty` where the language contract permits
it. A one-item union or intersection lowers to an identity-preserving alias; a
one-item hull lowers to a same-node alias that re-owns identity. Difference
aliases only when its first authored child supplies the sole base item and no
cutter exists; an empty first child never promotes a cutter into the base.
None creates a one-input DAG node. Boolean and hull nodes therefore contain at
least two inputs accepted by their exact value-type signatures. Duplicate
authored geometry is preserved and must not be canonicalized away.

An `OccurrenceId` identifies one logical evaluation of a static operation. A
logical occurrence can own multiple output slots. Those records share the
same `OccurrenceId` and use zero-based, dense `outputOrdinal` values in
language-result order. The pair `(OccurrenceId, outputOrdinal)` is unique.
Every root output explicitly references both the occurrence that produced its
DAG node (`producerOccurrence`) and the occurrence that owns its stable scene
identity (`identityOccurrence`). They are equal for an ordinary materialized
operation; an identity-preserving alias or transform may keep the latter while
changing the former. The producer row must reference the output node, the
identity row must own exactly that output slot, and root order must agree with
the identity-slot order. Intermediate producing occurrences may be absent from
the root result but remain identity and provenance owners.

Different producer and identity occurrences require the complete recomputed
production proof below. Same-root, descendant, equal-node, and transform-input
checks are necessary local facts but are not sufficient: a forged sibling can
reuse a shared DAG node while satisfying all four. `producerOccurrence` must be
the exact output row emitted by its logical operation group, and
`identityOccurrence` must equal the identity owner recursively derived by that
group's frozen production rule. No caller-authored lineage assertion is
trusted.

### Recomputed occurrence-to-node production proof

Occurrence rows are first grouped by their recomputed `occurrenceId`. The
lowest-index row is the canonical group row and, for a value-producing group,
is exactly `outputOrdinal = 0`. Every group has exactly one of two forms:

- a zero/frame group has one row with `node`, `outputOrdinal`, and
  `sceneEntityId` all `null`;
- an output group has one or more rows with identical `occurrenceId`,
  `operation`, `parent`, `staticParent`, and `dynamicSlots`; each row's `id` is
  its array index, its `node` is non-null, its `outputOrdinal` is the dense
  `0..n-1` slot, and its `sceneEntityId` is recomputed from that slot. It has no
  producer-only null ordinal.

Rows of one output group need not be contiguous, but their array order must be
in increasing ordinal order and that ordinal sequence is their only output
order. Every `parent` and `staticParent` targets a canonical group row.
Canonical `parent` references form the sole runtime group tree. `staticParent`
is an identity/provenance binding and never contributes a production edge,
frontier item, child bucket, reachability path, or second parent.

For a group `G`, `frontier(G)` is recomputed from its direct runtime children in
canonical-row/evaluation order. An output child contributes all of its output
records in ordinal order and traversal stops at that child. A zero/frame child
is traversed only when its frozen rule is a transparent `control` or `module`
rule; every other zero-output child contributes nothing and hides its
descendants. Traversal is depth-first and never follows a DAG edge.

For an operation with a compiler-owned `$body`, validation also retains
`frontierBuckets(G)`: one ordered bucket for every direct runtime child group
of that body, with all nearest producing descendants reached through only
transparent groups. Buckets remain present when empty. Each non-empty frontier
item records its bucket's direct child occurrence, and flat `frontier(G)` is
the concatenation of the buckets. The empty boundary is essential: a later
`difference` cutter cannot become the base merely because the first child
produced no value. An admitted `$expansion` and all of its runtime descendants
remain inside the bucket of the enclosing `children()` group; its caller-body
`staticParent` never moves or duplicates them into a caller bucket. Operations
without a body use an empty bucket list.

The closed production function
`produce(languageContract, operation.category, operation.name,
frontierBuckets, frontier)`
returns three things: ordered semantic output roots, the set of newly
materialized DAG nodes owned by `G`, and one recursively derived identity owner
for each output. The occurrence rows must name exactly those roots. Each DAG
node has exactly one owning materializer group; an alias group owns no node.
Every edge of a node owned by `G` points either to another node in the exact
canonical expression owned by `G` or to the exact ordered frontier item named
by the rule. At the end every reachable node is owned exactly once and no
unclaimed, multiply owned, reordered, deduplicated, or cross-frontier edge is
accepted.

A synthetic program group has, in canonical-row order, every group whose
`parent = null`. Applying the same frontier traversal to this synthetic group
recomputes `programFrontier`. The result tag, cardinality, order, node,
`producerOccurrence`, and derived `identityOccurrence` must equal this frontier
exactly; a result cannot add an inner alias/descendant as another root, omit a
top-level output, or select an otherwise valid row ad hoc. Result color is core
meaning but is source-derived rather than reconstructible from the current
occurrence fields, so its source equivalence is covered by the trust states
below.

Global publication reachability is applied to `programFrontier`: it alone
defines result cardinality, scene identity and retained result leases. An
unchosen branch, which the language never evaluated, contributes neither value
nodes nor an effect. That rule does not authorize pruning geometry which the
pinned evaluator eagerly executed and only then discarded. In particular,
`difference()` evaluates and unions cutter geometry even when its first/base
bucket is empty. Constructors or that implicit union may fail; allocations may
affect original-ID equality, work/cancellation and later kernel behavior. A
warning/report retained after deleting its kernel work is not equivalent.

An ordered node list alone is still insufficient. The current lowerer can build
an invalid polyhedron node, continue evaluating source, and throw a later
`assert(false)` before a backend session is opened. Legacy execution performs
the earlier kernel call first. The plan must therefore represent the first
deterministic language terminal as a deferred event after its kernel prefix,
not throw it during inert planning.

The qualification-only implementation emits schema minor 1.2 and required
feature `semantic.execution-v2` with this exact, hash-covered core field:

```text
execution: {
  version: "semantic-execution-v2",
  evaluationOrder: readonly NodeId[],
  discardedEffects: readonly {
    tag: "legacy-difference-cutters",
    ownerOccurrence: OccurrenceIndex,
    root: NodeId
  }[],
  terminal: null | {
    tag: "legacy-language-error",
    occurrence: OccurrenceIndex,
    diagnosticTemplate: DiagnosticTemplateIndex,
    prefixFrontier: readonly {
      ownerOccurrence: OccurrenceIndex | null,
      root: NodeId
    }[]
  }
}
```

The trusted lowerer stores nodes in exact authored kernel-call order and emits
`evaluationOrder[i] === i`; the executor follows that authenticated array.
Production and independently implemented reference validators also replay the
exact schedule from occurrence production rather than accepting an arbitrary
dependency-safe permutation. A sibling order-only mutation and a coherent
node-ID/reference renumber which changes the replayed schedule are rejected.
Repeated loop activations are aggregated under their one authored `difference`
statement before base/cutter partitions and schedules are derived.
The lowerer remains an observationally inert geometry planner: it captures the
first deterministic runtime language error and stops planning. Syntax/UTF-8,
decode, trust, cancellation, panic and internal-validator failures are not
planned language terminals.

The 1.2 terminal names the canonical occurrence row of the interrupted dynamic
activation. Assignment statements have compiler-owned node-null `$assign`
occurrences. The referenced diagnostic template has severity `error`, binds its
operation to that occurrence, and has the exact argument pair
`errorName=OpenSCADParseError|TypeError` plus `detailSha256`; rendered text/span
are present in SPE1. The
exact `OpenSCADParseError` or compatibility `TypeError` used by the local facade
is a nonserialized `SemanticLoweringSuccess.terminalError` sidecar.

Structural validation proves a self-consistent schedule and that every row
after the claimed terminal belongs to the same logical activation or its
dynamic subtree. A parent may fail after evaluating all children or after some
map outputs, so the terminal occurrence need not be the last row and may itself
have completed node-bearing output rows. Structural SPC/SPE cannot prove that
its operation/occurrence structure came from the bundled source, nor that the
claimed activation is the exact deepest/first source error. Those source-to-
trace properties belong to the trusted lowerer and frozen source fixtures.
Normal execution therefore accepts only the exact object minted by
the process-local lowerer trust `WeakSet`; decoded, migrated or copied SPC/SPE
cannot reach the executor.

`prefixFrontier` is the ordered maximal live value frontier at the terminal
boundary. It makes every already-created node reachable without claiming a
publishable value. When `terminal` is non-null, `core.result` is exact
bottom/empty and the assembler is never called.

For a nonterminal program, `discardedEffects` is empty except for the closed
`legacy/current` rule above.
Each row names the exact zero-output `difference` activation and cutter value:
one cutter is used directly; multiple cutter values have the exact canonical
n-ary implicit union node; a zero cutter frontier adds no row. The base bucket
is evaluated and, when needed, implicitly unioned before any cutter statement.
All cutter statements are then evaluated and their implicit union is built even
when the base frontier is empty. This ordering is part of the versioned rule,
not an optimizer choice.

For terminal programs, prior/live discarded work is subsumed by
`prefixFrontier`; a separate `discardedEffects` row is forbidden. Every
materialized prefix root names its exact canonical owner, while a compiler-owned
internal union reducer has null ownership. Production and reference validators
independently reject a primitive root changed to null ownership, an injected
terminal effect, later-sibling terminal forgery, non-maximal roots and invalid
post-child/partial-map production.

On the process-local trusted path, terminal-prefix and effect nodes never enter
scene publication or commit retention. A kernel failure in the prefix/effect
wins and the later planned terminal is not emitted. Every 1.2 execution field
and the core error template enter canonical bytes and `programHash`; source
presentation remains bound by SPE1/sourceHash. Structural schedule/active-
subtree validation does not make a decoded artifact executable and must not be
described as proof of source-authored schedule or exact deepest/first error.

The precedence fixture is frozen without calling another evaluator at runtime:

```text
source:
  polyhedron(open-four-point-mesh);
  assert(false, "later") cube(1);

execution.evaluationOrder = [polyhedronNode]
execution.terminal = {
  tag: "legacy-language-error",
  occurrence: assertionOccurrence,
  diagnosticTemplate: assertionTemplate,
  prefixFrontier: [{ ownerOccurrence: polyhedronOccurrence, root: polyhedronNode }]
}
core.result = { tag: "empty", type: "never" }
```

If the polyhedron kernel call fails, that failure closes the session and is the
only public terminal. If an injected conforming kernel accepts it, the executor
closes the non-publishing prefix, releases every lease, and only then raises the
planned assertion. A runtime direct-evaluator fallback or consultation is
forbidden. Qualification uses frozen event traces, a recording/fault-injected
fake kernel and the independent validator; the historical direct evaluator
remains only a discovery corpus builder, never an oracle or fallback.

The SPC1 operation-name/category table is exact:

| Category and name | Production rule | Identity rule |
| --- | --- | --- |
| `geometry:cube` | empty frontier; one `box` root | own |
| `geometry:sphere` | empty frontier; one contract-selected `sphere-analytic|sphere-polygonal` root | own |
| `geometry:cylinder` | empty frontier; one contract-selected `cylinder-analytic|cylinder-polygonal` root | own |
| `geometry:polyhedron` | empty frontier; one `polyhedron` root | own |
| `geometry:square` | empty frontier; one `rectangle` root | own |
| `geometry:circle` | empty frontier; one contract-selected `circle-analytic|circle-polygonal` root | own |
| `geometry:polygon` | empty frontier; one `polygon` root | own |
| `transform:translate|rotate|scale|mirror|multmatrix` | map: one new `transform(input = frontier[i])` root per item; parameter coherence is defined below | preserve item owner |
| `geometry:projection` | map: one new `projection(input = frontier[i])` root per item; one shared `cut` value | own |
| `geometry:offset` | map: one new `offset(input = frontier[i])` root per item; one shared distance | own |
| `presentation:color`, `assertion:assert` | map-alias: output root `i` equals frontier root `i`; own no node | preserve item owner |
| `boolean:union|intersection` | zero items -> zero output; one -> same-node alias; two or more -> one exact ordered `boolean` root | preserve for alias, otherwise own |
| `boolean:hull` | zero items -> zero output; one -> same-node alias; two or more -> one exact ordered `hull` root | own, including one-item alias |
| `boolean:difference` | canonical partitioned reduction defined below | conditional below |
| `geometry:linear_extrude|rotate_extrude` | canonical union-reduce of all profiles, then one matching extrusion root | own |
| `control:$assign` | one node-null evaluation row per executed assignment; transparent and terminal-capable | none |
| `control:$body|$then|$else|$expansion|if|let|for|children|group|render` | one zero/frame row; transparent, owns no node | none |
| `module:<non-reserved user name>` | one zero/frame row; transparent, owns no node | none |

The category must match the row containing the name; a known name under the
wrong category, an authored name beginning with the compiler-reserved `$`
prefix, an unknown pair, or a node kind not selected by the pair fails closed.
Future operation pairs and NURBS constructors require recognized features and
new production vectors. `$assign` is compiler-owned: authored source cannot
inject another `$` operation or use the row as geometry production.

Compiler-frame recognition is structural, not name-only. `$body` requires a
final `body` path segment, `$then|$else` require `branch`, and `$expansion`
requires `control`; each has ordinal zero, category `control`, the exact static
owner relationship created by its corresponding operation, and the matching
runtime/static-parent relation, including only the narrowly admitted
`children()` continuation binding defined below for `$expansion`. No other `$`
name or path-kind/name combination is transparent. Likewise a user-module
definition ends in a
`module` segment while a call ends in `call`; both use the exact registered
non-reserved module name. A forged `control:$body` label on an ordinary call
therefore cannot expose its descendants to a frontier.

`unionReduce_G(items)` is canonical: zero items produce bottom, one returns the
existing item without ownership transfer, and two or more create exactly one
n-ary `boolean(operation = union, inputs = items)` node owned by `G`. It never
builds a binary tree or sorts/deduplicates inputs. Extrusion owns both this
internal union node, when present, and its public extrusion root; the internal
node needs no occurrence row.

For `difference`, `base = frontierBuckets[0]` and `cutters` is the ordered
concatenation of every later bucket. Both partitions are independently
union-reduced by the difference group. No bucket means no base; an empty first
bucket remains an empty base even when later buckets contain values, and
produces no output. Non-empty base with empty cutters returns the reduced base:
it preserves the base identity only when no union node was created, otherwise
it owns the new identity. With non-empty cutters, the group additionally owns
exactly one
`boolean(operation = difference, inputs = [reducedBase, reducedCutters])` root
and owns its identity. This encodes the legacy first-child boundary without an
unattested implicit node or synthetic scene entity.

For map rules, output cardinality and ordinals equal frontier cardinality,
row `i` consumes only frontier item `i`, and all operation parameters that are
common to the source call are field-equal across outputs. Thus projection
`cut`, offset `distance`, and the matrices for
`translate|scale|mirror|multmatrix` are each equal across their group. The
legacy rotation rule may project one authored rotation differently for `d2`
and `d3`; its matrices must be equal within each input-space bucket and may
differ across those two buckets only. Newly materialized map roots are
distinct. For aliases, the output node must equal the matching frontier node,
the group owns no node, and the table alone determines whether identity is
preserved or re-owned.

The recursively computed identity owner is part of validation state, not SPC1:
`own` selects the current output row, while `preserve` selects the matched
frontier item's already computed owner. A root result's `producerOccurrence`
must be its exact emitting row and its `identityOccurrence` must equal this
derived owner. This rejects sibling identity donation even if the forged row is
a descendant and references a structurally valid shared node.

No new production/lineage field is added for v1.0: under this closed operation
table the roots, ownership, and identity owner are uniquely recomputable, while
a caller-authored duplicate claim would still need the same proof. This
minimality depends on forbidding cross-branch reuse and on canonical hidden
reducers. A future operation whose ownership cannot be derived from its
ordered descendant frontier must add an explicit feature-gated lineage/reuse
variant; it cannot weaken this proof or overload an arbitrary operation name.

This is an ownership/lineage guarantee, not an independent replay of frontend
argument evaluation. Current fields intentionally collapse all transform calls
to a `transform` node, do not store every normalized source argument in
`dynamicSlots`, and carry final output color without a complete intermediate
color state. Consequently a structural validator can prove, for example, that
`translate` maps exactly its frontier to transform nodes, but cannot prove from
SPC1 alone that each matrix was computed from the exact authored translate
argument. The `trusted-lowered` boundary below supplies that stronger claim. If
source-free validation of exact operation attribution is made a requirement,
then a new canonical, feature-gated evaluated-operation payload is necessary:
at minimum an operation subtype plus all normalized evaluated arguments needed
to recompute every node parameter and propagated presentation value. Merely
adding another caller-authored producer index would not solve it.

A conforming validator implements group evaluation with an explicit bounded
stack, not host recursion. It detects runtime-tree cycles defensively, checks
occurrence, edge, output, and expanded-frontier limits before allocation, and
memoizes group results so nested transparent frames cannot cause unbounded repeated
frontier construction. Failure is a typed limit/order error, never a partial
proof.

Compile-time `result.tag = empty` is distinct from a typed geometry node whose
kernel evaluation yields the empty set, for example a disjoint intersection.
The executor-internal runtime union has exact variants:

```text
SemanticRuntimeValue =
  {
    tag: "empty",
    valueType: SemanticValueType,
    evidence: SemanticRuntimeEvidence
  }
  | {
      tag: "value",
      valueType: SemanticValueType,
      evidence: SemanticRuntimeEvidence,
      payload: BackendValue
    }
  | {
      tag: "materialized-empty",
      valueType: SemanticValueType,
      evidence: { tag: "representation-preserving" },
      payload: BackendValue
    }

SemanticRuntimeEvidence =
  { tag: "representation-preserving" }
  | {
      tag: "certified-approximation",
      certificateProfile: string,
      certificatePolicyHash: Sha256Hex,
      certificateId: CertificateId
    }
```

`BackendValue` is an associated payload selected by the canonical carrier key
`<geometryKind>/<space>/<representation>` from a provider's closed
`BackendPayloadFamily`; evidence is deliberately not hidden in that payload.
Invalid kind/space combinations have no key. TypeScript and Rust adapters may
encode the family differently, but a two-bucket `Region2|Solid3` generic or one
untyped payload that requires downcasting is not the v1 provider ABI.

`CertificateId` is content-addressed evidence, not a provider-chosen label:

```text
CertificateId = "cert:v1:" || SHA-256(
  length-prefix("semantic-approximation-certificate-v1"),
  length-prefix(
    length-prefix(certificateProfile UTF-8) ||
    length-prefix(certificatePolicyHash ASCII) ||
    length-prefix(canonical certificate bytes)))
```

The verifier recomputes the ID before applying the named profile and policy.
Unknown, missing, noncanonical, colliding, or unverifiable certificate bytes
are typed execution errors.

The canonical shared discriminants are `empty` and `value`; the executor also
admits the explicit runtime-only `materialized-empty` compatibility state only
for `legacy/current`. It is neither an opaque encoding of semantic `empty` nor
an ordinary non-empty `value`:

- `empty` is the mathematical typed empty used by shared algebra. It has no
  payload and no lease;
- `value` is a payload-bearing kernel value eligible for normal root
  materialization;
- `materialized-empty` says that the legacy provider actually constructed a
  kernel handle and that its exact kernel emptiness query is true, while its
  presence and operand order can still affect downstream Manifold bytes,
  provenance and failures. It therefore carries the same checked payload and a
  unique lease as `value`, participates in downstream kernel calls as present,
  and is never rewritten to shared `empty`.

`materialized-empty` is forbidden for `openscad-viewer/brep-1`, certified
evidence, SPC1/SPE1/TSP1 encoding, protocol-v5 fields and public v6 scene
payloads. It is accepted only by the shared executor after exact-key snapshot,
`legacy/current` check and the same session validation of
`(carrierKey,payload,lease)` as a value. The provider does not hide the state in
payload truthiness. A conforming Manifold adapter emits it only after the real
handle's exact kernel `isEmpty` query; a backend that merely asserts the tag is
non-conforming even though the generic executor cannot independently inspect an
opaque handle. The owning legacy assembler omits a materialized-empty root from
mesh publication while its lease is still live; a 2D authored root still
contributes the pinned top-level-2D warning. Disposal then releases it normally.

For the Manifold path this classification is total and bidirectional after
every admitted backend kernel operation: exact `isEmpty(handle) = true` returns
`materialized-empty`, and false returns `value`. That backend never substitutes
shared `empty` after it has invoked the kernel and obtained a handle; on this
path mathematical `empty` comes only from a proven executor reduction. This is
a provider conformance rule, not a new serialized semantic axis.

For all variants, the returned `valueType` must equal the node's SPC1
`valueType` field-for-field. Runtime evidence must satisfy the core evidence
requirement; certified fields must match and the additional `certificateId`
must resolve and verify. A backend may not change kind/representation or omit
evidence. A mathematical runtime-empty root retains occurrence identity in the
execution result but publishes no geometry payload or derived mesh.

Empty algebra is shared by all providers and tests only the exact `empty` tag.
Transform preserves typed empty; union ignores empty operands and is empty only
when all are empty; intersection is empty when any operand is empty; difference
is empty for an empty minuend and ignores empty subtrahends; hull ignores empty
operands and is empty when no non-empty operand remains. Extrusion, revolve,
projection, and offset map an empty input to their declared typed empty output.
A `materialized-empty` operand is not removed by any of these reductions: it is
passed to the legacy backend in authored order. These runtime reductions do not
rewrite SPC1 or its hash.

Only a certain, validated result may return `empty`. Unsupported capability,
invalid topology, ambiguity, an unresolved tolerance band, exhausted budget,
cancellation, panic, or missing certificate is a typed refusal/error and must
never be converted to empty. A certified approximation may certify emptiness
only when its named profile proves emptiness of the represented source value,
not merely an empty approximating payload.

### Backend session and payload ownership

**Verdict:** a stateless `validatePayload`/`evaluate` provider seam is not
resource-sound and is not an accepted G1 ABI; the session/lease shape below is
mandatory. In particular, racing an
`evaluate` promise against cancellation can reject the executor while that
promise later resolves with a newly allocated kernel object. Neither the
executor nor a plain `finally` then has an authoritative handle with which to
release it. The same leak exists when a provider allocates and then throws
before returning, a runtime-value snapshot or payload check rejects the
returned record, a later node fails, or the progress callback throws.

The v1 provider ABI therefore has one explicit per-execution session and an
opaque, unique lease token for every payload-bearing return: both `value` and
legacy `materialized-empty`. Payload object identity is not a lease: payload
families may contain primitive handles, two distinct leases may refer to a
provider-internal ref-counted object, and one payload object may not safely
encode release multiplicity. In TypeScript-like pseudocode, with the exact
carrier-key indexing elided only for readability:

```text
BackendPayloadLease<K> = opaque unique session-local token

BackendEvaluation<F, K> =
  {
    tag: "empty",
    valueType: SemanticValueTypeFor<K>,
    evidence: SemanticRuntimeEvidence
  }
  | {
      tag: "value",
      valueType: SemanticValueTypeFor<K>,
      evidence: SemanticRuntimeEvidence,
      payload: F[K],
      lease: BackendPayloadLease<K>
    }
  | {
      tag: "materialized-empty",
      valueType: SemanticValueTypeFor<K>,
      evidence: { tag: "representation-preserving" },
      payload: F[K],
      lease: BackendPayloadLease<K>
    }

BackendCloseOutcome =
  {
    tag: "commit",
    retained: readonly BackendPayloadLease[]
  }
  | {
      tag: "abort",
      code: "E_SEMANTIC_ABORTED" | "E_SEMANTIC_DEADLINE",
      node: NodeId | null
    }
  | {
      tag: "failure",
      code: BoundedSemanticExecutionErrorCode,
      node: NodeId | null
    }

SemanticProgramBackend<F>.begin(context): SemanticBackendSession<F>

SemanticBackendSession<F> = {
  evaluate<K extends SemanticCarrierKey>(
    node: SemanticNodeFor<K>, borrowedInputs, context: BackendContext<K>
  ): BackendEvaluation<F, K> | Promise<BackendEvaluation<F, K>>,
  validatePayload<K extends SemanticCarrierKey>(
    carrierKey: K, payload: unknown, lease: BackendPayloadLease<K>
  ): payload is F[K],
  releasePayload<K extends SemanticCarrierKey>(
    lease: BackendPayloadLease<K>
  ): Promise<void>,
  close(outcome): Promise<
    { tag: "closed" }
    | { tag: "committed", resultLease: BackendResultLease }
  >
}

BackendResultLease = {
  dispose(): Promise<void>
}

SemanticExecutionResult = {
  program,
  attestation,
  outputs,
  dispose(): Promise<void>,
  [Symbol.asyncDispose](): Promise<void>
}
```

`begin` is synchronous and atomic. It may throw, but if it throws it has not
published a session and the provider must already have reclaimed anything it
created. A provider needing asynchronous initialization returns a session
first and performs that initialization as a session-owned operation; this
keeps cancellation and cleanup reachable. The begin context contains the
program hash, language contract, limits, and the executor-owned abort signal,
but never source text or route choice.

Each `evaluate` context also carries an optional core-only `producer`. For a
node materialized by an authored operation it is exactly the canonical producer
occurrence index, its static operation index, and that operation's exact name,
all recomputed by the validator's production proof. It contains no source,
span, label, envelope policy or route. It is null only for an admitted internal
reducer with no authored producer. A transform node always has a non-null,
unique producer whose name is one of the transform production names; missing,
ambiguous or mismatched producer data is a contract failure before a kernel
call. The validator/executor rejects an ambiguous or core-mismatched projection
without calling `evaluate`; the backend independently rejects a null producer
or a locally invalid transform operation name before allocation.

This producer projection is semantic backend input, not display provenance.
Its source operations/occurrences are already in SPC1 and `programHash`; any
node-result cache must therefore bind the complete program/producer semantics
and must not key a transform result by node bytes alone. This distinction is
required even when matrices are bitwise equal: under `legacy/current`, a
zero-normal `mirror()` constructs a kernel empty handle and returns the explicit
`materialized-empty` runtime state, while a singular `multmatrix()` is passed to
the kernel and the frozen golden returns ordinary `value`. A backend may branch
on the validated producer name for that frozen dispatch rule; it may not
inspect source to rediscover it.

Producer is not an emptiness certificate, runtime tag, carrier lease or public
provenance field. It identifies the current node's authored materializer. A
`materialized-empty` therefore retains the producer of the operation that
created that handle (for example `mirror` or exact legacy `polyhedron`), while
a downstream Boolean/transform is evaluated with its own producer rather than
the empty child's producer. Kernel `isEmpty` determines the returned runtime
tag only after that authored dispatch. Neither tag may be inferred from
producer alone, and producer does not authorize `materialized-empty` outside
`legacy/current`.

The session, not the returned JavaScript/Rust value, is the initial owner. A
provider registers every allocation in the session before any async yield,
callback, possible throw, or promise resolution that can expose or lose it. A
provider does this through an atomic session allocator or holds a provider-local
RAII guard until synchronous registration succeeds; there is no unowned gap
between a native allocation returning and the session ledger acquiring it. A
`value` and `materialized-empty` returns carry their registered lease; `empty`
carries no lease. The executor snapshots the exact return record, admits
`materialized-empty` only under `legacy/current`, and asks the same session to
prove that `(carrierKey, payload, lease)` is a registered matching triple. A false
answer, a throw, an unregistered token, a token from another session, an extra
field, an accessor/proxy trap, a null payload, or a descriptor/evidence
mismatch is a typed invalid-backend return. It does not transfer ownership.
Session closure remains able to reclaim an allocation even when the invalid
record made its token unreadable to the executor.

The lease is executor-private metadata. After validation the executor snapshots
the executor-facing `SemanticRuntimeValue` shown above without the lease field;
callers receive the payload only through an owning `SemanticExecutionResult`
and cannot individually transfer or release root tokens. This TypeScript API
is not a protocol-v6 scene serialization.

Inputs to `evaluate` are borrowed for that call. The provider may not release
them, take ownership of them, mutate their ownership state, or retain an
untracked borrow after the call settles. Every payload-bearing output lease
(`value` or `materialized-empty`) must remain valid independently after those
input borrows end. A kernel may alias
or share its internal object only by making the output lease hold its own
reference/count and releasing that reference independently. Returning an input
lease as an output lease is forbidden. This permits the executor to reclaim an
intermediate after its final authored DAG use without knowing kernel internals.

There are exactly four lease-state transitions:

| From | Operation | To | Owner after the transition |
| --- | --- | --- | --- |
| provider allocation | synchronous session registration | session-owned | session |
| session-owned | successful `releasePayload` | released | none |
| session-owned | successful `close({ tag: "commit", retained })` for a retained token | result-owned | returned result lease |
| result-owned | first `result.dispose()` | released | none |

No other transfer is valid. `releasePayload` is permitted for a dead
intermediate and is idempotent for the same token: duplicate/concurrent calls
return the same promise and never double-free. The executor nevertheless keeps
its own token ledger and issues at most one release request. A determinate
release failure before native deletion leaves the token session-owned so
`close` remains the cleanup backstop. Once a native delete outcome is
indeterminate, retry is forbidden: ownership is fenced inside the permanently
quarantined module instead of being returned to the reusable session ledger.
The baseline implementation may retain all values until the graph is complete;
an optimized implementation precomputes authored-edge use counts plus result
roots and releases non-root leases at last use. Both produce the same ownership
trace at close. Runtime `empty` values never enter this ledger;
`materialized-empty` values always do.

`close` is the mandatory quiescence boundary. On `abort` or `failure`, it
signals/stops provider work and does not resolve until every in-flight operation
has settled or been synchronously terminated and deletion of every registered,
nonreleased lease has been attempted once. A determinate close reclaims them
all; an indeterminate native delete permanently fences the unresolved ownership
inside a quarantined module. This includes an `evaluate` that resolves only after
the executor's cancellation/deadline race has already selected the other
branch. Cleanup itself is not raced against the user's expired deadline or
aborted signal. An in-process provider must offer bounded close; an isolated
worker provider may satisfy the same terminal guarantee by terminating and
joining the worker. A provider that can neither quiesce nor terminate is not
qualified for the execution boundary.

On `commit`, `retained` is the deduplicated set of leases referenced by public
root outputs. `close` first quiesces all work, releases every session-owned
non-retained lease, and then atomically transfers the retained set into one
`BackendResultLease`. Until its promise resolves, retained leases are still
session-owned. If commit-close rejects, no transfer occurred and no execution
result is returned. Determinate cleanup reclaims all tokens; indeterminate
native ownership is fenced in a permanently quarantined module. For an empty
result it returns a valid no-op result lease. After any close begins, no further
evaluation or ordinary release may begin.

The executor calls `close` exactly once, but the session defensively memoizes
the first close promise. Repeating the identical outcome returns that promise;
a conflicting outcome is a provider-contract error and cannot change resource
state. Close receives only the bounded outcome code and node ID, never an
arbitrary thrown object. On a failure path the original semantic execution
error remains primary. If cleanup also fails, the source-facing Manifold facade
returns ordered `AggregateError([mappedPrimary, cleanupError])`; neither cause
may be dropped or mistaken for empty geometry. Any indeterminate Manifold
`delete()` permanently quarantines that backend/module lane. Its session may
finish releasing the busy flag, but the same backend instance and raw module are
never reusable: every future `begin()` fails `E_MANIFOLD_PLAN_LIFECYCLE` with
the quarantine cause. On a success path, close failure becomes a typed backend-
close error and applies the same quarantine rule when ownership is indeterminate.

The returned `SemanticExecutionResult` is the sole owner of its root payloads.
Its `dispose` synchronously latches the disposed state and memoizes one async
promise; repeated or concurrent calls return that same promise. A rejected
dispose is also terminal and is not retried by calling provider release twice.
It may reject only after remaining ownership is known released, or fenced inside
a permanently quarantined/terminated module which cannot serve another session.
Ambiguous cleanup is a provider qualification failure, not a recoverable
disposal result or permission to reopen the lane.
`Symbol.asyncDispose` delegates to it. Payload use after disposal is invalid,
and consumers must not cache a raw kernel handle beyond the result lifetime.
A finalizer may report an undisposed result in development but is never the
correctness mechanism. Browser and MCP entry points must use `await using` or a
`try/finally` that awaits disposal after serialization/tessellation/export.

Construction around the ownership boundary is ordered as follows:

1. authenticate, validate, attest, check static budgets, and sample control
   before `begin`, so these failures allocate no backend resources;
2. begin one session, and thereafter route every exit through its single close;
3. evaluate `execution.evaluationOrder` and snapshot values while recording
   each validated lease;
4. if a planned terminal is reached, failure-close with bounded code
   `E_SEMANTIC_LANGUAGE_TERMINAL`, await exact cleanup, and throw only its
   diagnostic reference; otherwise derive outputs and the deduplicated
   retained-root set;
5. on the success branch, call commit-close and receive the result lease;
6. construct/freeze and return the public result under a guard that disposes
   the result lease if any throw occurs after commit-close but before return.

The executor aborts the session controller before failure/abort close. A throw
from `evaluate`, descriptor snapshot, `validatePayload`, `releasePayload`,
`onNode`, output construction, or control sampling follows the same path. The
provider remains responsible for allocations made before an `evaluate` throw
or rejection because they were registered before the throw; this is the only
sound treatment of “allocated but threw before return.”

The required lifecycle test matrix is:

| Scenario | Required observable ownership/result |
| --- | --- |
| trust/attestation/budget/control failure before begin | `begin`, `close`, and release are not called |
| `begin` throws after provider-internal allocation | begin rejection occurs only after provider cleanup; executor has no session to close |
| synchronous `evaluate` throw before return | failure-close reclaims the session-registered hidden lease exactly once |
| asynchronous `evaluate` rejection | failure-close quiesces it and reclaims every prior/current lease |
| abort or deadline wins while `evaluate` remains pending, then it resolves a value | signal is observed; close waits for the late settlement and releases its lease; the caller receives only the original abort/deadline error |
| invalid prototype, accessor, exact keys, tag, descriptor/evidence, null payload, foreign/unregistered lease, or `validatePayload` false/throw | no ownership transfer; failure-close releases the candidate and all earlier live leases |
| valid value followed by post-validation abort, a later-node failure, or `onNode` throw | all live leases close exactly once; no result escapes |
| planned language terminal with an empty kernel prefix | after trust/budget/control prechecks, `begin` still opens a session; begin failure wins, otherwise failure-close receives `E_SEMANTIC_LANGUAGE_TERMINAL` and only then exposes the sidecar terminal |
| planned language terminal after successful prefix/effect nodes | failure-close receives `E_SEMANTIC_LANGUAGE_TERMINAL`, retains no roots, releases every ordinary/materialized-empty lease exactly once, and only then exposes the terminal reference; assembler is not called |
| kernel failure in the prefix before a planned language terminal | kernel failure owns the sole close/public terminal; the later language diagnostic is not rendered |
| mapped legacy kernel failure plus session cleanup failure | source-facing facade throws ordered `AggregateError([mappedPrimary, cleanupError])`; the Manifold lane is permanently quarantined and every future begin fails with backend cause `E_MANIFOLD_PLAN_LIFECYCLE` |
| shared empty-algebra reduction | no lease is invented or released for the empty value; any now-dead payload-bearing inputs still follow normal ledger release |
| `materialized-empty` operand and downstream operation | its payload/lease is passed in authored position exactly like `value`; it is never removed by shared empty algebra |
| `materialized-empty` published root | assembler emits no mesh for that root while it is live; commit retains its lease and result disposal releases it exactly once |
| `materialized-empty` effect-only root | exact downstream effect graph is executed; commit does not retain the root and session cleanup releases it exactly once |
| successful multi-node graph | every non-root lease is released once; only the deduplicated root set is passed to commit-close |
| duplicate/aliased root references | one retained token and one final release, while all output rows remain present and ordered |
| successful runtime-empty result | commit-close receives an empty retained set; result disposal is a repeatable no-op |
| commit-close rejects | no result is returned and no token is ambiguously result-owned |
| injected throw after successful commit-close but before public return | guard invokes the returned result lease's dispose exactly once |
| two simultaneous `result.dispose()` calls, then a third call | all calls receive the same promise and the backend frees every root once |
| result disposal has an indeterminate native delete | all later calls receive the same rejection; no second delete occurs; unresolved ownership is fenced in a permanently quarantined module and every future begin fails `E_MANIFOLD_PLAN_LIFECYCLE` |
| duplicate identical close versus conflicting close | identical call returns the memoized promise; conflicting outcome cannot mutate ownership |
| output shares a provider-internal object with an input under distinct leases | input release cannot invalidate output; provider ref-count/lease conformance is demonstrated |

Every failure row is exercised for the first node, a middle node with earlier
live values, and the final/root node where applicable. The tests use counted
fake leases and a late-resolving backend, assert zero live provider allocations
after every determinate awaited close/dispose, and for indeterminate deletion
assert permanent lane quarantine plus future-begin refusal. They inject throws at every boundary between
allocation, registration, return, snapshot, validation, retention, close, and
public return. Merely observing that an abort signal fired is not a lifecycle
test.

### Coercion and representation-transition rules

There are no implicit space, geometry-kind, representation, or evidence
coercions. In particular, a backend cannot make inputs compatible by
tessellating B-rep, reconstructing B-rep from mesh, filling a wire, discarding
solid components, or weakening evidence. The admitted transitions are:

| Transition | Rule |
| --- | --- |
| same type -> same type | Alias and presentation/color preserve all four axes and stable identity. |
| `Curve -> Wire` | Future explicit `make-wire` node; ordered endpoints/domain compatibility are validated. |
| `Wire/d2 -> Region/d2` | Future explicit `fill-region`; requires closed planar wire evidence. |
| `Region/d2 -> SolidSet/d3` | Explicit linear/rotate extrusion in the current subset; future sweep is also explicit. |
| `Solid|SolidSet/d3 -> Region/d2` | Explicit projection or slice only. |
| `Solid -> SolidSet` | No coercion node is needed for Boolean admission: the Boolean signature accepts either input kind and its 3D result is always `SolidSet`. |
| `SolidSet -> Solid` | Never implicit; a future explicit `require-single-solid` validates exactly one non-empty connected component or returns a typed error. |
| `AnalyticBrep -> RationalBrep` | Only an explicit, representation-preserving conversion node/capability. |
| B-rep -> `Mesh` | TSP1 viewport/export tessellation is derived outside SPC1; a semantic Mesh requires a future explicit approximation/conversion node. |
| `Mesh` -> B-rep | Future explicit reconstruction/healing operation with validation and declared evidence; never routing fallback. |
| representation-preserving -> certified approximation | Only an explicit approximation node whose policy is in SPC1. |
| certified approximation -> representation-preserving | Forbidden; neither `exact()` nor downstream operations may upgrade evidence. |

Mixed-representation operands are rejected unless the node signature names the
exact combination and output transition. Operations consuming certified values
must preserve certificate lineage and cannot claim representation-preserving
output. New conversion nodes require recognized feature IDs, exact capability
closure entries, and new golden vectors; a provider-local conversion is never
observable as if it were absent from SPC1.

## Static operation and dynamic occurrence identity

`NodeId`, `SemanticOperationId`, `SemanticOccurrenceId`, and
`SemanticSceneEntityId` are different namespaces:

- `NodeId` is a local canonical DAG index and is never persistent identity;
- `SemanticOperationId` identifies a static source operation path;
- `SemanticOccurrenceId` identifies one evaluated operation in a module,
  control-flow, or loop expansion;
- `SemanticSceneEntityId` identifies one output slot of an occurrence.

`SemanticSceneEntityId` is source/evaluation identity, not geometry-value or
cache identity. A consumer must pair it with language contract and `valueType`.
Selection-like state may cross a revision only under the ambiguity rules;
kind-, representation-, or evidence-specific state additionally requires equal
compatible axes. Geometry payloads, topology handles, certificates, and export
eligibility are never reused from SceneEntityId alone.

IDs are recomputed, not trusted as caller-authored strings:

```text
SemanticOperationId =
  "opv1:" || SHA-256(length-prefix("semantic-operation-v1"),
                     canonical structural path)

SemanticOccurrenceId =
  "occv1:" || SHA-256(length-prefix("semantic-occurrence-v1"),
                      parent occurrence ID,
                      static-parent occurrence ID,
                      operation ID,
                      ordered typed dynamic slots)

SemanticSceneEntityId =
  "entity:v2:" || SHA-256(length-prefix("semantic-scene-entity-v1"),
                          occurrence ID,
                          output ordinal)
```

The identity payload after the domain is UTF-8 of canonical compact JSON, not
SPC1 and not host `JSON.stringify` over an arbitrary object. Primitive values
use JSON spellings after semantic numeric validation; arrays preserve order;
object keys are unique and sorted by raw UTF-8 bytes; no whitespace or Unicode
normalization is added. The preimage is exactly `u32be(domain byte length)`,
domain bytes, `u32be(payload byte length)`, and payload bytes. Native and
TypeScript implementations must use this same identity codec and vectors.

The compact-JSON primitive grammar is frozen to the ECMAScript
`JSON.stringify` spelling used by the v1 implementation. Finite numbers use
ECMAScript's shortest round-tripping decimal form, including its exponent
thresholds and explicit `+` in positive exponents. Strings escape quotation
mark, reverse solidus, and U+0000..U+001F exactly as JSON string serialization
does; solidus and well-formed non-ASCII scalar values, including U+2028 and
U+2029, remain unescaped UTF-8. Hex digits in `\u00xx` escapes are lowercase.
No native implementation may substitute a locale formatter, a generic
serde/default JSON policy, or a different canonical-JSON profile without an
identity-version change.

Structural paths use explicit segment kind, name, and ordinal. For every
`(parent, category, name)` sibling group, ordinals are zero-based, dense, and
assigned in structural source order. Operation rows are deterministic
preorder with every parent before its children. A validator must reject gaps,
duplicates, or a different operation-table order even when all references are
otherwise valid.

Dynamic slot values are a discriminated tree (`undefined`, `null`, Boolean,
finite number, string, or vector), never a JavaScript stringification. Slot
order is authored evaluation order. `duplicateOrdinal` is zero-based and dense
among equal typed values for the same slot in one parent occurrence. This
preserves stable identities when distinct loop values are reordered while
still distinguishing repeated equal values.

Every occurrence row has the exact fields `id`, `occurrenceId`, `operation`,
`parent`, `staticParent`, `dynamicSlots`, `node`, `outputOrdinal`, and
`sceneEntityId`. Rows are emitted in language evaluation order and referenced
rows precede their children. Both parent fields are occurrence indexes or
`null`, but they have different meanings:

- `parent` is the immediate runtime evaluation frame. A module call owns its
  caller `$body` frame, that body owns the selected module-definition
  activation, and the definition owns its lexical body evaluation. Loop,
  branch, and `children()` expansion frames remain explicit even when they
  produce no geometry.
- `staticParent` binds the runtime row to the static operation tree. If
  `operations[operation].parent` is `null`, it is exactly `null`. Otherwise it
  is normally the canonical row of the nearest occurrence on the `parent`
  ancestor chain whose `operation` equals that static parent operation. Every
  normally skipped row between `parent` and `staticParent` must reference an
  operation whose category is exactly `control` or `module`; encountering any
  other category, the end of the chain, or an earlier matching occurrence makes
  the row invalid. Thus the usual case has `staticParent = parent`.

An operation whose static parent is `null` also normally requires runtime
`parent = null`; otherwise an arbitrary root operation could be injected into a
different branch while keeping a null `staticParent`. The sole exception is the
module-definition activation `d` anchored directly below its matching caller
body in the continuation tuple below. A module definition is declaration-only
outside such a validated activation.

There is exactly one non-category-skip exception: the compiler-owned
`$expansion` continuation used by `children()`. It is necessary because caller
children remain lexically owned by the caller's module-call body but execute at
the callee's `children()` site, which may be below transforms, Booleans, or
other producing operations. The exception is admitted only when all of the
following are recomputed from existing fields:

1. Static operations form an exact call continuation. `W` is a `module`
   operation whose final path segment is `call` and whose non-reserved name is
   `m`. Its unique `$body` child `B` has category `control`, final path kind
   `body`, and ordinal zero. `B` has exactly one compiler `$expansion` child
   `E`, with category `control`, final path kind `control`, and ordinal zero;
   all registered caller-child operations are static descendants of `E`.
2. Runtime groups form one activation, not a second parent graph. A canonical
   call group `w` instantiates `W`; its sole direct body group `b` instantiates
   `B` with `b.parent = w`. Its module-definition activation `d` is the sole
   direct runtime child of `b`, has category `module`, final path kind `module`,
   the same name `m`, and the same ordered parameter-slot names and typed values
   as `w`; each side's duplicate ordinals are independently recomputed.
3. The canonical `children` group `c` has category `control`, name `children`,
   and final path kind `control`. It is a runtime descendant of `d`, its static
   operation is a lexical descendant of `d.operation`, and `d` is the nearest
   containing module-definition activation. Producing groups between `d` and
   `c` are allowed and remain semantically active; they are not skipped by
   production traversal.
4. The expansion group `e` is exactly one zero/frame row instantiating `E`, is
   the sole direct runtime child of `c`, has `e.parent = c`, and has
   `e.staticParent = b`. `b` is the nearest runtime ancestor instantiating `B`,
   even under recursion. The `children` and
   `$expansion` groups each carry exactly one `$index` slot with the same
   canonical typed identity value and independently recomputed duplicate
   ordinal. Every runtime child admitted below `e` is normally
   static-parent-bound to `e` and belongs to `E`'s caller-child static subtree.

Only step 4 may cross non-control/module runtime ancestors for `staticParent`.
It does not authorize any other row to skip them, does not allow an expansion
to bind an outer or sibling call body, and does not infer module resolution
from an opaque ID. The expansion occurrence ID still hashes both the immediate
`children()` parent ID and caller-body static-parent ID, so repeated expansion
sites, recursive activations, and repeated equal arguments remain distinct.
No new v1 field is needed because every anchor in this tuple is already an
exact operation/occurrence reference and enters recomputed identity. A future
first-class continuation that can detach from this active call/definition pair
requires an explicit feature-gated continuation edge/ID; it cannot generalize
the skip rule.

For example, this admitted source:

```text
module wrap() { translate([1, 0, 0]) children(); }
wrap() cube();
```

keeps two static lexical subtrees:
`definition(wrap) -> definition.$body -> translate -> translate.$body ->
children` and `call(wrap) -> caller.$body -> caller.$expansion -> cube`. It has
the single runtime-parent chain
`call(wrap) -> caller.$body -> definition(wrap) -> definition.$body ->
translate -> translate.$body -> children -> caller.$expansion -> cube`.
Only the `$expansion` row points its `staticParent` back to `caller.$body`;
every other static binding follows the ordinary rule. Production follows the
runtime chain, so `frontierBuckets(translate)` contains the expanded cube once,
the translate group materializes the transform, and the program root uses the
translate producer while preserving the cube's recursively derived identity.
Following the `$expansion.staticParent` edge for production would both bypass
the transform and duplicate/rebucket the cube, and is invalid.

The canonical row of a logical occurrence is its lowest-index row, which is
`outputOrdinal = 0` for every output group; all `parent` and `staticParent`
references target that row. `occurrenceId` is recomputed from
the exact compact-JSON payload `{ parentOccurrenceId,
staticParentOccurrenceId, operationId, dynamicSlots }`, where both referenced
IDs are `null` or the recomputed IDs of those rows. The explicit static-parent
ID remains in the preimage even when it equals the immediate parent ID.

A non-producing logical occurrence has exactly one row. Repeated rows for one
logical occurrence are allowed only for its dense output slots and must
otherwise have identical operation, parent, static-parent, and dynamic-slot
data.

Same-name positional source siblings are not strong cross-revision identity.
Every member has `identityEvidence = same-name-positional` and the same
derived `ambiguityGroup`; a singleton must use `structural-unique` and no
group. Scene publication may carry state across revisions only when the
evidence and group multiplicity make the match unambiguous. It must never
recover identity strength by parsing an opaque ID string.

Protocol v5 compatibility adapters continue publishing the frozen legacy raw
operation/entity IDs. Fixed-size v2 IDs and explicit ambiguity evidence become
public only with the atomic protocol-v6/`GeometrySceneV2` transition. State is
not silently matched across the v5/v6 identity boundary.

## Capability closure

`declaredCapabilities` and `capabilityClosure` are unique arrays sorted by raw
UTF-8 bytes. The v1 closure is deterministic:

```text
closure = sorted_unique(
  declared capabilities
  union fixed semantic-program/operation/identity/result capabilities
  union capabilities implied by every node discriminant
  union geometry-kind, space, representation, and evidence capabilities
  union certified-evidence profile and policy capabilities
  union operation-specific Boolean capabilities
  union deterministic-diagnostic capability when templates exist
)
```

The canonical inferred prefixes are `geometry.kind.*`, `geometry.space.*`,
`representation.*`, and `evidence.*`. A certified approximation additionally
requires exactly `evidence.profile.<certificateProfile>` and
`evidence.policy.sha256.<certificatePolicyHash>`. When an admitted explicit
conversion node changes an axis it contributes its node operation capability
and the applicable canonical transition IDs
`geometry.transition.<sourceKind>.<sourceSpace>.to.<outputKind>.<outputSpace>`,
`representation.transition.<sourceRepresentation>.to.<outputRepresentation>`,
and `evidence.transition.<sourceTag>.to.<outputTag>`. Unchanged axes contribute
no transition ID. Merely supporting the output descriptor is not permission to
perform a transition.

`semantic-capabilities-v1` has no additional implicit transitive edges beyond
the explicit implications above. Adding an implication changes the capability
graph version. The validator independently recomputes the closure and rejects
missing, extra, duplicate, or misordered values. Provider admission checks the
validated closure after lowering; the provider cannot weaken it.

The leading source header is still checked before provider readiness according
to ADR 0002. A planned v5 execution descriptor contains the authored header
requirements because v5 cannot attest the new closure. Protocol v6 introduces
an exact-key runtime descriptor that distinguishes authored requirements from
the validated program closure. An old descriptor must not acquire closure by
an unchecked extra JSON field.

## SPE1: source-bound envelope

`SPE1` encodes one `SemanticProgramEnvelopeV1` containing a complete SPC1
core plus source-bound data. The source descriptor is:

```text
source.sha256            = SHA-256(exact well-formed UTF-8 source bytes)
source.utf8ByteLength     = byte length of those exact bytes
source.utf16CodeUnitLength = JavaScript/DOM UTF-16 length
```

Ill-formed Unicode is rejected before UTF-8 encoding, so replacement-character
aliases cannot become source identity. Unicode normalization is forbidden:
canonically equivalent source strings remain different exact source bytes.

Source spans are half-open UTF-16 code-unit offsets `[start, end)`, matching
the current browser/editor and v5 contracts. Static-operation provenance has
exactly one non-empty span per operation in operation order. Diagnostic spans
may be empty. Every span is bounded by `utf16CodeUnitLength`.

Structural SPE1 validation without source bytes proves only bounds against the
attested lengths. Before a span is used for UI selection, diagnostics,
publication, persistence, or cache rebinding, the consumer must provide the
exact source, recompute all descriptor fields, and verify that neither endpoint
splits a surrogate pair. A digest-only envelope is not sufficient evidence of
scalar-boundary validity.

### Trust states and permitted claims

Schema validity, exact-source binding, and trustworthy lowering are three
different claims. An artifact has exactly the highest state whose evidence the
consumer has verified:

1. `structurally-valid-core`: SPC1 exact-key, hash, type/transition, capability,
   occurrence-production, ownership, schedule, and identity-owner validation
   passed. It proves a self-consistent execution-shaped semantic core, but is
   not executable through the normal API. It does not prove that
   any source text lowers to that core; primitive parameters, transform
   matrices, colors, and other evaluated arguments cannot all be reconstructed
   from the identity slots currently carried by SPC1.
2. `exact-source-bound`: the complete SPE1 passed structural validation and the
   consumer supplied the exact source bytes, recomputed its hash and both
   lengths, checked route/header requirements, and validated every span against
   those bytes. This proves that the envelope is attached to those bytes, but
   not that its enclosed core is the result of lowering them.
3. `trusted-lowered`: a qualified lowerer produced the core from those exact
   source bytes and policy in the executing process. A receiver may independently
   re-lower those bytes, but execution uses the newly minted process-local object;
   exact SPC1-byte comparison does not transfer its brand to decoded bytes. A
   future cross-process alternative would require a separately versioned
   authenticated lowerer statement binding at least the lowerer manifest/
   fingerprint, language contract, semantics revision, exact `sourceHash`, and
   `programHash`; hashes supplied by the artifact itself are not authentication,
   and such a statement is not accepted by the current normal executor.

The in-process representation of `trusted-lowered` is an unforgeable capability
or branded result minted only after atomic parse/lower/validate/encode. It is
not a caller-set Boolean and is deliberately absent from SPC1 and SPE1. A
persisted or MCP/browser-crossing artifact loses that process-local state. The
current normal source execution/publication APIs accept only the exact object
minted by their process-local lowerer; receiver-side re-lowering creates that
new object, while a verified external statement remains future design evidence.
A structurally valid third-party core may be admitted only by an explicitly
separate nonexecuting core-import API, labeled unattested and never presented as
the meaning of bundled source. Flat-v0 is refused with
`E_SEMANTIC_RELOWER_REQUIRED`; historical source attachment or a new hash cannot
upgrade it.

The recomputed production proof above is therefore sufficient without a new
SPC1 lineage field for the closed v1.0 operation subset, but it is not a source
equivalence proof. A future non-derivable cross-branch value reuse needs a
feature-gated lineage/reuse field or node. A future portable lowering trust
claim needs external authenticated evidence or deterministic receiver-side
re-lowering, not another unauthenticated field inside the artifact.

Provenance labels, rendered diagnostic messages, diagnostic spans, and source
spans are display/source evidence. They are in SPE1 but never in SPC1 or
`programHash`. Stable diagnostic templates remain in SPC1: code, severity,
operation reference, ordered typed arguments, and order are semantic; localized
or formatted text is not.

## TSP1: program-bound tessellation intent

`TSP1` is a canonical program-bound tessellation-intent payload:

```text
{
  schema: "semantic-tessellation-policy",
  schemaVersion: { major: 1, minor: 0 },
  programHash,
  intents: ordered SemanticTessellationIntent[]
}
```

Intents are unique and ordered by producing occurrence. Optional chord and
angular tolerances are finite and positive. Optional segment bounds are
integers of at least three and `maxSegments >= minSegments` when both exist.
Binding `programHash` prevents an occurrence-index policy from being replayed
against a different program.

For `legacy/current`, `tessellationIntents` must be empty. Quality-dependent
polygon counts and extrusion slices are already materialized in polygonal SPC1
nodes and therefore affect `programHash`. Raw preview/full labels do not.

For `openscad-viewer/brep-1`, analytic nodes enter SPC1 while authored
`$fn/$fa/$fs` intent enters TSP1 and does not affect topology `programHash`.
The eventual resolved preview/export tessellation policy also contains product
preset and hard-cap inputs; it must be separately and canonically attested
before `MeshAssetId` construction. TSP1 must not be presented as proof of a
future resolved policy that it does not encode.

## Matrix and numeric rules

All semantic numbers are IEEE-754 binary64. NaN, positive or negative
infinity, and encoded negative zero are invalid. A lowerer canonicalizes `-0`
to `+0` before validation; decoders reject `-0` rather than silently rewriting
untrusted bytes. Finite subnormal values are preserved bitwise. Integer fields
must additionally be safe integers within their field limits.

Matrices are finite column-major 4x4 values. The affine final row is represented
by indices `3, 7, 11, 15` and must be exactly `[0, 0, 0, 1]`; translation is at
indices `12, 13, 14`. Transform composition is `parent × local`. Reflection is
permitted.

The language contracts differ only where legacy compatibility requires it:

- `legacy/current` permits a finite affine singular `multmatrix`, because the
  pinned direct evaluator accepts it. A zero-normal `mirror()` is distinct: its
  lowered singular matrix must construct the pinned legacy kernel-empty handle
  and return runtime-only `materialized-empty`, rather than being reinterpreted
  as `multmatrix` or collapsed to mathematical `empty`. The validated core-only
  producer projection above is the dispatch discriminant; the subsequent exact
  kernel `isEmpty` result supplies the runtime tag, and source or span
  inspection is forbidden.
  A zero component supplied through `scale()` remains a positioned language
  error under its existing rule.
  This rule does not transfer to `linear_extrude(scale=...)`: the pinned
  evaluator permits a zero top-scale component, including `[0, 0]`, and its
  materialized legacy result must remain representable in SPC1. Likewise,
  `rotate_extrude(angle=...)` must preserve the pinned evaluator's finite-angle
  domain and exact result; v1 must not introduce an unqualified `(0, 360]`
  restriction because the legacy oracle accepts zero, negative, and values
  above 360 degrees.
- `openscad-viewer/brep-1` requires a nonsingular linear 3x3 part before a
  transform can preserve its declared geometry kind, space, representation,
  and evidence. The determinant algorithm and operation order must be frozen
  in native/TypeScript vectors before this check is used cross-runtime; a
  backend must not make an unversioned, tolerance-dependent reinterpretation.
  A collapsed `linear_extrude` top is a separate apex/singularity case, not an
  affine-transform determinant failure; support or capability refusal must be
  explicit and versioned.

Changing these rules under `legacy/current` is forbidden. If the implementation
chooses one uniform nonsingular rule, it requires a new language contract and
cannot pass the v5 pinned-oracle gate.

## Canonical binary grammar

All three frames begin with:

```text
magic[4]
major       u16 big-endian
minor       u16 big-endian
body_length u32 big-endian
body[body_length]
```

The body is a canonical tagged JSON-value encoding:

| Tag | Value | Body |
| --- | --- | --- |
| 0 | null | none |
| 1 | false | none |
| 2 | true | none |
| 3 | number | finite canonical f64 little-endian |
| 4 | string | u32 big-endian byte length + strict shortest-form UTF-8 |
| 5 | array | u32 big-endian count + ordered values |
| 6 | object | u32 big-endian count + key/value pairs |

Object keys are untagged length-prefixed strict UTF-8 and occur once in
strict raw-byte lexicographic order. Arrays retain authored order. The frame
length must match the exact remaining bytes; trailing bytes are forbidden.
Counts, lengths, depth, value count, and the 64 MiB frame ceiling are checked
before proportional allocation. The aggregate raw UTF-8 bytes of object keys
and string values in one frame are capped at 16 MiB. This is one symmetric
wire-domain limit: validation/encoding must reject a value before emitting a
frame that the decoder would reject under that limit. Decode is followed by
exact schema validation and canonical re-encoding; different re-encoded bytes
are rejected.

## Hash boundary and inclusion matrix

All hashes are SHA-256. Domain preimages use a big-endian u32 length before the
domain and before the payload:

```text
programHash = SHA-256(
  length-prefix("semantic-program-core-v1"),
  length-prefix(exact SPC1 bytes)
)

tessellationPolicyHash = SHA-256(
  length-prefix("tessellation-policy-v1"),
  length-prefix(exact TSP1 bytes)
)
```

`programHash` is the only semantic-program hash name. Auxiliary raw
`coreBytesSha256` and `envelopeBytesSha256` are transport/integrity evidence,
not aliases for `programHash`.

| Field or state | SPC1 / `programHash` | SPE1 | TSP1 | External execution evidence |
| --- | --- | --- | --- | --- |
| schema/features/identity version | yes | embeds core | no | version references |
| language contract and semantics revision | yes | embeds core | bound through `programHash` | yes |
| units/frame/matrix convention | yes | embeds core | no | no |
| operations, structural paths, ambiguity evidence | yes | embeds core | no | no |
| occurrences, runtime/static parents, dynamic slots, output ordinals, v2 IDs | yes | embeds core | keyed by occurrence | no |
| ordered typed DAG and exact root cardinality/order | yes | embeds core | no | no |
| execution version/order, discarded effects, planned terminal and prefix frontier | yes | embeds core | no | reached terminal/cleanup evidence |
| node geometry kind, space, representation, evidence requirement | yes | embeds core | bound through `programHash` | provider must match |
| certified profile, canonical policy payload/hash, and explicit conversion node | yes | embeds core | bound through `programHash` | provider must match |
| output colors | yes | embeds core | no | no |
| declared capabilities and exact closure | yes | embeds core | no | yes in v6 |
| diagnostic template code/severity/arguments/order | yes | embeds core | no | no |
| exact source hash and source lengths | no | yes | no | yes |
| source spans, provenance labels | no | yes | no | source-bound evidence only |
| rendered diagnostic message/span | no | yes | no | bounded result evidence |
| B-rep authored tessellation intent | no | yes | yes | policy hash |
| legacy materialized segments/slices | yes, as node fields | embeds core | must be empty | no |
| raw quality/purpose label | no | no | no | yes |
| resolved preview/export tessellation policy | no | no | future resolved policy | policy evidence |
| runtime empty/value/materialized-empty tag and actual certificate ID/bytes | no | no | no | topology/execution evidence; materialized-empty is legacy-internal only |
| execution limit, deadline, cancellation, work evidence | no | no | no | yes |
| qualified lowerer fingerprint and authenticated source-to-program statement | no | no | no | yes |
| queue/cache/publication/latest-only state | no | no | no | separate policy only |
| engine key, kernel fingerprint, manifest, tolerance evidence | no | no | no | yes |

Whitespace-only source changes can therefore change `sourceHash`, SPE1 bytes,
and spans while retaining identical SPC1 bytes and `programHash`. Such reuse
must rebind the current source envelope; it must never reuse prior display
spans or messages.

## Collision, cache, and derived-value policy

A cache entry stores the canonical bytes alongside every digest and revalidates
both on read. If equal typed digests are accompanied by different canonical
bytes, the event is a fatal typed collision: quarantine the entry/provider,
emit bounded evidence, and do not overwrite, alias, fall back, or reuse either
value. Corruption or ordinary validation mismatch discards the expendable
cache and rebuilds from exact source.

`programHash` alone is sufficient only for source-independent semantic-core
reuse. Source maps, diagnostics, and provenance require at least
`(sourceHash, programHash, semantic-program version)`. Backend-value reuse also
requires language contract, kernel fingerprint, capability-manifest version,
tolerance/evidence policy, and compatible saved work bounds.

The current `asset:v1:*` geometry-buffer ID is a legacy scene-local identifier
and is never accepted as `MeshAssetId`. Derived B-rep and mesh identities remain
domain separated:

```text
TopologySnapshotId = H(
  "topology-snapshot-v1", programHash, languageContract,
  kernelFingerprint, capabilityManifestVersion, topologySchemaVersion,
  toleranceEvidencePolicyHash, canonicalTopologyBytes
)

MeshAssetId = H(
  "mesh-asset-v1", TopologySnapshotId, tessellationPolicyHash,
  meshPacketSchemaVersion, canonicalMeshPacketBytes
)
```

`canonicalTopologyBytes` includes the published geometry kind, representation,
and verified runtime evidence/certificate references. A certificate store keeps
canonical certificate bytes beside each `CertificateId` and applies the same
typed collision rule. A derived mesh never overwrites those source-value axes;
its Mesh representation is described by the mesh packet and provenance from
`TopologySnapshotId`.

## Versioning and migration

Unknown future major versions and unknown required features fail closed. Minor
versions are additive only when an older consumer can determine from required
feature IDs whether it can execute the value. A semantic reinterpretation,
changed ordering rule, changed capability implication, changed matrix rule, or
changed hash projection requires a version change.

Current SPC1/SPE1 uses schema 1.2, requires the sorted feature
`semantic.execution-v2`, and retains the legacy semantics revision `1.0.0`
because the language behavior did not change. TSP remains schema 1.0. The
normal object validators, SPC/SPE frame decoders and public normalizers reject
schema 1.0/1.1 and flat-v0 with `E_SEMANTIC_RELOWER_REQUIRED`; they do not
synthesize order, effects, terminal, occurrence or diagnostic defaults.

An upgrade requires fresh exact-source lowering under the current trusted
lowerer and therefore creates new canonical bytes, `programHash` and
attestation. It is not an archival claim about what an older evaluator executed.
Byte patching and structural old-to-1.2 execution migration are forbidden.
Unknown shapes outside the explicitly recognized pre-1.2 cases continue to fail
closed under the normal schema/version/migration diagnostics.

The pre-freeze `dimension`-only SPC1 draft has never been accepted or persisted
and is replaced in place by this candidate v1 contract; accepting both shapes
under one version is forbidden.

Historical direct-evaluator builds never contained an executable SPC1 core.
They remain `historical-unattested`/`not-produced` for program identity. A new
hash can be recorded only after a new lowering/build; migration must not
compile old source with a new frontend and claim that result was the historical
program.

## Protocol v5 and pinned-oracle boundary

Worker protocol v5 continues to carry exact source and `sourceSha256`, the
legacy execution descriptor, and the current mesh/scene payload. SPC1/SPE1 or
`programHash` must not be smuggled into v5 as ignored extra fields. Protocol v6
adds them atomically with exact-key request/event validation,
`GeometrySceneV2`, capability closure, and v2 identity evidence.

The v5 adapter does not add representation/evidence fields to old payloads.
Internally it maps pinned 2D/3D legacy values to the conservative Region/Mesh
and SolidSet/Mesh descriptors above, then emits the exact old mesh/scene bytes.
Protocol v6 publishes the four axes and runtime empty/evidence discriminants
atomically; partial exposure through optional v5 fields is forbidden.

During G1 the shared lowerer and executor may run internally in Worker and
Node/MCP. The plan path never calls or consults another evaluator. Its exact
order/terminal gates use frozen source-to-event fixtures, an independent schema
validator and a recording/fault-injected kernel port. The legacy production
provider remains separately routed until the adapter achieves the frozen
qualification contract. This is not permission to switch the frozen v5
publisher. While v5 retains its current execution descriptor,
`parseOpenSCAD` remains the sole production-v5 publisher; the plan path is a
test/dev/CI shadow whose result is never placed in a v5 response. The
production provider boundary becomes
`build(validated core, request policy, control)` only in the versioned v6
activation described below and cannot accept source.

The adapter must preserve protocol-v5 observable behavior: success/failure,
diagnostic codes/order/spans, warnings, preview/full reduction, exact mesh and
scene bytes, colors, legacy operation/entity IDs, provenance, metrics, exports,
cancellation, queue behavior, and MCP behavior. A newly qualified Manifold
plan provider receives a new immutable manifest/version at v6 activation; the
archived `legacy-direct-evaluator-v1` manifest is not mutated, reused or
reported to attest the new input contract. Advertising the new manifest in v5
would itself change a frozen v5 observable. Doing so requires a separate
versioned migration ADR and is outside G1.

Protocol-v5 shape stability does not waive its existing bounds. The pinned
direct evaluator can produce a 257-character entity identity for a valid small
source while v5 admits at most 256 characters. This outcome is explicitly
`worker-level-unavailable`, not a successful differential fixture. Before any
runtime shadow, a Worker harness must freeze one bounded existing-shape failed
terminal for oversized operation/entity/instance identities and produce it
before a success event crosses the Worker boundary. Truncation, lossy hashing,
optional v5 fields and allowing the main thread to discover an invalid success
are forbidden. The 256/257 boundary and a subsequent healthy job are goldens
for both direct and plan paths.

MCP cancellation is a separate hard boundary, not evidence from a cooperative
`AbortSignal` unit test. Today a synchronous in-process Manifold call can block
the stdio event loop before the cancellation notification is dispatched. No
runtime MCP shadow is safe while that remains true: even an out-of-band result
can deny service to primary requests. Qualification requires a supervised
provider child/Worker, bounded deadline termination plus join, a responsive
stdio/admission parent, one terminal outcome, no late persistence/publication,
and a successful following request. Hard kill never authorizes cross-engine
fallback or an unpinned retry.

### Normative Manifold plan boundary

The proposed plan backend implements the session/unique-payload-lease ABI
above as `SemanticProgramBackend<ManifoldPayloadFamily>`. Its closed payload
family has exactly two admitted carrier keys:

```text
Region/d2/Mesh/RepresentationPreserving   -> opaque session region handle
SolidSet/d3/Mesh/RepresentationPreserving -> opaque session solid handle
```

The backend receives one materialized semantic node at a time, borrowed input
values, the exact carrier key, program/language identity, effective limits and
the execution signal. Its parameter surface, payloads and import closure must
not contain source text, compiler AST, parser results, Worker/MCP protocol
packets or public Manifold types. It returns typed refusal for every analytic,
certified, RationalBrep or other unlisted carrier. Empty algebra remains owned
by the shared executor and never becomes an opaque Manifold empty payload.

The mandatory dependency graph is:

```text
source -> qualified lowerer -> trusted SPE1/SPC1 -> shared executor
                                                     |
                                                     v
                                           ManifoldPlanBackend
                                                     |
                                                     v
                                      narrow ManifoldKernelOps port

owning execution result + trusted core/provenance -> LegacyV5Assembler
                                                   -> neutral legacy result
                                                   -> existing v5 facade
```

Within the plan dependency closure only the qualified lowerer may import
source/compiler facilities. A source-facing v5 orchestrator may transport the
request and invoke the lowerer, but it must not consult another evaluator or
pass source across the plan seam. Core and executor import neither compiler nor
Manifold.
`ManifoldPlanBackend` imports only semantic ABI types and a narrow opaque
kernel port. Exactly one runtime adapter may import `manifold-3d`, and no
Manifold type crosses that adapter's public boundary. `LegacyV5Assembler` may
consume the trusted core, SPE1 provenance, request policy and an owning
execution result while its result lease is live; it may not receive source or
import compiler/parser. It performs mesh extraction, normals, metrics,
BVH/semantic-edge construction and neutral legacy-result assembly, then the
caller awaits `result.dispose()` in `finally`. It does not import Worker/MCP
protocol types; the existing source-facing facade adds the unchanged v5
envelope and execution descriptor. Syntax/lowering failures occur before
`begin` and are mapped from structured lowerer diagnostics by that facade.
Deterministic runtime language failures instead become the versioned planned
terminal above: the facade renders them from SPE1 only after the kernel prefix
has completed and the session has closed. Lowerer warnings are never published
speculatively and are merged only on the corresponding reached success path.
Neither diagnostic path grants source access to the backend or assembler.

The operation map is closed and exhaustive for only the admitted
`legacy/current` polygonal constructors, affine transforms, Boolean/hull,
linear/rotate polygonal extrusion, projection and offset. Values such as
segments, slices, tessellation policy and loop/control results are already
materialized by the lowerer. Legacy winding/fill, cone/mirror, centering,
operation ordering and Manifold original-ID behavior are versioned
compatibility rules; the backend must not rediscover them by parsing source.
For transform, the backend consumes the validated core-only producer projection
defined above. Zero-normal `mirror` constructs the pinned empty kernel handle,
which is then returned as `materialized-empty`, while a bitwise-equal singular
`multmatrix` calls the ordinary transform operation and is `value` in the
frozen golden. Validator/executor prevents an ambiguous or core-mismatched
producer from reaching `evaluate`; the backend rejects null/non-transform names
before allocation. Producer chooses the authored operation rule; it never
substitutes for the handle's exact emptiness check.

The backend also executes every node in
`program.core.execution.evaluationOrder`, including the exact roots named by
`execution.discardedEffects`. Those effect values are never assembler
inputs or scene outputs and are never retained by commit-close, but their
kernel failure, cancellation and allocation order remain observable. Skipping
them because they are unreachable from the published result is forbidden. A
kernel-empty intermediate remains a leased `materialized-empty` operand through
the exact cutter union or other downstream effect work; only the completed
effect root is omitted from retention.

If `execution.terminal` is present, executor success through the final scheduled
node is not a commit. The executor failure-closes the session with the bounded
planned-language code, releases prefix/materialized-empty leases exactly
once, and then throws a structured terminal reference. The source-facing facade
renders its one SPE1 diagnostic; `LegacyV5Assembler` is never entered. Any
kernel failure, cancellation or lifecycle failure reached earlier selects its
own close path and suppresses the later language terminal.

Protocol v5 identity uses a separate, versioned
`LegacyV5CompatibilityMap`. It reconstructs the exact historical structural
operation path, instance path, loop value plus duplicate ordinal, module and
`children()` activation, source span/label, and Manifold-original-ID producer
mapping needed for old `op:...` and `entity:root>...` values. The v1 semantic
`opv1:`, `occv1:` and `entity:v2:` identifiers must not escape into v5. Every
mapping row is pinned by golden fixtures. If trusted SPE1 lacks information
needed for an exact historical value, SPE1 is corrected before freeze or G1
stops; neither backend nor assembler may inspect/reparse source to fill the
gap.

`begin()` synchronously and atomically publishes a session over an already
initialized raw Manifold module. The default qualification facade passes lazy
`defaultBackend` into `evaluateInternal`; source-length validation and complete
lowering finish before module readiness is awaited. Source/syntax/lowering
failure therefore neither loads nor opens the backend, as pinned by regression
vectors. Every native handle
is registered to its session before any yield or throw. Inputs remain borrowed
only for the `evaluate` call. Every payload-bearing `value` or
`materialized-empty` output has a unique session-local lease even when
provider-internal storage is shared. Commit-close reclaims
non-roots and atomically transfers retained roots to the result lease;
failure/abort close quiesces late operations and attempts every registered
handle deletion once. A mapped kernel primary plus cleanup failure leaves the
mapped legacy error first and cleanup error second in one `AggregateError`.
Any indeterminate delete permanently quarantines the backend/module lane;
subsequent `begin()` calls fail `E_MANIFOLD_PLAN_LIFECYCLE` and the raw module is
never reused. The process-global Manifold garbage collector, whose cleanup can
delete another session's handles, is forbidden for this backend. Conformance
requires an explicit per-session handle registry with idempotent exact-once
native deletion, or a disposable isolated Worker/module. Until the runtime
proves independent registries, one shared runtime lane is held from the first
kernel operation through packet assembly and result disposal; separately routed
legacy and plan executions are strictly sequential or use isolated runtimes.

The current G1 shadow-adapter verdict is **FAIL**. Its test-only differential
runner remains useful only for discovery. The qualification-local execution,
discarded-effect, planned-terminal and `materialized-empty` subgates are
**PASS**: the latter retains its 53/53 targeted parity, and the former preserves
kernel-before-language-error precedence with exact cleanup. This does not close
the external boundaries: a valid 257-character direct identity exceeds the
protocol-v5 256-character boundary without a frozen Worker terminal, and Node/MCP has no
hard-killable provider boundary. The historical differential corpus also
discovers fixtures through `parseOpenSCAD` and is not independent qualification
evidence.

The current production-cutover verdict is independently **FAIL**. Protocol v5
continues to publish only the pinned direct evaluator; the local compatibility
subgate neither inherits its manifest nor authorizes a provider switch. A
production plan activation still requires the versioned v6 manifest and every
qualification gate below.

After those blockers close, G1 integration has three non-publishing phases: a
test-only differential runner; a bounded low-priority runtime shadow after the
primary direct result is fixed; and qualification shadow over the frozen
browser/MCP corpus. Browser primary and shadow use separate Workers. MCP keeps
stdio/admission in the parent and executes shadow kernel work in a supervised
child/Worker which a deadline can terminate and join. Direct and shadow runs
never overlap in one global registry. Shadow timeout, hard kill, panic, invalid
lease, cleanup failure or diff is recorded only in an out-of-band comparison
artifact and cannot alter primary success/error, timings, queue state, history,
export or MCP publication. Rollback merely disables the shadow flag. No
route/provider manifest changes in v5.

Syntax, decode, trust and internal lowering failures before a complete plan have
`sourceHash` but no fabricated `programHash`. A captured deterministic language
terminal is different: it belongs to a fully validated prefix plan and its core
error template/order therefore has a real `programHash`. Backend failures after
successful planning may retain that attested identity. Neither cancellation nor
stale publication invents an identity for a core that was not completed and
validated.

## Mandatory implementation delta before acceptance

The tree has a partial, actively converging migration. The core and production
validator already use four-axis `valueType`, the 96-character certificate
profile bound, exact closure policy/transition capabilities, both parent roles,
and nearest/canonical parent checks. The executor already snapshots exact
descriptor keys, applies the shared empty reductions, admits
`materialized-empty` only for `legacy/current`, and requires the process-local
trusted-lowering brand. The lowerer's internal shape carrier
still collapses its descriptor to `dimension`. The validator now contains the
closed group/frontier/materializer pass, structural compiler-frame recognition,
partitioned difference, distinct ownership, and synthetic program frontier;
the lowerer emits the corresponding aliases/reducers and the positive nested
`children()` case. The new continuation check is not yet the complete tuple
above: its call/body, lexical-containment, parameter-slot, and sole-child checks
must be retained, while expanded-subtree membership and static-root reparenting
still need enforcement and independent negatives. The older local root-
lineage heuristic also remains beside the recomputed proof. Flat-v0 still
constructs a flat producer list, and the independent reference/golden vectors
lag the production profile. The candidate cannot be accepted, used as a B-rep
qualification boundary, or advertised through protocol v6 until every
remaining delta below lands together. Keeping both serialized shapes,
supplying descriptor defaults during decode, or inferring evidence from an
opaque backend payload is explicitly non-conforming.

The tree now also contains a qualification-only `ManifoldPlanBackend`, explicit
kernel handles/leases, a legacy assembler and a local direct differential
corpus. Its core-only producer correctly preserves the zero-mirror versus
singular-`multmatrix` distinction. Its explicit runtime-only
`materialized-empty` preserves kernel-empty handles through downstream
operations and passes the current 53/53 targeted parity corpus; that local ABI
subgate is accepted. The producer PASS applies to the trusted executor path;
validator/executor ambiguity/mismatch negatives and backend-local
null/non-transform-name refusal remain mandatory conformance rows. The adapter
passes the qualification-local discarded-cutter, authored-schedule and deferred-
terminal behavioral contract: eager cutter work is scheduled, the production
and independent validators reject order/owner/effect mutations, interrupted
occurrence and `$assign` are represented, partial post-child/map prefixes are
admitted, and an earlier kernel failure suppresses the captured language
terminal. Structural active-subtree validation does not prove the source's exact
deepest/first error; normal execution remains limited to the exact process-local
trusted-lowerer object. The targeted semantic/reference/Manifold-plan run is
`250/250` green, but the adapter as a whole is still not qualified. The Manifold
parity corpus calls `parseOpenSCAD` in `beforeAll`, so it remains differential
discovery rather than independent qualification evidence. The green local corpus
cannot settle the Worker-v5 257-character identity boundary or MCP hard
cancellation. These are current blockers, not future test ideas.

1. `src/core/semanticProgram.ts` must retain its current mandatory four-axis
   SPC shape, exact `staticParent` and four-field
   `deriveSemanticOccurrenceId` preimage, and current emission of
   `evidence.policy.sha256.<certificatePolicyHash>` plus explicit axis-transition
   capabilities. `SemanticDimension` is confined to flat-v0/protocol-v5
   compatibility and must disappear from the lowerer's semantic shape carrier;
   no SPC-facing helper may reintroduce a dimension-only alias. The pre-freeze
   core emits schema minor 1.2, requires `semantic.execution-v2`, adds the
   exact-key `execution` object above, and admits an `error` template only with
   a terminal. Terminal occurrence and compiler-owned `$assign` rows are
   mandatory v2 structure. Every execution field enters canonical bytes and
   `programHash`; TSP remains schema 1.0.
2. `src/services/semanticProgramValidator.ts` must retain its current 96-byte
   profile bound, exact-key descriptor checks, pre-1.2/flat-v0 re-lower refusal,
   closed representation/evidence table, and nearest-ancestor/canonical-row
   parent checks and its new bottom-up group/frontier/materializer,
   compiler-frame, difference-bucket, rotation-bucket, ownership, and synthetic
   `programFrontier` proof. It must remove the remaining
   same-root/descendant/node-lineage acceptance path so root identity has one
   authority. Ordinary nearest-static-parent validation stays closed, with a
   separate exact validator for the sole `$expansion` continuation tuple
   `(call, caller body, definition activation, children, expansion)` specified
   above; a category-wide or name-only skip is forbidden. The tuple validator
   must additionally prove lexical containment of `children`, sole runtime
   children for caller body and `children`, equal call/definition parameter
   slots, and membership of every expanded child in the caller expansion
   subtree. It must also reject non-null runtime parents on static-root
   operations except for the tuple's matching module-definition activation.
   The bottom-up proof must retain explicit bounds/memoization for deep valid
   occurrence trees and expanded frontiers. It must separately recompute the
   closed `legacy-difference-cutters` effect frontier, prove its owner is the
   exact zero-output difference group and its root is the exact one-cutter value
   or canonical n-ary cutter union, and reject any other unpublished node or
   occurrence. It must retain exact authored-schedule replay rather than merely
   accepting any dependency-safe order. Terminal validation retains canonical
   interrupted occurrence, active-subtree, partial production, maximal frontier
   and exact materializer-owner checks, while rejecting separate injected
   effects. These structural rules prove a self-consistent trace, not that its
   occurrence is the exact deepest/first error in source; only trusted lowering
   supplies that property. Schema 1.0/1.1 and flat-v0 must continue to fail
   `E_SEMANTIC_RELOWER_REQUIRED` rather than receive synthesized defaults.
3. `src/services/semanticProgramExecutor.ts` must retain its exact-key
   descriptor snapshot, statically carrier-indexed payload family, provider
   payload validation, shared empty reductions, cancellation/deadline
   mediation, and trusted-lowering gate. The exact-key runtime snapshot must
   retain the explicit `materialized-empty` variant only for `legacy/current`,
   require representation-preserving evidence plus a registered unique
   payload lease, and never treat that variant as shared `empty` or normal root
   `value`. A certified result remains fail-closed until an injected trusted
   certificate byte resolver recomputes the frozen content ID and a
   profile-specific verifier validates it; a backend-supplied Boolean or
   identifier syntax is not evidence. The current mandatory
   session/unique-payload-lease ABI must be
   retained and completed as specified above: atomic synchronous `begin`,
   session-owned registration before throw/yield, borrowed inputs, checked
   lease/carrier/payload triples, idempotent `releasePayload`, quiescent atomic
   outcome-bearing `close`, root transfer into a result lease, and an
   idempotent async `SemanticExecutionResult.dispose`. Every post-begin exit,
   including a late resolution after an abort race and a throw after
   commit-close but before return, must follow the stated ownership path.
   Execution must follow authenticated, replay-validated
   `execution.evaluationOrder`, which in v1.2 equals node storage order,
   and must evaluate/release effect-only roots without adding them to public
   outputs or commit retention; a materialized-empty effect operand remains a
   payload-bearing input until that exact effect graph completes. Its per-node
   context must retain the canonical core-only producer projection; a
   transform with an ambiguous/core-mismatched producer fails before backend
   evaluation, and the backend locally refuses a null/non-transform name before
   allocation. After a green prefix, a planned terminal takes the non-commit
   `E_SEMANTIC_LANGUAGE_TERMINAL` close path, retains no root, awaits exact-once
   release, and exposes only a diagnostic reference; any earlier kernel/control
   failure suppresses it. Even an empty prefix opens and failure-closes a
   session so backend-begin failure preserves its earlier legacy position. The
   assembler is unreachable on every terminal branch.
   A close failure never overwrites the selected primary: the executor retains
   it as `cleanupError`, and the source-facing facade exposes both in an ordered
   `AggregateError` where legacy mapping applies.
   Independent truth-table, hostile accessor/prototype/payload/foreign-lease,
   counted-release, late-resolution, close-failure, and concurrent-dispose
   tests must pin these runtime checks.
4. `src/services/openscadSemanticLowerer.ts` must retain its actual shared
   lowering entry point and unforgeable process-local trust brand, replace
   `SemanticShape.dimension` with the complete descriptor, and derive that
   descriptor only from contract, typed inputs, parameters, and the versioned
   transition table. It must emit output rows for every table-defined producing
   operation, including aliases and map outputs; retain zero rows only for the
   exact transparent/non-producing cases; and retain its new first-child
   `difference`, canonical extrusion reducers, internal-node ownership, and
   pruning only for branches the legacy language did not evaluate. For an empty
   difference base it must retain all eagerly evaluated cutter nodes and build
   the exact implicit one/n-ary cutter value as a
   `legacy-difference-cutters` effect, preserving original evaluation order,
   language state and subsequent outputs. It must union a multi-shape base
   immediately before evaluating cutter statements, then evaluate every cutter
   and construct their exact n-ary union even for an empty base. Retaining
   warnings while dropping this kernel work is forbidden. Deterministic runtime
   `evaluationError` becomes a non-published first terminal and canonicalization
   retains every maximal completed node; the lowerer stops all later source
   siblings. It records the canonical interrupted activation, including
   compiler-owned `$assign`, while allowing completed child or partial map
   outputs in the terminal prefix. Planning remains geometry-blind and
   observationally inert, while cancellation/internal faults still throw
   normally. Exact source-error provenance stays process-local and trusted.
   The exact
   `polyhedron(vertices=[], triangles=[])` constructor remains admitted only
   under `legacy/current` so the kernel can create the leased compatibility
   value; every nonempty sub-four-vertex form and every `brep-1` empty form is
   refused. Its 16-element matrices must be constructed through a checked tuple
   helper rather than unchecked array-to-tuple casts.
   The generic occurrence builder must not gain a caller-selectable
   `staticParent` override. A private `children()`-expansion constructor may set
   the caller-body binding only after checking the registered call/body/
   expansion tuple, the active call/definition pair, the exact `children`
   parent, and matching `$index`; it then derives `occv1` from both anchors.
5. `src/services/semanticProgramCodec.ts` can retain its generic canonical
   value framing only after golden vectors prove exact round-trip and hash
   inclusion of every descriptor/evidence and both parent fields. The newly
   value-typed `tests/support/referenceSemanticProgramV1.ts` must retain its
   separately implemented 96-character profile without importing production
   validation. Its independent authored-schedule, active-subtree, partial-map,
   sibling-order, null-owner, forged-effect and continuation vectors are
   mandatory regression evidence. Semantic fixtures and executor tests must add
   the remaining ownership/continuation attacks, certificate verification,
   trust-state, and empty-algebra rules listed below. SPE1/SPC1 length/hash
   goldens are freeze inputs and require deliberate repinning after the
   output-group/continuation correction and independent review. Migration
   goldens must additionally cover the actual schema-1.2 execution object,
   error-template/SPE mapping, and `E_SEMANTIC_RELOWER_REQUIRED` for schema
   1.0/1.1 object/frame and flat-v0 inputs. Fresh exact-source re-lowering
   receives a new hash/attestation; no wire-level or structural migration may
   synthesize v2 execution evidence.
   Order/precedence tests use frozen event traces, a recording/fault-injected
   fake kernel and the independent validator, never a runtime direct-evaluator
   consultation.
6. The Manifold plan delta must be split into a source-free
   `ManifoldPlanBackend`, opaque `ManifoldKernelOps` adapter,
   `LegacyV5CompatibilityMap`, owning `LegacyV5Assembler`, and a non-publishing
   differential/shadow runner. The backend consumes only the conservative
   `Region/d2/Mesh/RepresentationPreserving` and
   `SolidSet/d3/Mesh/RepresentationPreserving` descriptors through the exact
   session/lease ABI; neither backend, kernel adapter nor assembler may receive
   or reparse source. The adapter must replace process-global garbage
   collection with exact-once per-session handle ownership or isolated runtime
   disposal. Any indeterminate native delete permanently quarantines the entire
   backend/raw-module lane; releasing the session-busy flag must not make it
   reusable, and every future `begin()` must fail
   `E_MANIFOLD_PLAN_LIFECYCLE` with the original quarantine cause.
   The assembler must reproduce exact protocol-v5 bytes and historical IDs
   while the result lease is alive, omit a `materialized-empty` only at root
   mesh publication (without erasing the pinned top-level-2D warning), and then
   await disposal on every path. The internal tag never enters protocol v5.
   A planned language terminal bypasses the assembler entirely and is rendered
   by the source-facing facade only after the executor's cleanup boundary.
   Shadow stays out-of-band and cannot affect a v5 observable. It remains
   unqualified until discarded-effect parity, the 256/257 Worker terminal and
   supervised MCP hard-cancel/join gates are green. Every B-rep
   adapter must return the declared descriptor and evidence, plus a verifiable
   certificate for any certified result. Protocol v5 exposes none of these
   fields; protocol v6 introduces both parent roles, all four value axes, its
   public `empty|value` result union, and the new immutable plan-provider
   manifest atomically. The legacy-only `materialized-empty` sentinel remains
   executor-internal and is normalized solely by the owning legacy assembler.
7. A native Rust implementation and independent fixtures must cover every
   allowed descriptor, forbidden cross-product, current transition,
   typed-empty rule, evidence mutation, coercion refusal, v0 re-lower refusal,
   runtime/static-parent mutation and reparent attempt, execution schedule,
   effect/terminal/prefix mutation, and TypeScript/Rust canonical-byte/hash
   vector before the freeze review can pass.

Because SPC1 v1 is still proposed and has no persisted accepted encoding, this
is a pre-freeze minor-1.2 replacement with deliberate repinning; accepting the
old 1.0/1.1 execution shapes beside it is forbidden. After acceptance, changing an axis,
transition, evidence rule, empty algebra, or hash inclusion requires an
admitted feature or schema-version change rather than a permissive shim.

## Acceptance gates

G0.4/G0.5 are complete only when all of the following are checked in and green:

- exact IDL fixtures covering every node/result/identity variant;
- value-type fixtures covering every allowed and forbidden kind/space,
  representation/evidence combination and every current node transition;
- typed runtime empty/value truth tables for Boolean, hull, transform,
  extrusion, revolve, projection, and offset in both providers, plus the
  separate legacy-only `materialized-empty` rows which prove it is never
  selected by shared empty algebra;
- the complete backend-session lifecycle matrix above in both providers and a
  counted fake provider, including allocation followed by throw-before-return,
  invalid/foreign leases, a late value after abort/deadline, failure at every
  later node/callback/snapshot boundary, atomic commit-close failure, guarded
  post-commit construction failure, shared internal payloads under independent
  leases, empty/no-op results, retained and effect-only materialized-empty
  roots, empty-prefix and nonempty-prefix planned language terminals, earlier
  kernel failure suppression, and concurrent/repeated disposal with zero live
  allocations after every determinate awaited terminal path; indeterminate
  delete paths instead require permanent module quarantine and future-begin
  refusal;
- negative coercion vectors proving that wire fill, component collapse,
  B-rep/mesh conversion, mixed representations, and evidence upgrade never
  occur without their explicit admitted node/capability;
- reserved-type injection vectors proving that a future Curve/Wire/Sheet or
  RationalBrep producer cannot enter any pre-existing consumer signature until
  the same recognized feature explicitly admits that transition;
- re-lower refusal fixtures proving that schema 1.0/1.1 objects and SPC/SPE
  frames plus flat-v0 fail `E_SEMANTIC_RELOWER_REQUIRED`; none may synthesize
  order, effects, terminal, occurrence or diagnostic defaults, and only fresh
  exact-source lowering may mint a new 1.2 hash/attestation;
- SPE1 round-trip and independent SPC1/TSP1 decode/re-encode/hash vectors;
- trust-state vectors proving that a structurally valid SPC1, a structurally
  valid SPE1, and even an exact-source-bound SPE1 cannot call the normal source
  execution/publication path; copied branded objects fail; fresh receiver-side
  source lowering mints a new trusted object rather than upgrading decoded
  bytes; and mutations of the source/core/lowerer binding invalidate
  authenticated cross-process evidence;
- boundary-size fixtures proving every successfully encoded frame is accepted
  by the matching decoder under the same frame/string/value budgets;
- an independent validator that imports no production schema, codec, identity,
  closure, or SHA implementation;
- native Rust and TypeScript agreement on canonical bytes, every public hash,
  Unicode, binary64, matrix, identity, and closure vectors;
- one-invariant mutations for exact keys, all tags, bounds, numeric classes,
  UTF-8, spans, DAG order/sharing, result cardinality, operation/occurrence
  ordinals, ambiguity evidence, execution version/order/effects/terminal/
  prefix frontier, closure, and all hash-domain swaps;
- runtime/static-parent vectors covering direct nesting, module/control frame
  skips, `children()` expansion, recursion with repeated static operations,
  canonical multi-output parent references, cross-branch reparent attempts,
  nearest-ancestor violations, and mutation of either parent ID in `occv1`;
- positive continuation vectors for `children()` below transform, Boolean, and
  nested module frames, multiple `children()` sites, indexed selection, and
  recursive activations; each must keep expanded values in the enclosing
  runtime frontier/bucket and derive distinct occurrence/scene identities;
- negative continuation vectors where an ordinary row skips a transform, an
  expansion's runtime parent is not the exact `children` group, its
  `staticParent` is an outer/sibling or non-nearest caller body, call and
  definition names/parameter slots disagree, the definition is not the sole
  child of that body, `$index` differs, a fake name/path kind claims compiler
  status, an unrelated static-root operation is injected below the expansion,
  or an expanded runtime child belongs to a different call site's static
  subtree;
- scene-lineage vectors covering nested transforms, admitted same-node aliases,
  node-null expansion frames, shared DAG nodes in sibling branches, and proof
  that a common runtime root alone never authorizes identity donation;
- independent production-grammar vectors covering every exact operation
  name/category pair, zero/one/many cardinality branch, preserve/own identity
  result, and compiler-reserved name rejection;
- adversarial ownership vectors where `cube` claims a Boolean node, two sibling
  primitives claim one constructor node, an alias points at a sibling-owned
  node, a transform consumes a non-frontier sibling, or a root names a valid
  node row other than its recomputed emitter;
- synthetic-program-frontier vectors where a result omits a top-level output,
  appends an inner descendant/alias as a second root, duplicates a valid root,
  or substitutes a visited but non-frontier producing row;
- ordered-frontier vectors for Boolean input reorder/omission/duplication,
  node-null non-control/module descendant smuggling, multi-output row swaps,
  duplicate roots, ordinal gaps, and unequal per-map parameters;
- `difference` and extrusion vectors for first-child partition loss, cutter/base
  swaps, base-union movement after the first cutter, an empty first bucket
  followed by non-empty cutters, splitting repeated runtime loop activations of
  one authored statement into separate buckets, skipped one-cutter effects,
  nested/binary instead of exact n-ary implicit unions, unattested reducer
  nodes, discarded geometry outside the exact effect frontier, and an internal
  reducer incorrectly receiving an occurrence or scene identity;
- expansion-frontier vectors proving that following `$expansion.staticParent`
  never bypasses an enclosing transform/Boolean, creates a second production
  path, duplicates the expanded node, or moves it from its `children()` bucket
  into the caller-body bucket;
- bounded parser/lowerer/decoder fuzz with deterministic diagnostics;
- inclusion-matrix tests showing every included field changes `programHash`
  and every excluded field does not;
- cache cross-pair tests proving that equivalent cores never reuse stale source
  spans or display diagnostics;
- legacy tests proving materialized preview changes affect SPC1 only when they
  alter geometry, and B-rep intent changes affect TSP1 but not SPC1;
- singular/reflected/near-singular matrix fixtures proving the contract-specific
  legacy/B-rep rules;
- legacy parity fixtures proving conservative Region/Mesh and SolidSet/Mesh
  descriptors do not alter exact v5 mesh/scene bytes, diagnostics, or IDs;
- producer-context goldens with bitwise-equal singular transform matrices from
  zero-normal `mirror` and `multmatrix`; they prove distinct terminal geometry,
  the exact `materialized-empty` versus `value` runtime tags, inclusion of
  operation/occurrence in `programHash`, no source dependency, and refusal of
  null, ambiguous or wrong transform producers at the responsibility boundary
  above; producer mutation alone may change authored dispatch but must never
  masquerade as an emptiness query;
- materialized-empty conformance vectors proving exact tag keys, bidirectional
  real-kernel classification (`isEmpty=true` gives `materialized-empty`, false
  gives `value`, never backend `empty` after allocation), checked carrier/
  payload/unique lease, downstream authored-order preservation and exact
  Manifold mesh bytes, root-only
  assembler omission, pinned 2D warnings, exact-once release, and absence from
  SPC1/SPE1/TSP1/v5/public-v6 serialization; opaque payload truthiness,
  certified evidence, `brep-1`, or ordinary-value substitution is rejected;
- exact empty-polyhedron fixtures accepting only
  `legacy/current + vertices=[] + triangles=[]`, rejecting every nonempty
  sub-four-vertex form and the exact empty form under `brep-1`; the current
  53/53 targeted parity matrix is a required local subgate, not G1
  qualification;
- independent execution/effect vectors proving
  `execution.evaluationOrder` is complete, topological, equal to node storage
  and reproduced from occurrence production; sibling order-only mutation and
  coherent node-ID/reference renumber that changes the schedule must fail in
  both validators. The only admitted non-publishing roots are recomputed
  `legacy-difference-cutters` or the exact terminal prefix; null primitive owner
  and forged terminal effect also fail independently;
- frozen source-to-event traces and a recording/fault-injected kernel covering
  one/many/mixed-dimension cutters, base union before cutters, kernel-invalid
  constructor/union, warnings, palette/preview state, original-ID equality,
  cancellation and a later sibling, including materialized-empty operands
  retained through the exact implicit union; the harness imports no runtime
  direct evaluator;
- planned-terminal traces for no prefix, one/many prefix roots, nested
  transform/Boolean/module/loop frames, assignment failures, post-child parent
  failures, partial map outputs, a later-sibling terminal forgery, a terminal
  while cutter accumulation is live, and the frozen open-polyhedron-then-
  assertion repro;
  injected kernel failure must win, injected kernel success must release the
  entire non-publishing prefix before the assertion is rendered, and no case
  may call the assembler or emit two terminals. The no-prefix case must still
  begin/failure-close its backend and prove begin failure precedence;
- Manifold import-graph fixtures proving source/compiler/parser/protocol types
  cannot reach `ManifoldPlanBackend` or `LegacyV5Assembler`, public Manifold
  types cannot leave the narrow kernel adapter, and a fake backend never sees
  source;
- a closed Manifold operation/carrier table with positive rows for every
  admitted legacy polygonal operation and typed refusal for every analytic,
  certified, B-rep, wrong-dimension or wrong-representation request;
- golden `LegacyV5CompatibilityMap` vectors for structural operation paths,
  duplicate loop values/ordinals, module and `children()` activations,
  multi-output producers, source spans/labels, original IDs, and exact old
  `op:...`/`entity:root>...` values; mutation of any anchor must be detected;
- Manifold registry fixtures proving zero live handles and exact-once deletion
  after every determinate success, failure, abort/deadline with late resolution,
  invalid/foreign lease, planned-language failure-close, commit-close failure,
  assembler failure, and repeated/concurrent result disposal. Faulted native
  delete fixtures instead prove ordered mapped-primary/cleanup `AggregateError`,
  memoized no-retry disposal, permanent backend/module quarantine, and future
  begin refusal with `E_MANIFOLD_PLAN_LIFECYCLE`; a two-session negative proves
  cleanup cannot cross a session boundary;
- v5 primary-packet goldens and shadow fault injection proving that every
  shadow timeout, cancellation, panic, leak quarantine and comparator diff
  leaves primary bytes, status, diagnostics, timings, queue/history, export and
  MCP response unchanged;
- a real Worker-v5 boundary harness for exactly 256 and 257 characters in each
  legacy operation/entity/instance identity position; the oversized case emits
  the frozen bounded failed terminal before success publication, never truncates
  or aliases an ID, and leaves a following job healthy;
- an MCP non-cooperative kernel harness proving cancellation/deadline hard-kills
  and joins the supervised provider child within its bound while the stdio
  parent stays responsive, publishes/persists exactly one terminal, ignores all
  late output and successfully serves the next request without fallback;
- certified-evidence fixtures with an independent certificate verifier,
  mismatched profile/policy/ID mutations, and proof that indeterminate or
  unverified results never become empty or representation-preserving;
- dense static sibling, duplicate loop value, multi-output occurrence, and
  same-name ambiguity fixtures.

The qualification-local execution/effect/planned-terminal subgate is green, but
the current shadow-adapter gate is still **FAIL** until its independent frozen
source-to-event evidence, 256/257 Worker and supervised MCP hard-cancel rows are
implemented and green. Afterwards G1 shadow
qualification additionally requires 100%
agreement with the predeclared independent event/result fixtures in browser and
MCP, a mutation-sensitive comparator, bounded memory-slope runs, and proof that
separately routed legacy and plan runs are serialized or physically isolated.
The historical differential runner remains discovery evidence, not a fallback
or qualification oracle. This
qualifies only the shadow adapter; it does not authorize a production-v5
switch. Production activation additionally requires protocol v6, its new
immutable provider manifest, exact-key v6 fixtures and an explicit rollback
decision. A request to retain the v5 wire shape while changing its attested
provider is a separate versioned migration, not an exception to this gate.

## Kill conditions

Stop G1 and return to contract review if any of the following occurs:

- browser and MCP produce different SPC1 bytes, closure, identity, diagnostic
  order, or `programHash` for the same admitted source and semantic policy;
- two observably different semantic cores alias to one `programHash`, or one
  semantic core has multiple accepted canonical encodings;
- source-bound spans/messages are reused across different `sourceHash` values;
- commutative operand sorting or structural deduplication changes diagnostics,
  provenance, identity, or exact output;
- same-name positional identity is treated as strong cross-revision identity;
- a sibling/shared-DAG occurrence donates scene identity on the strength of a
  common runtime root or descendant relation without the recomputed production
  and identity-owner proof;
- an occurrence/node pair is accepted without the exact operation production
  rule, a node has zero or multiple recomputed materializer groups, an owned
  edge escapes its ordered frontier, or root producer/identity rows differ from
  the recursively derived rows;
- on a success plan, the result differs in tag, cardinality, order, node,
  producer, or identity from the recomputed synthetic `programFrontier`; on a
  terminal plan, result is not exact bottom/empty or any result is published;
- a node-bearing occurrence uses a null output ordinal, a non-control/module
  zero group leaks descendants into a frontier, or an implicit reduction node
  is given a synthetic scene identity;
- an occurrence omits `staticParent`, binds it to a non-nearest/cross-branch
  row, skips a non-control/non-module frame outside the exact compiler-owned
  `$expansion` continuation tuple, or derives `occv1` without both parent IDs;
- a static-root operation receives a non-null runtime parent outside the exact
  matching module-definition activation tuple;
- a `$expansion` continuation is accepted without its exact call/body/
  definition/children anchors and matching `$index`, or its `staticParent` edge
  affects production frontier, buckets, reachability, or node ownership;
- a provider changes any declared value axis, performs an implicit
  representation/kind/space conversion, upgrades evidence, hides mathematical
  emptiness inside an opaque payload/ordinary `value`, or emits
  `materialized-empty` without `legacy/current`, representation-preserving
  evidence, a checked payload/unique lease and a real kernel emptiness result;
- the Manifold adapter returns shared `empty` after an admitted kernel call has
  produced a handle, or its `isEmpty` false/true classification does not map
  exactly to `value`/`materialized-empty`;
- `materialized-empty` is reduced as shared `empty`, published as an ordinary
  mesh/value, serialized into SPC1/SPE1/TSP1/v5/public-v6 fields, omitted before
  downstream/effect kernel use, or admitted for certified/B-rep execution;
- a backend allocation can exist without a registered session lease, an input
  borrow can outlive `evaluate` without an independently owned output lease,
  cancellation can detach a still-allocating operation from quiescent close,
  commit-close can leave ownership ambiguous, or any successful/failing result
  path can double-free or retain an unreachable payload;
- browser or MCP code consumes a kernel payload after result disposal or drops
  an owning execution result without awaiting its idempotent disposal;
- an ambiguous, unsupported, budget-exhausted, invalid, or uncertified result
  is reported as typed empty;
- legacy tessellation intent escapes SPC1, or B-rep tessellation intent enters
  topology `programHash`;
- a provider receives raw source, reparses it, weakens closure, or chooses a
  different engine;
- a transform backend infers zero-mirror behavior from determinant alone,
  receives producer data not recomputed from authenticated core, accepts a
  missing/ambiguous producer, or a node-only cache aliases mirror and
  multmatrix semantics; producer is treated as proof of emptiness, inherited
  from a child instead of naming the current materializer, or exposed as public
  runtime provenance;
- geometry which the legacy evaluator eagerly computes and discards is pruned
  without an exact effect row and kernel execution, including a frozen expected
  kernel error which becomes plan success, a base union moved after cutter
  evaluation, or a missing/non-n-ary implicit cutter union;
- `execution.evaluationOrder` omits/duplicates/reorders a node, differs from
  node storage, disagrees with the independently replayed production schedule,
  or permits a dependency after its consumer; an effect/prefix node enters
  scene/result retention, or an arbitrary hidden audit node is admitted as an
  effect;
- trusted lowering captures a terminal other than the first deterministic
  language error; the terminal does not name its canonical interrupted
  occurrence; a later row escapes that active subtree; the error template does
  not match operation/errorName/detail hash; the prefix is non-maximal,
  overlapping or injected; partial completed outputs are published; or
  structural replay is claimed to prove source-authored schedule or exact
  deepest/first source error;
- a later planned assertion is exposed before an earlier kernel prefix/effect
  finishes, a green prefix commits roots or calls the assembler before raising
  its terminal, an earlier kernel/control failure does not suppress it, cleanup
  occurs after it is exposed, or two public terminals escape;
- speculative planning publishes warnings/callbacks/history, invokes a kernel,
  observes geometry, or admits a language feature whose control/values depend
  on kernel results; the latter requires stopping G1 and designing a separately
  versioned streaming/chunk-attestation boundary rather than guessing a trace;
- a schema-1.0/1.1 object or SPC/SPE frame, or flat-v0 artifact, receives
  synthesized execution defaults instead of `E_SEMANTIC_RELOWER_REQUIRED`, or
  bypasses fresh exact-source lowering;
- the Manifold backend or v5 assembler imports compiler/parser/protocol code,
  receives source through an alias/callback, or reconstructs a missing legacy
  value by inspecting source;
- process-global Manifold cleanup can delete a handle owned by another live
  session/result, separately routed legacy and plan runs overlap in the same
  global registry, or packet assembly observes a disposed root;
- a mapped kernel primary or its cleanup failure is dropped instead of being
  preserved in ordered `AggregateError`, an indeterminate native delete is
  retried, or a quarantined Manifold backend/raw module serves another session
  instead of refusing every future begin with `E_MANIFOLD_PLAN_LIFECYCLE`;
- a shadow result, failure, timeout or cleanup event changes any primary v5
  observable, triggers engine fallback/retry, or is published through Worker,
  history, export or MCP;
- runtime shadow begins while the 257-character v5 identity has no frozen
  bounded Worker terminal, silently truncates/hashes it, or lets an invalid
  success cross the Worker boundary;
- MCP shadow runs a synchronous kernel in the stdio process, cancellation is
  only cooperative, deadline does not terminate and join its child, or late
  child output can be persisted/published;
- the new plan path reports the archived `legacy-direct-evaluator-v1` manifest,
  or production switches under frozen v5 without a separately versioned
  activation;
- a historical v5 operation/entity/original-ID value is synthesized from a new
  semantic ID without an exact golden compatibility proof;
- a structurally valid, decoded, migrated, copied or merely source-bound
  artifact is executed by the normal path without being the exact object minted
  by the process-local trusted lowerer;
- the plan path calls another evaluator as a fallback, validation aid or runtime
  oracle, or a frozen independent event/result fixture disagrees, including a
  singular-matrix acceptance change under `legacy/current`;
- native and TypeScript canonical bytes/hashes disagree;
- an unknown required feature, future major, malformed UTF-8, noncanonical
  number, cycle, dangling reference, or oversized allocation is accepted;
- current `asset:v1:*` identity is reused as cryptographic `MeshAssetId`;
- two fundamental post-freeze schema redesigns are required.
