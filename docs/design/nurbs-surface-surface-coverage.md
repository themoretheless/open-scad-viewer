# Пересечение поверхностей: область доказательства

`nurbs_core::ss_intersection::intersect_surface_surface_report` возвращает геометрические компоненты и отдельную область доказательства покрытия. NURBS-0030 остаётся planned: общий алгоритм пока выдаёт кандидатов, а не доказанное полное пересечение.

## Частичные доказанные редукции

- Строго разделённые исходные Cartesian control hulls при положительных весах доказывают пустое пересечение. Допуск не превращает близкие плоскости в совпадающие.
- Буквально одинаковые исходные поверхности имеют одинаковый геометрический образ. Enclosure покрывает весь control hull, а не только углы. Это не доказывает регулярность или однозначность параметризации.
- Координатные прямоугольники первой степени с постоянными весами обрезаются по действительным пространственным границам обоих патчей. Площадное перекрытие и поперечная линия принимаются только с проверенным точно представимым аффинным UV-отображением. Непредставимое отношение, например 1/3, сохраняет неопределённость. Линии короче допуска не удаляются.

Касательные контакты меньшей размерности и общий некоординатный случай остаются неподтверждёнными. Точное совпадение извлечённых округлённых контролов не доказывает совпадения исходных поверхностей. Малый residual Newton-уточнения не является сертификатом Krawczyk. Даже после исчерпания численной очереди неподтверждённый отчёт сохраняет всю исходную область как нерешённое обязательство покрытия.

## Отчёт и адаптеры

`coverage_proof` определяет область геометрического покрытия. `complete()` требует доказанную область и отсутствие нерешённых участков. `topology_proven` — отдельное обязательство владения графом ветвей и trim; производимый API отчёт пока его не подтверждает. Имена старых контейнеров `ExactBranch` и `krawczyk_isolated` сохранены для совместимости; они сами по себе не дают сертификата.

JSON помечает неподтверждённые компоненты и соответствия как `candidate_only`, а счётчик уточнённых начальных точек — `candidateSeeds`. Граф не заявляет однозначную склейку, классификацию всех ячеек или владение полурёбрами. Список UV traces не выдаётся за построенный DCEL. Rust- и JSON-адаптеры B-rep сохраняют запрет топологического авторства; неизвестный proof scope не принимается за полноту.

`verify_ss_coverage` проверяет структурную согласованность отчёта и явно возвращает `semanticProofReplayed=false`. Это не независимое воспроизведение геометрического доказательства. Boolean mutation authority отсутствует.

Регрессии: `crates/nurbs-core/tests/ss_coverage.rs` и `crates/brep-core/src/nurbs_ss_general/tests.rs`. Итоги конкретных запусков фиксируются отдельно; наличие теста не означает его успешный запуск.

Актуальный целевой прогон адаптеров B-rep (`nurbs_ss_general::`, codec, offline/locked) завершился: 7 tests passed, 0 failed. Итоговый integration-прогон NURBS остаётся отдельной проверкой; он также включает полное тело прежнего codec regression общего SS-случая. Это свидетельство для ограниченного query-контракта, а не завершение NURBS-0030.

## Uniform original-source root fibers, 2026-10-16

`ss_intersection::FiberSources::new(&first, &second)` owns both actual original
NURBS definitions in one immutable shared context. `certify(domain, FiberAxis,
FiberLimits)` addresses the exact system F(u,v,s,t)=S1(u,v)-S2(s,t) on a complete
four-parameter product. One coordinate is free; the other three have positive
interval widths. Limits bound all original rectangle evaluations, including
full-product and center/free-parameter queries. No tolerance, continuation seed,
preview mesh or cached report grants root authority.

Every original knot rectangle contributes outward position/Jacobian bounds.
The three-variable center is an interior binary64 proposal. A supplied matrix
or the inverse of a midpoint Jacobian is only a preconditioner proposal. The
actual interval operator is

K = x0 - R F(x0, Lambda) + (I - R J(X,Lambda))(X-x0).

The whole free interval Lambda remains in the center residual, not a sampled
free value. A weighted infinity-norm bound q for I-RJ must be strictly below1,
and K must lie strictly inside X. These conditions prove a unique zero in X
for every free value: the fixed-point map is a contraction mapping the complete
box into itself, and q<1 also implies that R is invertible. The positive result
owns a `QualifiedFiber`; a report flag alone cannot manufacture that owner.
The root image encloses the three nonfree coordinates, in original axis order.

