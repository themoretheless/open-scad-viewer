# ADR 0002: Permanent source-routed geometry engines

- Status: accepted
- Date: 2026-08-01
- Contract: `geometry-routing-contract-v1`
- Frozen matrix SHA-256: `3bb84d80ad89d9efdcde2ff0d678ab5357bc91312e852fced087427528c1d1c7`

## Decision

The exact source text is the only route authority. The permanent routes are:

| Source language contract | Engine class | Automatic fallback |
| --- | --- | --- |
| absent or `legacy/current` | `manifold` | never |
| `openscad-viewer/brep-1` | `brep` | never |

The route table is defined once by the deeply frozen
`GEOMETRY_ENGINE_ROUTES`. Browser, Worker, MCP and DuckDB validation derive
their engine class from that table. A failure, timeout, cancellation,
quarantine, revocation, rollback or retry must not change the engine class.

`@engine` is forbidden. MCP input objects containing `engine`,
`engine_class`, `backend`, `kernel`, `provider` or `automatic_fallback` are
rejected as invalid input instead of being ignored. A provider cannot return a
facade-owned routing error; doing so quarantines it as a contract violation.

## Leading source header

Directives are lowercase and may occur only in the leading header. There may
be one `@language` and multiple `@requires` directives. Requirements are
validated, duplicate-collapsed and bytewise sorted. The limits are 32 distinct
requirements and 128 characters per identifier. LF, CRLF and CR line endings
are accepted. A leading BOM is accepted. Directive-shaped text inside block
comments or strings is not interpreted. Mixed-case spellings of reserved
directives fail closed.

Source admission rejects malformed Unicode and more than 250,000 UTF-16 code
units before scanning a directive. The normative positive and negative cases
are in
`docs/qualification/geometry-routing-contract-v1.json`. A test-only reference
router has no imports from the production router and must agree with every
frozen case.

## Refusal precedence

After transport/schema admission, conflicts are resolved in this order:

1. invalid source Unicode, size, Worker envelope or source digest;
2. malformed, unknown, duplicate or misplaced routing directives;
3. pre-admission cancellation/staleness and bounded queue admission;
4. missing capabilities in the selected immutable manifest;
5. selected-manifest revocation;
6. deployment and provider presence;
7. provider readiness failure, timeout or quarantine;
8. selected-provider parsing/evaluation/numerical failure;
9. result serialization, export or persistence failure.

Routing, capability and revocation refusals invoke neither `warm()` nor
`build()`. Exact messages, reported contract, line and call counts are frozen
by the qualification matrix. Persisted diagnostics retain a stable public
code, retry class and bounded details. A late `AbortSignal` does not rewrite an
unrelated failure as cancellation.

## Provider admission and identity

Changing `availability` is insufficient to make a provider executable.
Admission evaluates the immutable archived manifest.

There is one narrow grandfathered evaluator exception, represented by an
exact immutable allowlist. `manifold-node-v1` remains the historical catalog
identity with its original digest and dependency evidence;
`manifold-node-v2` is the current package snapshot after the repository lockfile
and notices gained the independent text engine dependency. Both entries bind
the same legacy evaluator key, fingerprint, semantics and target. Browser-safe
core code recomputes the canonical digest over every immutable manifest field
before either allowlist or qualification admission is considered; changing any
field while retaining an archived digest fails integrity admission. Every
other provider requires all of the following:

- production maturity and a deployed isolation mode;
- `qualification.status = qualified` with non-null record and corpus IDs;
- a kernel fingerprint of the form `sha256:<artifact digest>`;
- complete package, version, license, SBOM and lockfile attestation.

The B-rep manifest is `not-deployed`, has no executable capabilities and is
not admissible. Planned capabilities are not qualification evidence.

## Provenance and replay resistance

Every execution descriptor is deeply frozen and records the language route,
requirements, engine identity, kernel fingerprint, semantic-program version,
capability-manifest version, immutable manifest digest, purpose, quality,
representation, effective limits, evidence level and
`automaticFallback=false`.

Worker build requests and every Worker event carry `sourceSha256`, computed
independently on both sides over the exact UTF-8 source. The Coordinator
compares the digest, job/revision envelope, route, requirements and source
spans before publication. Failure offsets must be paired and fit the exact
source; mesh provenance runs are non-empty, sorted and non-overlapping. A
terminal event for different source bytes is a protocol failure even when both
sources select the same route. Because structured clone removes frozen state,
the Coordinator re-clones and freezes the execution descriptor and its owning
event after validation and before publication.

Worker job IDs are monotonically increasing on the FIFO Coordinator channel.
A bounded high-water tombstone rejects active, terminal and out-of-order replay
for the lifetime of that Worker. Routing-header planning precedes stale/cancel
classification, and neither Worker import nor a B-rep/refused request warms
Manifold; only the selected admitted provider may warm.

Protocol v5 was not a released third-party wire boundary before this contract
was frozen. The mandatory source attestation is therefore part of the frozen
v5 profile; a pre-contract Worker lacking it fails the v5 validator instead of
being accepted. The next scene/semantic-program wire remains protocol v6.

New model-backed history is re-attested against its immutable revision. New
inline history stores a bounded immutable source snapshot and re-attests its
digest and route on every read. Schema-v4 migration adds manifest digests to
historical execution JSON. Schema-v5 adds an explicit per-row source evidence
class (`model-revision`, `inline-snapshot`, or `historical-unattested`) with
mutually exclusive invariants; the class is also exposed over MCP. Historical
inline rows whose original source was never stored remain visibly unattested
and cannot silently acquire fabricated source evidence.

New failed and cancelled history uses diagnostic contract v1: a code from the
frozen public taxonomy, its exact retry class, bounded details and any source
position are mandatory. Engine/capability/language refusals retain their route
details and `automatic_fallback=false`. DuckDB rejects unknown diagnostic
fields and rechecks route-critical details against the immutable execution
descriptor both before write and after read. Bounded pre-contract diagnostics
remain readable only as `legacy-unattested`; unknown diagnostic versions fail
closed.

## Retry and revocation

The engine facade performs one provider attempt and no automatic retry.
Readiness is single-flight and a failed readiness state is not retried by
later discovery calls in the same engine instance. The browser watchdog may
create at most one new job after replacing a wedged Worker; source routing is
recomputed from the same source and still cannot cross engine classes.

A future same-engine retry must preserve source digest, engine class/key,
kernel fingerprint, capability manifest and digest, purpose, quality, limits
and policy epoch. Rollback to another fingerprint is a new attested execution,
not a retry.

The runtime accepts host-authenticated revocation records through a
process-local registry. Records have consecutive epochs and an integrity
attestation over their exact fields. Remote signature verification belongs to
the embedding policy authority and must occur before a record reaches this
registry. Revocation is checked before readiness, before provider invocation
and again before result publication. A policy epoch change or revocation while
a build is running suppresses the late success. Re-enabling requires a new
qualified manifest/policy decision; archived history is never rewritten.

## Rollback and last-known-good state

Rollback may select only a compatible manifest of the same engine class.
Missing or revoked B-rep therefore returns a typed unavailable error and never
uses Manifold. A prior mesh may remain visible as last-known-good context, but
an error makes it non-exportable even when its source and quality otherwise
match the editor. Durable LKG restoration must additionally attest source,
revision, engine/manifest, policy epoch and mesh-asset integrity; it is a
rollout gate and is not represented as a successful current execution.

MCP analyze/export never substitute an older artifact. Historical artifacts
remain accessible only through explicit historical resources.

## Consequences and deferred gates

The contract gate does not deploy the Rust engine and does not claim formal
geometry qualification. Hard subprocess/Worker isolation, signed remote
revocation distribution, durable mesh LKG storage and the protocol-v6 scene
ABI remain mandatory before B-rep activation. These are implementation gates
for the runtime/MCP rollout stages, not exceptions to source-only routing.

The contract is continuously checked by:

- the hash-pinned routing matrix and independent reference router;
- exact diagnostic and precedence cases;
- zero-call no-fallback/refusal tests;
- provider admission, readiness and live-revocation tests;
- same-source replay and Worker payload-boundary tests;
- MCP override rejection;
- model-backed and inline DuckDB write/read attestation tests.