Original world-box separation excludes roots immediately. A Krawczyk image
component disjoint from its unknown interval also excludes roots even when all
world-coordinate residual hulls contain zero. Failed inclusion, contraction,
preconditioning, representation or work leaves `Unresolved`. Invalid domains,
nonfinite proposals and invalid limits remain input errors. Periodic upper
endpoints require actual exact supporting-seam identity; a periodic flag or
approximate storage match is insufficient. The existing native trim seam proof
is reused rather than duplicated.

`reverify` recomputes the original operator and receipt. `at(parameter, limits)`
freshly specializes the same owned original box; it cannot extrapolate beyond
the qualified free interval. Separate original-span rank evidence is recorded
for each support. A valid parameter fiber on a singular support can have a
single physical point image; it is not automatically a transverse world curve.
False rank evidence means unproven and can remain unproven on collapsed point
specializations. Smoothness across C0 knots or periodic seams is not implied.

Tests independently check polynomial and rational roots (including a true
unknown root at1/3), both surface orders, all free-axis proposals, whole knot
census, stored periodic seams, empty correlated boxes, tangencies, partial work,
false preconditioners, changed source coefficients and singular point images.
The actual original surfaces stay owned when caller data changes.

This local proof does not change `SurfaceSurfaceIntersection::complete()` or
its existing coverage scopes. The legacy `krawczykCertified` seed counter is
still not this certificate. General full-domain coverage, branch-atlas
transitions, owned graph/trim topology and source-bound world traces remain
required for NURBS-0030; the catalog stays198/200. Current evidence is in
`docs/qualification/nurbs-ss-fibers-2026-10-16/native-tests.json`.

## Full-product coverage around an owned fiber, 2026-10-16

`cover_fiber_surfaces(&QualifiedFiber, CoverageLimits)` now requests both whole
original natural domains. `cover_fiber_box` accepts an explicit contained product
and reports that narrower scope. A narrower free interval cannot silently become
whole-surface authority: the qualified fiber must cover the entire requested
free interval.

The complement of the three-unknown tube is partitioned into at most six closed
axis-peeling slabs. Every shared interface is included. Original-system position
or Krawczyk exclusion must prove each slab empty; unresolved boxes subdivide
into two complete closed children. All cells, previous split attempts and
original rectangle evaluations share bounded work. Excluded leaves, split
coordinates and their original domains are retained in an owned partition tree.
No failed cell, boundary or unfinished child can be discarded.

A second uniform root in any complement cell produces `AdditionalRoots` and an
owned additional fiber, never a complete single-fiber result. A natural-boundary
root that cannot be isolated by strict inclusion remains unresolved; it is not
removed as an open-domain artifact. Cell, depth and rectangle limits deny
coverage authority when exhausted.

`QualifiedFiberCoverage::reverify` freshly replays the primary fiber, recomputes
the exact original complement slabs, verifies every stored exclusion/split,
checks both child boxes and audits complete reachability and counts. Missing
slabs, changed domains, forged exclusion receipts, orphan nodes and partial work
are rejected. `covers_entire_surfaces` compares the owned domain with both actual
original natural domains; it is distinct from a successful local-box request.

Current independent fixtures prove one full-domain parabola/plane root graph,
reject an omitted second real branch, retain a real natural-boundary branch, and
verify a nonlinear cubic complement that needs64 tree nodes,35 excluded leaves
and190 original rectangle evaluations. The cubic's exact dyadic coefficients
give z(v)=3(v-1/2)((v-1/4)^2+1/64), so its sole real root is independently known.
Depth-zero proof cannot certify the same case. No symbolic recognition of these
surfaces occurs in the algorithm.

This certifies the original four-parameter zero set around one complete uniform
fiber. General multi-fiber/coordinate-chart discovery, physical branch graphs,
source-bound world traces and trim topology remain separate requirements for
NURBS-0030. The entry remains planned; the catalog is198/200. Evidence is in
`docs/qualification/nurbs-ss-fiber-coverage-2026-10-16/native-tests.json`.

## Multi-fiber parameter coverage (2026-10-16)

`cover_fiber_atlas_surfaces` and `cover_fiber_atlas_box` accept one to 64
owned qualified fibers on the same actual ordered surface definitions and free
axis. Every tube must span the complete requested free interval; unknown-box
interiors must be pairwise disjoint. Sequential closed-box subtraction retains
all complement faces and excludes the entire remaining product under one shared
rectangle/cell/depth budget. Fresh replay checks every fiber, recomputes the union
complement, and verifies every exclusion and subdivision node.

The distinct `QualifiedFiberAtlasCoverage` exposes `fibers()`, not a singular
branch accessor. The original `QualifiedFiberCoverage` API remains restricted to
exactly one root graph. Overlapping/duplicate tubes and mixed source definitions
are rejected; partial intervals and insufficient proof work yield no authority.
Parameter-root graphs may have coincident physical images. General chart
transitions, boundary-root admission, owned physical traces and trim graphs are
still separate obligations; NURBS-0030 remains planned.

Two-root tests now qualify both original branches simultaneously. Negative cases
cover duplicate tubes, mixed sources, incomplete second intervals and shared work
exhaustion. Seven coverage tests, 17 related intersection tests, one corruption
replay and three former Boolean failures pass. Both kernels compile for WASM.
These focused results do not establish full native-suite success.


## Overlapping mixed-axis original root covers (2026-10-16)

`cover_mixed_fiber_box` / `cover_mixed_fiber_surfaces` own a complete closed
four-parameter product partition. Each leaf is either entirely contained in a
freshly verified original uniform root fiber, or independently excluded by the
original `S1-S2` interval system. Fibers may overlap, have different free axes
and cover different free intervals. Both children retain their shared closed
face; no root on a chart end or product interface is discarded.

Every original fiber replay and exclusion/split attempt consumes one shared
original rectangle allowance. Cell and depth budgets are separately bounded.
`QualifiedMixedFiberCoverage::reverify` reconstructs all actual original fibers,
exclusions, closed child boxes, visited-node census and report counters. Cached
leaf status, omitted nodes, altered source context or duplicate child IDs cannot
inherit coverage. Missing central charts and missing distinct root branches
remain unresolved. Existing common-axis disjoint-tube atlas behavior is unchanged.

Current regression uses a genuinely curved analytic intersection with an
essential central chart using `SecondU`, while endpoint charts use `FirstU`.
Independent position samples check its formula; completeness comes from the
entire original interval cover. Permuted overlapping charts, exact shared-work
limits and a second omitted branch are also checked.

This proves root-set coverage, not a unique branch decomposition or ownership
of transitions between overlapping charts. Physical world curves, tracing and
trim-graph ownership remain separate. NURBS-0030 stays planned. Current native,
WASM and source-replay evidence: `docs/qualification/nurbs-mixed-fiber-cover-2026-10-16`.


## Original local transitions between parameter-root charts (2026-10-16)

`qualify_fiber_transition` qualifies a whole proposed bridge box inside two
actual source-owned uniform fibers. The bridge has a freshly proved unique root
for every value of the first chart's free parameter. Both originals, the bridge
and its endpoint queries share one original rectangle work allowance.

The validated original positive-weight surface bases are continuous. Uniform
contraction therefore gives a continuous bridge root graph. Since every bridge
root is inside the second original chart, uniqueness there makes its free
coordinate injective along the first free interval. A continuous injective map
of an interval is strictly monotone; disjoint original endpoint root enclosures
prove Forward or Reverse orientation. This establishes a local continuous
coordinate bijection onto its actual image, not onto the entire second chart,
and not a differentiable inverse or complete overlap census. No sampled seed or
rounded inverse parameter formula authorizes the transition.

`QualifiedFiberTransition::reverify` repeats original source/bridge/endpoint
proofs and compares actual source bindings, endpoint images, orientation and
charged work. Damage to cached receipts cannot inherit the transition. Curved
regression uses v=sqrt(.25-u^2/16) with FirstU and SecondV charts; its reverse
orientation, same-axis forward case, shared-work exact limit, absent root overlap
and private cached receipt mutations are covered.

The native fiber verifier now also refuses a closed original product touching an
empty natural first/last knot cell (`original-endpoint-basis-unproven`). An
interior one-sided limit is not substituted for the actual evaluator endpoint.
Valid interior boxes on those original bases remain eligible. Both endpoint
refusals have regressions.

NURBS-0030 remains planned: connected-component/transition census, physical
world traces and owned trim topology are not provided by this local owner.
Evidence: `docs/qualification/nurbs-fiber-transitions-2026-10-16/native-tests.json`.


## Source-owned parameter connected components (2026-10-16)

`connect_mixed_fiber_cover` takes an actual complete `QualifiedMixedFiberCoverage`
and proposed bridge boxes. It freshly replays the entire closed original product
cover, then qualifies every bridge on the actual ordered original surfaces.
Coverage replay and bridges share one original rectangle allowance. Bounded
replay charges actual failed work as well; cached coverage counters are not
accepted as work or geometry authority.

Every uniform original fiber root graph is compact and connected on its closed
free interval. Positive bridges therefore establish connected unions. For EVERY
pair of charts assigned to DIFFERENT components, the constructor additionally
requires strict separation of their original complete four-parameter root
enclosures. There is no inference that unlinked charts are disjoint. The complete
cover makes the union of these graphs exactly the requested original root set;
finite disjoint compact connected unions are its exact connected components.

`QualifiedFiberComponents` owns the coverage, chart-index bridge proposals,
actual transition owners, canonical component chart lists, complete cross-group
separation receipts and charged work. Fresh replay reconstructs all of these,
including every private transition receipt. Missing bridges, ambiguous cross-group
contacts, missing pair witnesses, altered component claims and stale work refuse
ownership. The tests include an essential mixed-axis bridge on a curved source,
two distinct original root branches, omitted links, changed bridge order, exact
shared-work limits and private source-census receipt damage.

This is explicitly a PARAMETER connected-component census. Charts within one
component may have additional intersections or cycles; this API does not claim
simple branch edges, physical-world embedding, periodic-quotient topology or
world traces. Physical source self-overlap can make different parameter components
meet in world space. Those obligations and trim ownership keep NURBS-0030 planned.
Evidence: `docs/qualification/nurbs-fiber-components-2026-10-16/native-tests.json`.


## Specific-root continuous world traces (2026-10-16)

`qualify_fiber_world_trace` binds actual world and two UV NURBS curves to one
specific original uniform intersection fiber. Original source replay retains the
actual verified proposal matrix R, scaling D, contraction upper q<1 and original
complete first-support Jacobian bounds. R is not asserted to be an exact inverse.

Both positive-weight pcurve hulls must lie inside the actual original fiber box.
Their common normalized traversal has an explicit source-free-coordinate clock.
The clock has literal full source endpoints, ordered positive rational B-spline
scalar controls, continuous interior knots and strict progress on every active
block. It is therefore a continuous monotone bijection onto the entire original
free interval; flat spans and incomplete clocks refuse admission. The clock is
not silently treated as the identity parameter.

Whole original `curve_surface_agreement::verify` bounds C-S1(P1) and C-S2(P2).
Its complete cell traversal retains original curve/surface knot spans and shared
faces. With resulting bounds e1,e2, the original residual F at the candidate
parameters has norm <=e1+e2. The actual verified contraction gives
`||root-candidate||scaled <= ||D^-1 R F(candidate)||inf/(1-q)`.
Original first-support derivative bounds then transfer this to continuous world
error, plus e1. An outward-computed internal agreement tolerance reserves margin
inside the requested world tolerance. Original rectangle replay and complete
agreement cells have separate bounded ledgers. On verifier errors with no partial
count, the remaining agreement allowance is reserved rather than reused.

`QualifiedFiberWorldTrace` owns actual source, actual pcurves and world curve,
clock orientation, composition/error bounds and work. Fresh replay reconstructs
all geometry and evidence. A curve lying on both supports but on another original
root branch is refused; proximity alone is not root identity. Private operator
and world-error mutations are also refused.

Tests include a rational curved root with bound <=1e-9, reverse traversal,
multiple source/world/clock knot spans, a genuine C1 piecewise source whose second
derivative changes from .125 to .625, a rotated world frame and a free parameter
on the second support. Agreement-cell exhaustion and flat source clocks cannot
publish partial traces.

The shared-tree `construct_fiber_trace` adaptive constructor proposes
Newton/Hermite coefficients, reuses common endpoint sites and returns a chain
only after every original interval piece passes this source-specific world proof.
Sampling does not authorize the chain. Current tests cover two curved supports,
rotation, contracted unknown boxes, exact full interval coverage and large32x32
original control grids requiring more than1024 source rectangles. Partial/resource
failures expose no complete chain. Its publicly returned result is construction
output; downstream saved graphs still require independent source/interval replay.

These local traces do not establish automatic global atlas discovery, physical
branch graph assembly, periodic quotients or source-bound trim topology.
NURBS-0030 remains planned. Evidence:
`docs/qualification/nurbs-fiber-world-error-2026-10-16/native-tests.json`.
