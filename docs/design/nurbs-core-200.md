# Приоритетные 200 возможностей NURBS в Rust

Сохранены приоритетные наборы 150 и 180 возможностей; рубеж пользователя 160 достигнут внутри набора 180. Этот реестр сохраняет более широкий план из 200 пунктов. Дальняя цель 1200 сохраняется. Размеры, радиусы, число зубьев, варианты осей и одинаковые алгоритмы под разными именами не считаются отдельными возможностями. В набор входят формы и самостоятельные операции CAD; это не 200 разных примитивов.

Нативная готовность требует типизированного Rust API, независимых геометрических проверок, отказов на некорректных данных и описанных ограничений. WASM, TypeScript, Rush и редактор не являются условиями нативной готовности. Приближённая операция обязана явно различать выборочную оценку, непрерывную границу ошибки и учёт округления.

Текущее состояние JSON-каталога на 2026-10-22: 200 реализованных возможностей (79 integrated + 121 native), 0 planned в данном наборе ограниченных контрактов. Полная квалификация каждой возможности в WASM, UI и STEP учитывается отдельно. Исторические записи 198/200 ниже сохранены как снимки прежнего состояния. Рубеж 160 достигнут внутри сохранённого набора 180.

## Specific-root world error and local trace construction, 2026-10-16

**198/200**, unchanged. Native Rust now binds actual NURBS world/UV coefficients
to a specific original intersection root with continuous world-error bounds.
It retains and replays actual R/scales/q and complete source Jacobian bounds,
monotone full-interval clocks and all agreement cells. A curve on both supports
but on the wrong root is refused. Original multiple knot spans, reverse clocks,
genuine piecewise curvature and rotated/second-support frames pass. Adaptive
local construction publishes a chain only after complete interval admission.
Latest full libraries **1384 NURBS +63 predicates**, **46 SS integration tests**,
both-crate WASM, formatting and API/catalog checks pass. Global atlas discovery,
physical branch graphs and source-bound trim topology remain open; NURBS-0030
stays planned. [Evidence](../qualification/nurbs-fiber-world-error-2026-10-16/native-tests.json).

## Original parameter-root component census, 2026-10-16

**198/200**, unchanged. Rust now owns connected components of the complete
original parameter root set: fresh mixed-chart coverage, source-qualified bridges
and a complete separation census between different chart groups. An omitted
bridge refuses instead of creating a false extra component. Coverage and bridge
work share one bounded allowance; all stored component/separation/transition
receipts are independently replayed. Latest full libraries **1382 NURBS +63
predicates**, **34 SS integrations**, both-crate WASM and API/catalog checks pass.
Physical world branches, world traces, periodic quotients and trims remain open;
NURBS-0030 stays planned. [Evidence](../qualification/nurbs-fiber-components-2026-10-16/native-tests.json).

## Original mixed-axis local transitions, 2026-10-16

**198/200**, unchanged. Rust now qualifies whole original-root bridges between
charts using different free parameters, including a nonlinear reverse coordinate
map. Source replay, bridge and endpoint work share one allowance; private replay
rejects damaged geometry/context, orientation, endpoint receipts and charged work.
Closed natural endpoint cells must be valid before original fiber admission.
Latest full native libraries **1380 NURBS +63 exact predicates**, **31 SS
integration tests**, both-crate WASM, formatting and API/catalog checks pass.
Global component/world-trace/trim ownership remains open, so NURBS-0030 stays
planned. [Evidence](../qualification/nurbs-fiber-transitions-2026-10-16/native-tests.json).

## Broad native verification, 2026-10-16

Full three-crate command completed: **3523 passed, 0 failed, 3 explicit diagnostic/timing skips**, across219 result suites including doctests. All three former Boolean failures pass. This broad binary snapshot predates the additive mixed-axis module; that path additionally passes latest fullNURBS1379/predicates63,28uniqueSS integration tests andboth-crateWASM. Native catalog remains198/200; general physical SS branch/trace/trim ownership and general periodic correspondence/contacts are still planned. [Full evidence](../qualification/nurbs-exact-contacts-2026-10-16/native-tests.json).

## Mixed-axis original surface root covers, 2026-10-16

**198/200**, unchanged. Native Rust now owns complete closed parameter-product
covers using overlapping original Krawczyk fibers with different free axes and
intervals. Actual original exclusions cover every remaining cell; shared faces
are retained and all source replay/exclusion work is charged under one budget.
An essential mixed-axis bridge on a curved analytic intersection passes; removing
that bridge or a distinct branch refuses coverage. Latest full NURBS library
**1379**, exact predicates **63**, current SS integrations **28 unique** and
both-crate WASM checks pass. Physical chart transitions, branch/world-trace/trim
ownership remain open, so NURBS-0030 stays planned.
[Evidence](../qualification/nurbs-mixed-fiber-cover-2026-10-16/native-tests.json).

## Exact original nonlinear endpoint contacts, 2026-10-16

**198/200**, unchanged. A distinct typed Rust chart owner now transfers exact
original source restrictions through weighted flow and endpoint angular ownership.
B-rep admits endpoint-touching polynomial/rational components after independent
complete profile certification and exact inter-wire half-plane separation.
Arbitrary nonlinear correspondence, general contacts and physical face ownership
for this new path remain open; NURBS-0173 stays planned. Current verification
artifacts: [exact-contact checkpoint](../qualification/nurbs-exact-contacts-2026-10-16/CONTINUE.md).

## Exact affine/reverse coherent families, 2026-10-16

**195/200**, unchanged. Normalized original knot/control/weight identities now qualify affine and reversed source/candidate families without rounded parameter transforms. Signed original source roles are replayed independently; dictionary merging/cancellation and candidate UV regions preserve actual original weighted winding. **119 focused checks pass**, plus WASM compilation, including negative-domain seam cancellation, shifted/scaled domains, non-affine refusals and underflow/one-ULP exact comparisons. Nonlinear correspondence, contacts and supporting-surface faces remain open; NURBS-0173 is still planned. [Contract](nurbs-periodic-winding-regions.md), [evidence](../qualification/nurbs-coherent-affine-reverse-2026-10-16/native-tests.json).

## Coherent weighted charts and candidate regions, 2026-10-16

**195/200**, unchanged. Genuine copy-coherent source evidence now drives weighted chart construction and exact candidate-region ownership, preserving full original multiplicity without casting quotient embedding into full-copy embedding. All three classification stages share one allowance. Exact positive weight gauges merge without changing geometry; reverse cancellation remains correct under input permutations. **155 focused Rust checks pass**, plus WASM compilation. Nonuniform nested multiplicities retain a filled outer instead of a false quotient hole. Reversed whole-source homotopies, general contacts and supporting-surface faces remain open; NURBS-0173 is still planned. [Contract](nurbs-periodic-winding-regions.md), [evidence](../qualification/nurbs-coherent-weighted-chart-2026-10-16/native-tests.json).

## Coherent coincident-source anchor proof, 2026-10-16

**195/200**, unchanged. A separate typed copy-coherence proof now groups exact same-parameter source/candidate rational functions, including independently proportional positive weights. It verifies the distinct-function quotient isotopy and queries actual full original winding under a shared classification allowance; the quotient embedding is never relabelled as a full-copy embedding. **93 core periodic checks pass**, plus WASM compilation. Nonuniform group multiplicity is covered by a case with quotient winding 0 and original winding 1. Typed integration into weighted charts, reversed aliases, general contacts and supporting-surface faces remain open, so NURBS-0173 is still planned. [Contract](nurbs-periodic-winding-regions.md), [evidence](../qualification/nurbs-coherent-anchor-2026-10-16/native-tests.json).

## Tolerance-qualified periodic candidate regions, 2026-10-16

**195/200**, unchanged. Approximate weighted-chart admission retains the complete original isotopy, periodic-image separation and original anchor winding proof, then feeds exact candidate-region ownership. Immutable source replay rejects stale original/query winding integers and incomplete topology; pointwise source/candidate boundary equality is explicitly not claimed. The older unweighted angular-ownership policy remains unchanged. **136 focused Rust checks pass**, plus WASM compilation. Coherent coincident-source isotopy, general contacts and owned supporting-surface faces remain open; NURBS-0173 is still planned. [Contract](nurbs-periodic-winding-regions.md), [evidence](../qualification/nurbs-periodic-weighted-approx-2026-10-16/native-tests.json).

## Original periodic chart-to-region admission, 2026-10-16

**195/200**, unchanged. The typed Rust periodic-region constructor now recomputes original weighted-chart admission before exact integer cycle compression and nonzero-fill region ownership. Complete contribution/cancellation and dictionary-flow censuses preserve all original source roles; immutable provenance links region wires back to original dictionary edges and cycles. **39 native checks pass**, including repeated rational boundaries, opposite cancellation, negative fill, duplicated seam crossings, stale-source replay and budget failures; both crates compile for WASM. Approximate combined admission, general contacts and supporting-surface face topology remain open, so NURBS-0173 is still planned. [Contract](nurbs-periodic-winding-regions.md), [evidence](../qualification/nurbs-periodic-winding-regions-2026-10-15/native-tests.json).

## Original winding region regularization, 2026-10-15

**195/200**, unchanged. Exact signed winding normalization now removes redundant material-internal borders and assigns holes/material islands for separated original NURBS cycles. Immutable components retain original curves or exact parameter-negated reversals; every output receives a fresh material-left profile proof. The ordinary public profile admission policy remains strict. **45 native checks pass**, including replay/source tampering, compact rational ellipses, cubic non-dyadic reversals, cap/thread regressions and four gear profile checks; both crates compile for WASM. Inter-wire contacts and owned supporting-surface faces remain open, so NURBS-0173 is still planned. [Contract](nurbs-winding-region-regularization.md), [evidence](../qualification/nurbs-winding-regularization-2026-10-15/native-tests.json).

## Periodic endpoint B-spline germ ordering, 2026-10-15

**195/200**, unchanged. Exact common-projection contact ordering now supports clamped multispan B-spline endpoint bases, independent affine knot scales and global rational-weight scales. Adjacent binary64 bend coefficients admit exact ordering without disjoint floating intervals. Three new tests and the full **87-test periodic regression set** pass, as does WASM compilation. General unmatched germs and owned-face regularization remain open; NURBS-0173 is not promoted. [Contract](nurbs-periodic-trim.md), [evidence](../qualification/nurbs-periodic-bspline-germs-2026-10-15/native-tests.json).

## Finite external and internal threads, 2026-10-15

**195/200**: 79 integrated + 116 native, **5 planned** remain. Strict Rust constructors now bind every retained original wall to nominal rounding-inclusive bounds and complete affine clipping/UV isotopy. Compact degree-six traces preserve curved boundaries; exact disk/annular cap profiles and final continuous G0 checks admit closed body candidates. **20 core and 28 B-rep focused tests pass**, including default-precision left-handed threads and the older cap regressions. Both crates compile for WASM. Global solid geometry and a combined ideal finite-body error are explicitly unqualified. [Contract](nurbs-finite-thread-certificates.md), [evidence](../qualification/nurbs-thread-source-2026-10-15/native-tests.json).

Full native regression after the cap fix: **1861 core + 1214 B-rep tests passed**, plus two doctests; zero failures and three explicitly ignored B-rep diagnostics. Unrelated modules changed during the full run, so its result describes the compiled snapshot. The current thread/cap and periodic sources subsequently passed **132 focused checks**; qualification records the source hashes and separate logs.

## Four native gear families with verified original profiles, 2026-10-15

**193/200**: 79 integrated + 114 native, **7 planned** remain. Spur, signed helical, mirrored herringbone and internal-ring constructors now have a strict typed Rust API with complete modular original-profile embedding, independent exact winding/nesting, source-to-model binding and inclusive nominal profile/wall bounds. The 3- and 256-tooth boundaries, default 20-tooth profile, bore, internal 40-tooth ring, witness/source forgery and proof limits pass seven focused integration tests. Both crates compile for WASM. Global cap/wall embedding and complete owned-solid admission remain separate qualifications; the returned object explicitly reports `solid_geometry_certified == false`. This uses the existing native-constructor criterion, not a new fully-qualified-solid claim. [Contract](nurbs-gear-modular-profile.md), [evidence](../qualification/nurbs-gear-native-193-2026-10-15/native-tests.json).

## Complete nominal sharp-profile accuracy, 2026-10-15

Saved arcs now include angular/radius/weight uncertainty; below-base lines have complete endpoint-family bounds. Reports aggregate every admitted flank, arc and line into `profile_error_upper`, then add the saved-wall/nominal-twist bound for `ideal_wall_error_upper`. The shared Taylor owner avoids interval dependency with a midpoint/Lipschitz enclosure. **25 focused checks passed**, plus WASM compilation. Larger/internal profile embedding and whole-solid admission remain unfinished, so the native milestone remains **189/200**. [Contract](nurbs-gear-full-profile-accuracy.md), [evidence](../qualification/nurbs-gear-full-profile-accuracy-2026-10-15/native-tests.json).

## Continuous original gear arc radii, 2026-10-15

All saved tip/root, bore and outer-rim arcs now receive independent continuous radial bounds including nominal radius derivation and canonical endpoint changes. The typed curve API reuses the existing original homogeneous Bernstein-product owner. **15 focused checks passed**, plus WASM compilation. This proves radial accuracy, not nominal angular/line correspondence or a general embedded solid. Native count remains **189/200**. [Contract](nurbs-gear-arc-radial-bounds.md), [evidence](../qualification/nurbs-gear-arc-radius-2026-10-15/native-tests.json).

## Exact gear profile endpoint ownership, 2026-10-15

Line/arc joins and full-wire/hole seams now share identical saved endpoints while every certified involute pole is retained. The report includes an outward continuous adjustment bound. Internal reverse knots are independently requalified. Exact original-profile certificates pass for 3 and 8 teeth. **12 focused checks passed**, and WASM compilation passes. General larger/internal profile qualification and nominal root geometry remain unfinished, so the native count remains **189/200**. [Contract](nurbs-gear-exact-profile-joins.md), [evidence](../qualification/nurbs-gear-profile-joins-2026-10-15/native-tests.json).

## Inclusive saved helical walls, 2026-10-15

The original rational-profile helical sweep now has an independent outward saved-net certificate covering original V knots, saved poles, parameter boxes and the continuous interpolation remainder. Gear walls use nominal twist derivation boxes, including signed angles and herringbone halves. **22 focused tests passed**, and both crates compile for WASM. [Contract](nurbs-helical-saved-net-certificate.md), [evidence](../qualification/nurbs-helical-inclusive-2026-10-15/native-tests.json). Root/profile embedding remains unfinished, so the count remains **189/200**.

## Native regression qualification, 2026-10-15

Full Rust baseline: **1823 core tests + 1135 B-rep tests passed**, zero failures, three explicitly ignored B-rep tests. The nominal gear refactor subsequently passed 15 focused checks (these overlap the baseline and are not added as unique tests). Both crates pass WASM target compilation; this is not a packaged browser/editor/STEP runtime qualification. The catalog/API checks and focused formatting/diff checks pass. [Evidence](../qualification/nurbs-native-189-2026-10-15/verification.json). **189/200** remains the honest native count; eleven planned items are not promoted by candidate generation alone.

## Nominal gear flank parameter bounds, 2026-10-15

The full GearSpec module, degree-pressure, backlash and clearance derivation now has outward parameter boxes and a continuous saved-quintic family certificate. Per-gear flags distinguish admitted nominal bounds from stored-parameter-only bounds. Gear catalog status stays planned pending root/profile topology and saved helical-wall rounding. **189/200**, unchanged. [Contract](nurbs-gear-parameter-certification.md).

## Rational patch skins with sampled thickness assurance, 2026-10-15

**189/200**: 79 integrated + 110 native; **11 planned** remain. Three independent tests pass for signed planar thickness/volume, a rational curved skin, original source-cap retention, all six saved-face regularity proofs, exact owned seam topology and continuous saved-boundary agreement. Ideal normal-thickness accuracy is explicitly sampled; general solid shelling and continuous ideal-thickness/embedding certificates remain separate. [Contract](nurbs-patch-shell-thickness.md).

## Exact original orthogonal offset cleanup, 2026-10-15

**188/200**: 79 integrated + 109 native; **12 planned** remain. Three integration tests pass for source-exact orthogonal miter offsets, full exact arrangement identity, crossings and coincident opposite cancellation, multiple fill components, common Z, source immutability and explicit precision/degeneracy/work refusals. NURBS-0188 is native in this declared specialization; general smooth offset topology and Minkowski buffer semantics remain separate. [Contract](nurbs-orthogonal-offset-cleanup.md).

## Certified original curve / plane roots, 2026-10-15

**187/200**: 79 integrated + 108 native; **13 planned** remain. Six new integration tests pass for original nonplanar rational curve roots against a containing rectangular plane, nonunit/reversed UV domains, endpoints, complete overlap with checked UV trace, subnormal gaps, tangency and explicit clipping/nonaffine/work obligations. NURBS-0164 is native within this documented scope; general surface roots and trim ownership are not claimed. [Contract](nurbs-certified-curve-plane.md).

## Certified curve crossings, 2026-10-15

**186/200**: 79 integrated + 107 native; **14 planned** remain. Six new integration tests and the full 1342-test core suite pass. NURBS-0163 now has checked affine 3D-line roots, original planar rational transverse root existence/uniqueness, complete original product coverage, spatial localization and exact same-definition overlap. General nonlinear spatial/tangent contacts remain outside the admitted native scope. [Contract](nurbs-certified-curve-crossings.md).

## Automatic partial straight-boundary patch closure, 2026-10-15

**185/200**: 79 integrated + 106 native; **15 planned** remain. NURBS-0197 now includes automatic exact image partitions with unique opposite uses, partial one-to-many overlaps, and nonlinear rational boundary parameterization. Three new tests and nine existing seam/correspondence tests pass. Full original lift agreement is mandatory; general curved partial matching and solid embedding remain outside this native scope. The three multispan Boolean regressions also pass after adding original monotone graph coverage; general SS ownership remains planned. [Patch contract](nurbs-linear-patch-closure.md), [SS contract](nurbs-ss-monotone-graph.md).

## Original-boundary nonlinear correspondence, 2026-10-15

**184/200 remains unchanged**. NURBS-0197 gained continuous original-domain piecewise affine parameter correspondence, bounded nearest-lift inference, original-domain reversal and shared-budget B-rep checks. The search cannot authorize a match; full-interval proofs must independently admit both face lifts. Automatic partial-overlap fragmentation, unique pairing across a complete patch set and topology ownership remain open. [Contract](nurbs-boundary-correspondence.md).

## Clearance-qualified planar contour machining, 2026-10-15

**184/200**: 79 integrated + 105 native; **16 planned** remain. Six independent integration tests passed: threshold-driven original-curve clearance, close-pair versus work refusal, clockwise/counterclockwise paths, rational circle, nested spacing, full nominal-offset tolerance, limited work and a cutter too large for a concave slot. Complete paths require original-loop winding and global cutter/stock clearance; no sampled display proxy admits the plan. [Contract](nurbs-planar-toolpath.md).

## Authored trimmed multipatch shells, 2026-10-15

**183/200**: 79 integrated + 104 native; **17 planned** remain. Five independent integration tests passed: cube and rational hollow shell, full-interval boundary agreement, trim-region and regularity audits, vertex-link closure, malformed/discontinuous input, exhausted work and an endpoint-preserving pcurve bulge. The constructor authors a Shell only after all gates; no Body or embedding certificate is manufactured. [Contract](nurbs-trimmed-shell.md).

## Parallel plane sections, 2026-10-15

**182/200**: 79 integrated + 103 native; **18 planned** remain. Five independent integration tests passed, including rational sources, nonnormalized domains, negative nonunit normals, empty levels, tangencies/coincidence and shared-budget exhaustion. Complete admitted parameter graphs have continuous original-surface spatial error bounds; unsupported topology remains explicitly unresolved over the full source domain. [Contract](nurbs-parallel-sections.md).

## Section-authored blade, 2026-10-15

**181/200**: 79 integrated + 102 native; **19 planned** remain. Three independent integration tests passed: polynomial profile oracle, actual ruled walls, closed owned incidence and invalid inputs. Native construction supports chord, thickness-control, twist and stacking at 2..129 stations. Global embedding, aerodynamic performance and adapter/export integration remain separate. [Contract](nurbs-blade.md).

## G2 with inferred nonaffine correspondence, 2026-10-15

**180/200**: 79 integrated + 101 native; **20 planned** remain. Seven independent integration tests passed, including a quadratic boundary reparameterization and a rational cylinder under a Mobius reparameterization, with reversed orientation. Search proposes a monotone open-seam relation; original-surface interval bounds independently verify position, normal and curvature over its entire domain. Closed-seam phase search and topology authorship remain outside this native API. [Contract](nurbs-surface-g2-affine.md).

## Актуальный итог — замкнутый sweep native, 2026-10-15

**179/200**: 79 integrated + 100 native, осталось **21 planned**. Проверены progressive_profile_body_with_rmf_policy и surface-only framed_sweep: неплоский закрытый путь, holonomy, rational sections, periodic owned incidence и открытый cross-section. Body-режим отдельно подтверждает continuous original-to-retained boundary upper; surface-only диагностика остаётся sampled.

Прошли **3 independent integration tests**; полый periodic Body и STEP-void fixture включены в прошедший набор из 19 section-body tests. На сетке 129 sections увеличение proof steps 512→4096 уменьшило доказанную границу 0.12551→0.01665; допуск 0.03 и sampled geometry не менялись. Full Body требует boundary receipt <=0.03. G1/G2 seam, embedding/global solid и universal STEP не квалифицированы. [Контракт](nurbs-closed-sweep-native.md). Текущие связанные проверки завершены; далее — предыдущие рубежи.

## Актуальный итог — корпус по сечениям native, 2026-10-15

**178/200**: 79 integrated + 99 native, осталось **22 planned**. Проверен существующий `brep_core::rational_section_loft`: рациональные боковые стенки через authored sections, outer loop и holes, planar trim caps, shared edges и замкнутая B-rep incidence. Реализация не дублировалась; каталог приведён в соответствие с проверенным Rust API.

Прошли **19 integration tests** offline/locked, codec, no-default-features: independent cone wall equations, caps/holes, incidence, 33 sections, reverse travel, natural and supplied walls, malformed input, transport variants и конкретные STEP round trips. SHA-256 основного исходника и теста не изменились за прогон. Global embedding/solid и общий rounding-inclusive design-intent error не квалифицированы. [Контракт](nurbs-body-by-sections.md). Далее — предыдущие рубежи.

## Актуальный итог — огибающая сфер native, 2026-10-14

**177/200**: 79 integrated + 98 native, осталось **23 planned**. Добавлен `sphere_envelope::approximate_with_domain_audit`: характеристические круги движущихся сфер учитывают dr/ds, исходные rational jets непрерывно проверяют spanwise existence и reference angular chart, адаптивная сборка сохраняет rational circle sections и полное path coverage. Недостаточное доказательство не даёт поверхности в audited API. Выборочная точность аппроксимации отделена от доказательства существования кругов.

Прошли **9 integration tests** на актуальной Cargo-built Rust-библиотеке: sphere/envelope equations, rational radius и independent domains, конус/тор, неплоская polynomial centerline, whole-domain feasibility и недостаточный budget, invalid inputs и shared boundaries. Source/test/library hashes сохранены вместе с командой и полным результатом. Continuous approximation error, rounding, immersion/self-overlap, singular extensions и B-rep solid не квалифицированы. [Контракт](nurbs-sphere-envelope.md). Далее — история предыдущих рубежей.

## Сверка состояния — 2026-10-14

**176/200**, осталось **24**. NURBS-0030 (общее пересечение поверхностей) остаётся planned: наличие кандидатов и малый residual не доказывают полноту пересечения или корректность trim-топологии. Частичные исходные редукции: разделённые control hulls, идентичные поверхности и ограниченные координатные прямоугольные патчи с точно представимым UV-отображением. Геометрическое покрытие и право строить топологию учитываются отдельно; адаптер B-rep обязан сохранять это различие. JSON-проверка проверяет структуру отчёта, а не воспроизводит математическое доказательство.

Приоритет остатка: пересечения curve/curve, curve/surface, surface/surface; замкнутый sweep и shell; periodic trim, очистка offset и watertight audit; затем механические формы. Исторические результаты ниже относятся к указанным прогонам; эта сверка не означает повторную квалификацию всех 176 возможностей.

## Актуальный итог — гофрированная оболочка native, 2026-10-14

**176/200**: 79 integrated + 97 native, осталось **24 planned**. Добавлен `bellows::cubic`: cubic axial corrugation, outer/cavity rational surfaces и два annular rims с корректной normal orientation. Полная модель сохраняется до 256 periods; shared boundary controls/weights/knots формируют data-matched surface collection. Толщина задана радиально, не по normal; NURBS-0080 остаётся planned.

Прошли пять independent integration groups offline/locked без default features: analytic cubic-wave radius/height, signed/zero amplitude, radial gap, normals, boundary identity including reversed weights/knots, full resource limit и numerical-collapse/invalid-input refusals. Source/test hashes совпали до/после финального прогона. B-rep incidence/manifold/solid и binary64 rounding не квалифицированы. [Контракт](nurbs-bellows.md). Далее — история прошлых рубежей.

## Актуальный итог — рациональные G1/G2 bridges native, 2026-10-14

**175/200**: 79 integrated + 96 native, осталось **25 planned**. Добавлен `surface_blend`: cubic/quintic homogeneous Hermite bridges между любыми U/V boundaries рациональных Surfaces. Поддержаны разные degrees/knots/weights, unclamped boundaries, reversed affine correspondence и positive transverse gains. Полные homogeneous boundary jets выравниваются в polynomial basis, затем строятся rational Surface patches с положительными weights.

Прошли пять independent integration groups offline/locked, no-default-features: full Cartesian first/second/mixed jets, несепарабельные weights и W'', все boundary axes/orientations, unclamped domains, resource/invalid scales и analytic rational Möbius oracle. Конструкция даёт C1/C2 matching в real arithmetic и G1/G2 на регулярных source charts; binary64 continuity certificate, global regularity, nonlinear matching и B-rep topology не квалифицированы. Общая NURBS shell NURBS-0080 остаётся planned: её текущие реализации ограничены analytic/planar bodies. [Контракт](nurbs-surface-blend.md). Далее — история прошлых рубежей.

## Актуальный итог — радиальный кулачок native, 2026-10-14

**173/200**: 79 integrated + 94 native, осталось **27 planned**. Добавлен `radial_cam::profile`: translating knife-edge/roller follower, dwell/harmonic/cycloidal/345/4567 motion laws, adaptive cubic contour и sampled pressure/undercut diagnostics. Программа покрывает полный 360-degree cycle; source-angle cells и curve chunks сохраняют full coverage/shared endpoints, последняя control point канонически равна первой. Budget/parameter-precision stops возвращают полный contour с явным недостатком качества.

Прошли пять independent integration groups offline/locked, no-default-features: base-circle invariant для обоих followers, четыре motion laws, lift stations, independent roller-contact geometry, closure/coverage, sampled undercut, budget/invalid inputs и very short angular precision. Error/pressure/undercut остаются sampled estimates, а не continuous manufacturing certificate; output — контур, без extruded/manifold B-rep body. [Контракт](nurbs-radial-cam.md). Далее — история предыдущих рубежей.

## Актуальный итог — пружина с профилем native, 2026-10-14

**172/200**: 79 integrated + 93 native, осталось **28 planned**. Добавлен `profile_spring::approximate`: произвольный rational XY profile переносится в analytic helix normal frame. Profile basis/weights сохраняются; Hermite transport разбивается на Surface patches с одинаковыми boundary controls. Refinement управляется непрерывной real-arithmetic remainder estimate, при span budget возвращается вся спираль до t=1.

Прошли три independent integration groups offline/locked, no-default-features: обе навивки, signed/zero height, 2D/3D XY rational profile, analytic target, normal-plane/metric invariants, patch boundaries, non-power-of-two budget и invalid requests. Source hash конструктора совпал с предшествующим снимком; последняя версия tests прошла повторно после добавления 3D XY case. Libm/rounding, self-collision, regularity и caps/solid topology не квалифицированы. [Контракт](nurbs-profile-spring.md). Далее — история прошлых рубежей.

## Актуальный итог — поверхность по облаку native, 2026-10-14

**171/200**: 79 integrated + 92 native, осталось **29 planned**. Исправлена PCA parameterization cloud surface fitting: масштабированные координаты/covariance, независимая вторая ось с выбором по реальному projected spread, orthogonal power iteration и явный отказ numerically rank-one clouds. Удалена необоснованная diameter/sqrt(N) граница. Site residual теперь вычисляется outward interval tensor de Casteljau; весь Surface против конечного облака имеет coarse control/data-box Hausdorff bound.

Прошли **15 профильных tests** offline/locked без default features: четыре independent cloud integration groups, шесть dependent offset/fillet groups, пять fitting unit tests. Проверены rotated/correlated plane, все три axis-aligned orientations (регрессия вертикального chart), duplicated sites, degeneracy/invalid inputs. Три source/test hashes совпали. Границы относятся к возвращённой mathematical surface и конечным authored sites; неизвестная continuous source и её topology не реконструированы с сертификатом. [Контракт](nurbs-cloud-surface.md). Далее — история прошлых рубежей.

## Актуальный итог — constant/variable fillet native, 2026-10-14

**170/200**: 79 integrated + 91 native, осталось **30 planned**. Проверены rolling-ball и variable-radius surface fillet constructors. Public RadiusLaw evaluation больше не panic на malformed spline; добавлен typed `evaluate_checked`, проверяющий query и nonpositive/nonfinite результат. Уточнены sampled reachability и reference-spine semantics; positive cubic node radii не выданы за continuous positivity proof.

Прошли **14 fillet tests** offline/locked без default features: три независимых integration groups и 11 unit tests. Новые случаи — translated perpendicular supports с authored UV domains, independent cylinder equation, linear/spline station radii, boundary positions/normals, malformed public laws, positive-node negative overshoot и явный отказ для parallel separated supports. Source/test hashes совпали до/после. Нативный контракт — approximate surface constructors; continuous G1/G2, law positivity, B-rep trimming/solid и full error certificate не квалифицированы. [Контракт](nurbs-fillet-surfaces.md). Далее — история прежних рубежей.

## Актуальный итог — surface offset native, 2026-10-14

**168/200**: 79 integrated + 89 native, осталось **32 planned**. Проверен и исправлен `surface_offset::offset`: sample-based plane/sphere/cylinder recognition больше не выдаёт exact=true; точный флаг сохранён для zero-distance identity. Добавлена typed assurance ExactIdentity/SampledDeviation. Cloud-fit certificate больше не подменяет offset error: fitted Surface получает исходные UV domains и повторное измерение относительно S+dn перед принятием.

Прошли **29 профильных tests** offline/locked, no-default-features: три новые integration groups, 15 surface_offset unit tests и 11 dependent fillet tests. Независимая регрессия воспроизводит ложную exactness на z=1e-10*u*v; другая проверяет Euclidean residual в authored U=[2,7],V=[-3,1]. Четыре source/test hashes совпали до/после. Offset deviation и fold-free radius остаются sampled estimates, а не continuous/rounding certificate; tolerance excess и degeneracy требуют проверки caller. [Контракт](nurbs-surface-offset.md). Далее — история прежних рубежей.

## Актуальный итог — геодезическая native, 2026-10-14

**167/200**: 79 integrated + 88 native, осталось **33 planned**. Проверен существующий RK4 geodesic tracing исходной поверхности. Исправлены overflow/underflow при нормализации UV direction, length_achieved при boundary exit и разрешение очень малой доли boundary step через прямое rectangular clipping вместо фиксированных bisections. Слишком малая длина для представимого начального шага отклоняется.

Прошли **9 geodesic tests** offline/locked, no-default-features: четыре независимых integration groups и пять существующих unit tests. Проверены affine-plane endpoints/distance, direction scales 1e±300, boundary lengths при запросе 1e100, cylinder helix, sphere great circle/pole, geodesic offset и invalid/budget cases. Source/test hashes не изменились. Контракт — численное initial-value tracing с явными остановками; локальная sampled curvature не сертифицирует global drift/shortest path. [Ограничения](nurbs-geodesic.md). Далее — история прежних рубежей.

## Актуальный итог — крыло native, 2026-10-14

**166/200**: 79 integrated + 87 native; осталось **34 planned**. Проверен существующий `wing_loft`, добавлены root/domain API paths, исправлен лимит исходных сечений согласно выбранной законцовке (None 11 / Collapse 10 / Dome 9), reference parameterization переведена на hypot и масштабированное накопление с явным отказом при слиянии station parameters.

Прошли **13 wing tests** offline/locked без default features: три независимых integration groups и 10 существующих unit tests, включая NACA, placement, tip, sampled folding и fairing boundaries. Новые regressions проверяют исходные rational profiles после affine placement при разных twist/taper, неравномерные spans, сохранение curve, допустимый координатный предел 1e9 и отказ за ним, все tip limits и invalid inputs. Source/test hashes не изменились во время финального прогона. Sampled folding не является сертификатом injectivity; B-rep/root closure, формальный G1/G2 tip и binary64 error не квалифицированы. [Контракт](nurbs-wing-loft.md). Далее сохранена история прошлых рубежей.

## Актуальный итог — купол и линза native, 2026-10-14

**165/200**: 79 integrated + 86 native, осталось **35 planned**. Добавлен `optical_shapes`: rational sagitta dome с эллиптическим основанием (minor/hemisphere/major cap) и сборка convex lens из двух cap surfaces с общим ободом и optional barrel. Поддержаны plano-convex, zero-rim и cylinder limits; недопустимые толщины и размеры отклоняются.

Прошли три группы native tests offline/locked без default features: independent implicit sphere equation после обратного масштабирования Y, pole/rim/seam controls, вершины/толщина, отсутствие пересечения caps, точные barrel boundaries и invalid inputs. Исходник и тест совпали по SHA-256 до/после. Нативный контракт — поверхности с точной рациональной конструкцией в вещественной арифметике; B-rep solid, округление и concave/meniscus lens не квалифицированы. [Подробности](nurbs-optical-shapes.md). Далее — история прошлых рубежей.

## Актуальный итог — суперэллипсоид native, 2026-10-14

**163/200**: 79 integrated + 84 native; осталось **37 planned**. Добавлен `nurbs_core::superellipsoid::approximate`: полный signed-power суперэллипсоид с двумя независимыми показателями, полуосями, центром, адаптацией равномерной сетки по оценке и общим бюджетом. Восемь октантов разбиваются на допустимые Surface-патчи с общими исходными параметрами и одинаковыми seam controls; полюса/оси канонические.

Прошли три группы независимых геометрических/ресурсных тестов, offline/locked без default features. Проверки включают implicit equation четырёх семейств, отклонения внутри патчей, сетку из 32768 ячеек, стыки, полюса и invalid inputs. Ограничение контракта: аналитическая интерполяционная оценка предполагает точные samples; libm/rounding и B-rep solid не квалифицированы. Предел бюджета возвращает всю форму, без ложного полного сертификата точности. См. [контракт](nurbs-superellipsoid.md). Далее сохранена история предыдущих рубежей.

## Актуальный итог — NURBS-0162 native, 2026-10-14

**162/200**: 79 integrated + 83 native, осталось 38 planned. NURBS-0162 закрыт на нативном уровне: общий exact original-span Sturm/GCD путь, кратности, совпадения, исходные endpoint/seam ownership и полное покрытие активных интервалов. Исправлен строгий допуск: перед PrecisionLimit quadratic ветка обращается к original polynomial и сохраняет точно представимый dyadic root без snapping.

Последний прогон: **128 nurbs-core tests** (47 curve_plane, 42 periodic_trim, 22 contacts, 11 support, 6 weighted), offline/locked, no-default-features. Перед ним прошли 90 cad-predicates tests. Семь файлов Sturm-пути не изменились во время прогона (SHA-256). Проверены точный double root t=3.25 при tolerance=1e-20 и сохранение Unresolved для непредставимого корня при невозможном допуске. Ограничения degree/controls, исходные единицы, бюджет и отказы документированы в [curve-plane](nurbs-curve-plane.md). Это нативная квалификация; приложение, WASM и экспорт не проверены. Ниже — история предыдущих состояний.

## Последняя сверка рабочего дерева — 2026-10-14

Последний завершённый прогон: **127 nurbs-core tests прошли** (46 curve_plane, 42 periodic_trim, 22 contacts, 11 support, 6 weighted), offline/locked без default features. Общий original-span Sturm-путь проверен вместе с trim; четыре прежних ожидания proof обновлены для принятия точного Sturm-доказательства при неизменных геометрических проверках. Непосредственно перед этим прошли 90 cad-predicates tests. Ниже сохранена история устранённой блокировки и первого запуска.

Подтверждённый статус каталога: **161/200** (79 integrated + 82 native), **39 planned**. Проверки `map-nurbs-api.mjs --check` и `audit-nurbs-catalog.mjs` завершились успешно: 165 публичных модулей/реэкспортов, 200 записей, ошибок нет. Количество публичных имён не является количеством самостоятельных возможностей.

Для NURBS-0162 добавлен общий точный изолятор полиномиального числителя исходного рационального B-spline: цепочки Штурма, кратности корней, исходные не-Bezier интервалы и ограничение арифметического бюджета. В текущем дереве прошли **90 cad-predicates tests** (`--lib --tests`, offline/locked, без default features): 45 unit и 45 integration. Это подтверждает слой predicates, но не завершает квалификацию нового пути в nurbs-core.

Повторный профильный прогон nurbs-core **остановился до запуска тестов**: E0583 в `src/sweeps/progressive_sweep.rs:26`, отсутствует объявленный модуль `progressive_sweep/endpoint_points.rs`. Поэтому прежние 124 успешных профильных теста ниже являются исторической проверкой предыдущего состояния, а не подтверждением текущего дерева. Полный запуск cad-predicates с examples также блокируется отсутствующей зависимостью `value_codec` у примера `bezier-identity-fixtures`.

Повторная сверка в том же рабочем дереве: `progressive_sweep/endpoint_points.rs` появился; прежний E0583 больше не является текущей блокировкой. Повторный профильный прогон запущен, сборка ещё выполняется. Семь файлов нового Sturm-пути совпадают по SHA-256 с исходным снимком проверки predicates. Первый повторный native прогон завершился: 42 curve_plane tests прошли, четыре отказа касались только ожидания вида proof (точный Sturm вместо прежнего endpoint proof); геометрические assertions до них прошли. Ожидания обновлены с сохранением геометрических проверок, повторный прогон ещё выполняется.

NURBS-0162 и NURBS-0173 остаются planned. Следующий шаг: восстановить сборку рабочего дерева, проверить новый Sturm-путь вместе с periodic trim и затем пересмотреть готовность NURBS-0162. Для NURBS-0173 остаются неоднозначные контакты с совпадающими ведущими коэффициентами и общий алгоритм построения принадлежащих области граней. WASM, редактор и STEP текущей сверкой не проверены.

## Актуализация 2026-10-14

Источник статусов — [nurbs-catalog.json](nurbs-catalog.json): **161/200** (79 integrated, 82 native), осталось **39**. Это число возможностей по контрактам каталога; оно не означает, что все они доступны через редактор или проверены в общей WASM-сборке.

Структура Rust физически разделена по тематическим папкам; навигационный API и старые пути сохранены. Проверки карты API и каталога выполнены повторно: 165 публичных модулей/реэкспортов, 200 записей, ошибок нет.

NURBS-0173 остаётся planned. Уже реализованы поиск периодических швов, точное и приближённое разбиение, проверки отклонения и связности, перенос winding для точного пути и сборка по двум швам с однозначными угловыми переходами. Подробности и ограничения: [periodic trim](nurbs-periodic-trim.md).

Завершён профильный прогон **81 теста** (34 periodic_trim, 36 curve_plane, 4 curve_self_intersection, 4 uv_closure, 3 uv_simplicity), без default features. Проверка isotopy подтверждена для достаточных условий, описанных в periodic trim. После двух дополнительных регрессий повторно прошли **36 periodic_trim tests**; бюджеты и столкновение периодических образов не выдают ложный сертификат. Общая сборка приложения, WASM и STEP этим прогоном не квалифицированы.

Приближённое разбиение связано со сборкой trim-области; последняя профильная проверка — **88 тестов** (41 periodic_trim и 47 связанных проверок). Для supporting surface отдельно проверены точные швы и непрерывная граница 3D-отклонения (8 новых тестов); исправлены единицы производных на коротких knot spans (3 теста). Объединённый exact/approximate trim/surface admission проверен в native API (55 профильных тестов). Сбалансированные контакты с различными чередующимися направлениями получают точное назначение продолжений (58 профильных тестов). Несбалансированные multibranch side ports закрываются точным integer flux с проверкой противоположных сторон (60 профильных тестов). Многоветвевые corner ports поддержаны полным perimeter flux (62 различных профильных теста). Совпадающие leading rays с разными exact bend sides разделяются без снапинга (65 различных профильных тестов). Касания с одинаковой bend side разделяются по точному radial bend exponent, включая cusps и независимые параметризации (68 различных профильных тестов). Regular касания с равным показателем разделяются по строгим interval gaps коэффициентов, включая internally tangent hole и combined surface admission (71 различный профильный тест). Higher-order equal-exponent ветви проверяются общими powered leading coefficients без дробных корней (74 различных профильных теста). Общая целая кратность поддержана отдельными exact weighted chains с точным сокращением opposite encodings (80 различных профильных тестов). Ближайшая работа: неразделённые equal leading terms и регуляризация произвольных пересечений в owned faces. Затем продолжить остальные planned пункты реестра.

NURBS-0162: Original rational de Boor подтверждает noninterpolated cubic/quartic endpoint roots и periodic cubic seam с общим exact-work budget. Проверены 120 native nurbs-core tests и 2 новых exact-predicate tests; Добавлены arbitrary quadratic tangency и exact discriminant exclusion, включая seam search без snapping (124 native tests и 4 predicate tests). Общая higher-degree/non-Bezier tangency qualification остаётся planned.

## Порядок реализации

1. Завершить аудит существующих Rust операций редактирования, проекции, расстояний, пересечений и trim; подтвердить контракт и независимые проверки перед переводом planned → native.
2. Закрыть недостающие базовые операции и ограничения ошибки.
3. Реализовать профили деталей, шестерни и резьбы через общие операции; не заменять аналитический профиль фасетами.
4. Развить guided loft, sweep, сопряжения, offset и оболочки.
5. Проверить весь набор на вырожденных данных, периодических швах, разных масштабах и бюджетах ресурсов.

## Реестр

| ID | Возможность | Статус |
| --- | --- | --- |
| NURBS-0001 | отрезок | integrated |
| NURBS-0002 | ломаная | integrated |
| NURBS-0003 | окружность | integrated |
| NURBS-0004 | круговая дуга | integrated |
| NURBS-0005 | эллипс | integrated |
| NURBS-0006 | эллиптическая дуга | integrated |
| NURBS-0007 | парабола | integrated |
| NURBS-0008 | гипербола | integrated |
| NURBS-0009 | кривая Безье | integrated |
| NURBS-0010 | составная NURBS-кривая | integrated |
| NURBS-0011 | эвольвента | integrated |
| NURBS-0012 | архимедова спираль | integrated |
| NURBS-0013 | логарифмическая спираль | integrated |
| NURBS-0014 | клотоида | integrated |
| NURBS-0015 | циклоида | integrated |
| NURBS-0016 | эпициклоида | integrated |
| NURBS-0017 | гипоциклоида | integrated |
| NURBS-0018 | трохоида | integrated |
| NURBS-0019 | цепная линия | integrated |
| NURBS-0020 | Лиссажу | integrated |
| NURBS-0021 | цилиндрическая винтовая линия | integrated |
| NURBS-0022 | коническая винтовая линия | integrated |
| NURBS-0023 | переменный шаг | integrated |
| NURBS-0024 | эллиптическая винтовая линия | integrated |
| NURBS-0025 | сферическая спираль | integrated |
| NURBS-0026 | торическая спираль | integrated |
| NURBS-0027 | торический узел | integrated |
| NURBS-0028 | пространственная интерполяция | integrated |
| NURBS-0029 | геодезическая | native |
| NURBS-0030 | пересечение поверхностей | native |
| NURBS-0031 | плоский патч | integrated |
| NURBS-0032 | билинейный патч | integrated |
| NURBS-0033 | поверхность Безье | integrated |
| NURBS-0034 | цилиндр | integrated |
| NURBS-0035 | эллиптический цилиндр | integrated |
| NURBS-0036 | конус | integrated |
| NURBS-0037 | усечённый конус | integrated |
| NURBS-0038 | сфера | integrated |
| NURBS-0039 | эллипсоид | integrated |
| NURBS-0040 | тор | integrated |
| NURBS-0041 | эллиптический тор | integrated |
| NURBS-0042 | эллиптический параболоид | integrated |
| NURBS-0043 | гиперболический параболоид | integrated |
| NURBS-0044 | однополостный гиперболоид | integrated |
| NURBS-0045 | двуполостный гиперболоид | integrated |
| NURBS-0046 | геликоид | integrated |
| NURBS-0047 | катеноид | integrated |
| NURBS-0048 | вращение профиля | integrated |
| NURBS-0049 | винтовая поверхность профиля | integrated |
| NURBS-0050 | суперэллипсоид | native |
| NURBS-0051 | extrusion | integrated |
| NURBS-0052 | ruled | integrated |
| NURBS-0053 | loft | integrated |
| NURBS-0054 | guided loft | integrated |
| NURBS-0055 | closed loft | integrated |
| NURBS-0056 | Coons | integrated |
| NURBS-0057 | Gordon | integrated |
| NURBS-0058 | трёхграничный патч | integrated |
| NURBS-0059 | многограничное заполнение | integrated |
| NURBS-0060 | поверхность по облаку | native |
| NURBS-0061 | труба постоянного радиуса | integrated |
| NURBS-0062 | переменного радиуса | integrated |
| NURBS-0063 | profile sweep | integrated |
| NURBS-0064 | twisted sweep | integrated |
| NURBS-0065 | рациональный translational sweep с переменным масштабом профиля | integrated |
| NURBS-0066 | двухнаправляющий sweep | integrated |
| NURBS-0067 | closed sweep | native |
| NURBS-0068 | огибающая сфер | native |
| NURBS-0069 | лента вдоль пути | integrated |
| NURBS-0070 | пружина с профилем | native |
| NURBS-0071 | переход круг–круг | integrated |
| NURBS-0072 | круг–прямоугольник | integrated |
| NURBS-0073 | эллипс–эллипс | integrated |
| NURBS-0074 | смещённые профили | native |
| NURBS-0075 | constant fillet | native |
| NURBS-0076 | variable fillet | native |
| NURBS-0077 | G1 blend | native |
| NURBS-0078 | G2 blend | native |
| NURBS-0079 | offset | native |
| NURBS-0080 | оболочка заданной толщины | native |
| NURBS-0081 | прямозубая | native |
| NURBS-0082 | косозубая | native |
| NURBS-0083 | шевронная | native |
| NURBS-0084 | внутренняя | native |
| NURBS-0085 | рейка | native |
| NURBS-0086 | коническая шестерня | native |
| NURBS-0087 | червяк | native |
| NURBS-0088 | червячное колесо | native |
| NURBS-0089 | наружная резьба | native |
| NURBS-0090 | внутренняя резьба | native |
| NURBS-0091 | кулачок | native |
| NURBS-0092 | лопатка | native |
| NURBS-0093 | крыло | native |
| NURBS-0094 | корпус по сечениям | native |
| NURBS-0095 | купол | native |
| NURBS-0096 | линза | native |
| NURBS-0097 | гофрированная оболочка | native |
| NURBS-0098 | волновая поверхность | native |
| NURBS-0099 | S(u,v) | native |
| NURBS-0100 | trimmed multipatch shell | native |
| NURBS-0101 | Полиномиальная пространственная кривая | integrated |
| NURBS-0102 | Полиномиальная параметрическая поверхность XYZ | integrated |
| NURBS-0103 | Рациональная полиномиальная пространственная кривая | integrated |
| NURBS-0104 | Рациональная полиномиальная параметрическая поверхность XYZ/W | integrated |
| NURBS-0105 | Полиномиальный граф поверхности | integrated |
| NURBS-0106 | Кубическая седловая поверхность с тремя направлениями | integrated |
| NURBS-0107 | натуральный кубический пространственный сплайн по точкам | integrated |
| NURBS-0108 | кубический пространственный сплайн с заданными касательными на концах | integrated |
| NURBS-0109 | замкнутая кубическая интерполяция с согласованием производных на шве | integrated |
| NURBS-0110 | бикубический патч по угловым позициям и производным Hermite | integrated |
| NURBS-0111 | натуральная бикубическая интерполяция прямоугольной сетки точек | integrated |
| NURBS-0112 | натуральный кубический loft рациональных сечений с неравномерными параметрами | integrated |
| NURBS-0113 | кубический рациональный loft с заданными постоянными концевыми производными профилей | integrated |
| NURBS-0114 | скруглённый прямоугольный профиль | native |
| NURBS-0115 | капсульный профиль | native |
| NURBS-0116 | плоский кольцевой сектор | native |
| NURBS-0117 | профиль шпоночного паза | native |
| NURBS-0118 | профиль Т-образного паза | native |
| NURBS-0119 | профиль ласточкин хвост | native |
| NURBS-0120 | профиль поликлинового ремня | native |
| NURBS-0121 | профиль зубчатого ремня | native |
| NURBS-0122 | профиль O-ring канавки | native |
| NURBS-0123 | профиль стопорного кольца | native |
| NURBS-0124 | профиль шестигранного крепежа | native |
| NURBS-0125 | профиль квадратного крепежа | native |
| NURBS-0126 | вставка узлов кривой | native |
| NURBS-0127 | поднятие степени кривой | native |
| NURBS-0128 | разбиение кривой по параметру | native |
| NURBS-0129 | обрезка кривой по интервалу | native |
| NURBS-0130 | смена направления кривой | native |
| NURBS-0131 | разложение кривой на рациональные сегменты Безье | native |
| NURBS-0132 | удаление узла кривой с допуском | native |
| NURBS-0133 | понижение степени кривой с допуском | native |
| NURBS-0134 | rebuild кривой с допуском | native |
| NURBS-0135 | перенос шва периодической кривой | native |
| NURBS-0136 | линейная перепараметризация кривой | native |
| NURBS-0137 | локальное изменение весов кривой | native |
| NURBS-0138 | вставка узлов поверхности по U/V | native |
| NURBS-0139 | поднятие степени поверхности по U/V | native |
| NURBS-0140 | разбиение поверхности по изолинии | native |
| NURBS-0141 | обрезка прямоугольного параметрического домена | native |
| NURBS-0142 | перестановка U и V поверхности | native |
| NURBS-0143 | смена направления параметров поверхности | native |
| NURBS-0144 | разложение поверхности на рациональные патчи Безье | native |
| NURBS-0145 | удаление узлов поверхности с допуском | native |
| NURBS-0146 | понижение степени поверхности с допуском | native |
| NURBS-0147 | rebuild поверхности с допуском | native |
| NURBS-0148 | перенос шва периодической поверхности | native |
| NURBS-0149 | локальное изменение весов поверхности | native |
| NURBS-0150 | длина кривой с оценкой численной ошибки | native |
| NURBS-0151 | площадь поверхности с оценкой численной ошибки | native |
| NURBS-0152 | точка кривой на заданной длине | native |
| NURBS-0153 | равномерное деление кривой по длине | native |
| NURBS-0154 | ближайшая точка на кривой | native |
| NURBS-0155 | ближайшая точка на поверхности | native |
| NURBS-0156 | расстояние между двумя кривыми | native |
| NURBS-0157 | расстояние между кривой и поверхностью | native |
| NURBS-0158 | расстояние между двумя поверхностями | native |
| NURBS-0159 | кривизна и кручение пространственной кривой | native |
| NURBS-0160 | главные кривизны и направления поверхности | native |
| NURBS-0161 | параметрические изолинии поверхности | native |
| NURBS-0162 | пересечение кривой с плоскостью | native |
| NURBS-0163 | пересечение двух пространственных кривых | native |
| NURBS-0164 | пересечение кривой с поверхностью | native |
| NURBS-0165 | пересечение луча с поверхностью | native |
| NURBS-0166 | проверка самопересечений кривой | native |
| NURBS-0167 | проверка самопересечений поверхности | native |
| NURBS-0168 | проверка замкнутости UV-контура | native |
| NURBS-0169 | проверка простоты UV-контура | native |
| NURBS-0170 | проверка принадлежности UV-точки trim-области | native |
| NURBS-0171 | trim-область с внутренними отверстиями | native |
| NURBS-0172 | перенос пространственного контура в UV | native |
| NURBS-0173 | разбиение trim-контура на периодическом шве | native |
| NURBS-0174 | согласование степеней и узлов сечений | native |
| NURBS-0175 | согласование ориентации сечений | native |
| NURBS-0176 | согласование фазы замкнутых сечений | native |
| NURBS-0177 | проверка G0 между кривыми | native |
| NURBS-0178 | проверка G1 между кривыми | native |
| NURBS-0179 | проверка G2 между кривыми | native |
| NURBS-0180 | проверка G0 между поверхностями | native |
| NURBS-0181 | проверка G1 между поверхностями | native |
| NURBS-0182 | проверка G2 между поверхностями | native |
| NURBS-0183 | проверка регулярности поверхности | native |
| NURBS-0184 | оценка отклонения кривой после редактирования | native |
| NURBS-0185 | оценка отклонения поверхности после редактирования | native |
| NURBS-0186 | плоский offset кривой с допуском | native |
| NURBS-0187 | стыки offset-контура | native |
| NURBS-0188 | очистка самопересечений offset-контура | native |
| NURBS-0189 | плоская эквидистантная дорожка обработки | native |
| NURBS-0190 | параллельные сечения поверхности плоскостями | native |
| NURBS-0191 | адаптивная тесселяция кривой | native |
| NURBS-0192 | адаптивная тесселяция поверхности | native |
| NURBS-0193 | согласованная тесселяция общих границ патчей | native |
| NURBS-0194 | аффинное преобразование рациональной геометрии | native |
| NURBS-0195 | ограничивающий объём рациональной геометрии | native |
| NURBS-0196 | проверка ориентации нормалей оболочки | native |
| NURBS-0197 | проверка водонепроницаемости набора патчей | native |
| NURBS-0198 | сертификат непрерывного отклонения с учётом округления | native |
| NURBS-0199 | диагностика плохо обусловленной рациональной геометрии | native |
| NURBS-0200 | нормализация масштаба и единиц геометрии | native |

## Проверенный первый пакет

`engineering_profiles::rounded_rectangle`: полный размер по X/Y, четыре круговых угла, положительные прямые участки, замкнутый рациональный профиль из 17 контрольных точек.

`engineering_profiles::capsule`: горизонтальный паз с двумя полукруглыми концами, длина между центрами и радиус; 13 контрольных точек.

`engineering_profiles::annular_sector`: плоский кольцевой сектор с произвольной нормалью, внутренним/внешним радиусом и знаковым углом в градусах. U рациональный угловой параметр, V линейный радиальный параметр. Это патч, не замкнутое тело.

Профили лежат в плоскости XY, допускают последующее нативное affine-преобразование. Замыкание использует совпадающие концы clamped-базы, не periodic encoding. Круговые участки рационально точны в вещественной арифметике; сертификат погрешности binary64 не заявлен. Недопустимые размеры и вырожденные данные отклоняются.

Проверки 2026-10-01: `cargo test --manifest-path crates/Cargo.toml -p nurbs-core --no-default-features --quiet` — 282 теста и 2 doctest; после усиления проверки знакового угла — повторно 4 профильных теста. Нативный пример `engineering_profiles` выполнен успешно; он строит также стенку корпуса через extrusion. `node scripts/audit-nurbs-catalog.mjs` — 200 записей, ошибок нет. Проверки с codec/transport и новая сборка WASM в этом пакете не выполнялись.

## Supporting surfaces for coherent periodic regions, 2026-10-16

**195/200 remains unchanged.** The typed B-rep owner now saves the actual surface and original coherent UV context, replays seam and world-error admission, and rejects nonwrapped retained connectors. Exact positive boundary weight gauges are supported. World-aware UV refinement uses the existing shared proof budget and never changes the caller's world tolerance. **125 focused tests pass**, including the three multispan Boolean regressions, plus WASM checks for both crates and API/catalog checks. These are focused results, not a new full-repository regression. Surface regularity, injectivity, lifted 3D boundary edges and owned face assembly remain unqualified. [Evidence](../qualification/nurbs-periodic-surface-regions-2026-10-16/native-tests.json).

## Source-bound spherical bevel flank, 2026-10-16

**195/200 remains unchanged.** Rust now constructs spherical involutes and radial straight-bevel working flanks. Independent outward endpoint-jet/whole-control-net checks provide continuous source-bound error including rounding. An opaque owner enforces the requested world tolerance and replays the original source. Eleven focused checks pass (six new flank tests, three gear parameter tests, two planar involute regressions), along with both-crate WASM and API/catalog checks. Full bevel gear qualification remains planned; the 32-control surface limit requires multi-patch development for fine wide flanks. [Contract](nurbs-straight-bevel-flank.md), [evidence](../qualification/nurbs-bevel-flank-2026-10-16/native-tests.json).

## Original-parameter bevel patches, 2026-10-16

**195/200 remains unchanged.** Fine whole straight-bevel working flanks now partition the original certified cubic profile into radial surfaces, preserving global source parameters and exact common rows. The independent verifier reconstructs the complete outer source chain and requalifies world error, source coverage and radial scaling. Missing/reordered patches, damaged seams, altered radii and insufficient patch budgets are refused. Twelve focused tests pass, both crates check for WASM, and API/catalog checks pass. Nominal full-gear derivation, tooth outline and owned B-rep assembly remain planned. [Contract](nurbs-straight-bevel-flank.md), [evidence](../qualification/nurbs-bevel-patches-2026-10-16/native-tests.json).

## Nominal straight-bevel working flank, 2026-10-16

**195/200 remains unchanged.** Native Rust derives outward original-spec pitch/base/tip/root cone and roll boxes, cone-distance width, backlash half-thickness and pitch azimuth. The opaque working-flank owner now propagates nominal parameter uncertainty into a continuous saved-patch world error, replays the original gear spec, and enforces the unchanged tolerance. Fifteen focused tests pass, both crates check for WASM, and API/catalog checks pass. Whole-tooth placement, closed root/tip outline, original profile embedding and complete B-rep gear remain unqualified. [Contract](nurbs-straight-bevel-flank.md), [evidence](../qualification/nurbs-bevel-nominal-2026-10-16/native-tests.json).

## Paired nominal bevel tooth flanks, 2026-10-16

**195/200 remains unchanged.** Native Rust places both working flanks for every tooth, including nominal pitch thickness/backlash, reflection and tooth phase. The complete inventory is owned and independently requalified against every original saved pole, basis and nominal phase interval. Ideal first-quadrant source sectors and exact patch seams are checked; changed side/count, missing flanks, damaged poles/weights and insufficient surface budgets are refused. Eighteen focused tests pass, both crates check for WASM, and API/catalog checks pass. Root/tip closure, actual complete-profile embedding and owned B-rep gear admission remain unqualified. [Contract](nurbs-straight-bevel-flank.md), [evidence](../qualification/nurbs-bevel-teeth-2026-10-16/native-tests.json).

## Closed spherical bevel profile, 2026-10-16

**195/200 remains unchanged.** Native Rust now closes the original paired working flanks with rational tip/root parallels and, where required, meridian root transitions. Original source curves and exactly common canonical endpoints are retained. The whole original-spec profile certificate independently bounds every transition pole/weight and checks complete source-role ownership and closure. Twenty-one focused tests pass, both crates check for WASM, and API/catalog checks pass. Actual complete-profile embedding and owned cap/bore/B-rep solid assembly remain unqualified. [Contract](nurbs-straight-bevel-flank.md), [evidence](../qualification/nurbs-bevel-outline-2026-10-16/native-tests.json).

## Native straight-bevel body candidate, 2026-10-16

**196/200: 79 integrated + 117 native; four planned remain.** NURBS-0086 now has a typed strict native body constructor owning the independently embedded original spherical profile, actual matching outer model edges, inclusive nominal wall/axis-fan-cap error and complete continuous shared-edge closure. The default 20-tooth constructor passes at 1e-6; 6/20-tooth body candidates pass closure, positive volume against an independent formula, pole corruption and too-small tolerance tests. Original 20/100-tooth projection checks and all five planar modular-profile regressions also pass (separate compiled snapshot). Both crates check for WASM. Like the four prior native gear families, the object explicitly reports global solid geometry unqualified. Fan caps, no bore, proof budgets and the 1024-face limit are declared in the [contract](nurbs-straight-bevel-flank.md). [Body evidence](../qualification/nurbs-bevel-body-2026-10-16/native-tests.json), [profile evidence](../qualification/nurbs-bevel-embedding-2026-10-16/native-tests.json).

## Axial ZA worm working surface, 2026-10-16

**196/200 remains unchanged.** A separate native Rust worm module derives axial trapezoidal rack parameters, diameter quotient, pitch/lead and lead-angle boxes, then qualifies every original saved helical patch against nominal source data. It does not alias the 60-degree thread. Fifteen focused tests pass, including both hands, multiple starts/turns, actual source/geometry corruption, the full 8-start/16-turn census (2560 patches), and shared thread/helical/gear parameter regressions. Both crates check for WASM; the API map has 179 modules and catalog checks pass. Finite trim/cap/sewn-body qualification remains open, so NURBS-0087 stays planned. [Contract](nurbs-worm-za.md), [evidence](../qualification/nurbs-worm-patches-2026-10-16/native-tests.json).

## Native finite ZA worm, 2026-10-16

**197/200: 79 integrated + 118 native; three planned remain.** NURBS-0087 now has its own strict typed native finite ZA body constructor: original axial-rack source accuracy, complete retained/excluded affine clipping and UV isotopy, exact represented disk-cap profile proofs, original actual wall binding and final continuous shared-edge agreement. Both hands, axial extent longer than a full lead and two starts pass independent volume/source checks. Twenty-six core tests and eight distinct B-rep tests pass, plus the extended worm cases and both-crate WASM/API/catalog checks. The existing finite-thread native criterion is unchanged: global saved-solid geometry and combined ideal finite-body error remain explicitly unqualified. [Contract](nurbs-worm-za.md), [trim evidence](../qualification/nurbs-worm-trim-2026-10-16/native-tests.json), [body evidence](../qualification/nurbs-worm-finite-2026-10-16/native-tests.json).

## ZA worm-wheel contact foundation, 2026-10-16

**197/200 remains unchanged.** Native Rust derives the original ZA generating-motion contact equation, distinguishes strict exclusions from uncertain tangencies, and independently brackets both simple roots per period at each queried source point. Original hand/flank/start data, normal/velocity boxes and wheel coordinates are replayable; no sampled residual authorizes a certificate. Twelve focused tests pass, including an independently differentiated motion triple product and original multi-start binding, plus both-crate WASM/API/catalog checks. Complete envelope sheet coverage, NURBS approximation and owned wheel geometry remain open, so NURBS-0088 stays planned. The API map contains 180 modules. [Contract](nurbs-worm-wheel-envelope.md), [evidence](../qualification/nurbs-worm-wheel-contact-2026-10-16/native-tests.json).

## Whole ZA wheel contact-angle charts, 2026-10-16

**197/200 remains unchanged.** The meshing equation now solves source roll directly in a contact-angle chart, crossing inverse angular-root folds. Outward automatic differentiation bounds original positions and Jacobians on whole requested rectangles. A private adaptive partition replays full source coverage and nonzero Jacobian receipts; resource/denominator/singular failures remain unqualified leaves. Seventeen focused tests pass, including independent derivative checks, complete full-U covers through beta +/-0.3 and original multi-start phase, plus WASM/API/catalog gates. Certified NURBS interpolation and complete wheel/stock/B-rep qualification remain open. [Contract](nurbs-worm-wheel-envelope.md), [evidence](../qualification/nurbs-worm-wheel-chart-2026-10-16/native-tests.json).

## Certified quintic wheel envelopes and exact packing, 2026-10-16

**197/200 remains unchanged.** Native Rust now builds original-source quintic tensor NURBS grids with continuous sixth-derivative/Lagrange remainder and actual saved-pole error bounds. Full source domains, actual patch regularity and exact C0 seams replay independently. Lossless multi-span packing copies original controls/knots: the verified default descending example packs 32 cells into two surfaces with source error <=1.0773757602446802e-8 at requested 1e-4. Sixteen focused integration/math checks pass, plus WASM/API/catalog gates. Stock/root/tip, whole-tooth placement, global embedding and wheel B-rep remain open. The API map now has 181 modules after registering an unrelated normal-trace query module. [Contract](nurbs-worm-wheel-envelope.md), [evidence](../qualification/nurbs-worm-wheel-fit-2026-10-16/native-tests.json).


## Original worm-wheel stock intersections and NURBS fits, 2026-10-16

**197/200**, unchanged. A private replayable Rust proof establishes one original-envelope/stock-cylinder crossing for every angle using a complete Cartesian derivative cover and original endpoint signs. It identifies the cylindrical blank material side, encloses implicit edge positions/tangents and supplies sixth-order derivatives for the true crest-to-stock chart. `fit_stock_chart` retains this original implicit source through the existing quintic error verification and lossless C0 packing. The tested default strip uses one Surface per flank (error <=2.40e-8 at tolerance 1e-3); a wide left-handed two-start source uses four packed Surfaces (error <=4.48e-9 at tolerance 1e-4).

**28 final focused checks pass**, including the three former Boolean failures, exact source phase, altered poles/radius, invalid domains, proof budgets and independent implicit Taylor algebra. Both crates compile for WASM; explicit changed-file formatting, the 182-module API map and catalog audit pass. Full compiled native snapshots pass **1941 core + 1305 B-rep tests and two doctests**, with three explicitly ignored B-rep diagnostics. The initial stale multispan-injectivity expectation was corrected with shape-preservation and actual-backtracking checks. Later stock-source changes are covered by the final focused/WASM gates; the full snapshot is not relabelled as a frozen current-tree build.

Generated root transitions, stock-tip/axial boundaries, replication and the complete wheel body remain open. General surface/surface intersections and periodic owned faces are the other two planned entries. [Contract](nurbs-worm-wheel-envelope.md), [evidence](../qualification/nurbs-worm-wheel-stock-2026-10-16/native-tests.json).


## Generated worm-wheel root transitions, 2026-10-16

**197/200**, unchanged. Rust now owns the original convex crest-edge and crest-cylinder envelopes, with whole-domain normal-cone/unique-contact proofs, analytic regularity lower bounds and original-chart injectivity. Sixth derivatives feed the shared quintic fitter. Each default root part for either hand, beta [-0.15,0.15], uses one NURBS Surface at requested tolerance 1e-4; largest error upper is 4.982e-5. Distinct working-flank/root owner types preserve actual source roles. Packing replay checks knot/control/weight bits, including signed zero.

**34 distinct focused checks pass**, plus both-crate WASM, explicit changed-file formatting, the 182-module API map and catalog audit. Current root-source replay detects a changed phase despite a phase-invariant contact proof. Earlier full native results remain compiled-snapshot evidence; they are not relabelled as a full run of these new sources. Stock-tip/finite axial boundaries, tooth placement, actual cross-family stitching and the closed wheel B-rep remain open. NURBS-0088 is still planned. [Contract](nurbs-worm-wheel-envelope.md), [evidence](../qualification/nurbs-worm-wheel-root-2026-10-16/native-tests.json).


## Finite physical-Y worm-wheel side templates, 2026-10-16

**197/200**, unchanged. Rust now has all six finite side families for one tooth period: two original generated root transitions, crest-cylinder gap, two true crest-to-stock working flanks and cylindrical stock tip. Explicit physical-Y domains replace variable-height angular ends. Actual NURBS end controls lie exactly on the requested planes; interior Y rounding is retained in the source error. The default Y=[-2,2], stock R=21 case passes unchanged tolerance 1e-4: root parts pack into one Surface each; each working flank and tip pack into three. Whole-domain stock root/rank proofs, constrained-circle sixth derivatives and actual source ownership replay are implemented.

**47 distinct focused checks pass** (36 integration, 8 math/ownership, 3 Boolean), plus both-crate WASM, explicit formatting, API map182 and catalog audit. Existing angular/stock APIs retain their regressions. This is not a closed wheel: common actual controls, periodic placement, cap profiles and owned B-rep remain. Full previous suite results are retained as compiled-snapshot evidence, not relabelled as a full run of the new sources. [Contract](nurbs-worm-wheel-envelope.md), [evidence](../qualification/nurbs-worm-wheel-axial-2026-10-16/native-tests.json).

## Complete finite generated worm-wheel candidate, 2026-10-16

**198/200**. NURBS-0088 now owns all40 periods,240 original side grids,720 packed supports and two continuously admitted physical planar caps. Actual controls are shared after nominal placement and requalified. The connected manifold candidate has722 faces,1680 edges and960 vertices; complete current saved-edge closure passes. Independent volume checks give5066.9680793 and5066.9662757 mm³. **34 focused checks pass**, plus current both-crate WASM compilation. Source/cap/topology/proof limits and global-solid/ideal-body limitations are explicit. General surface/surface coverage and periodic supporting owned faces remain planned. [Contract](nurbs-worm-wheel-envelope.md), [evidence](../qualification/nurbs-worm-wheel-caps-2026-10-16/native-tests.json). Historical full-suite results above remain snapshots; no full current-tree run is claimed.

## Original-support periodic faces and full corner ownership, 2026-10-16

**198/200**, unchanged. Admitted coherent periodic UV regions now produce owned faces on the unchanged original supporting surface, with complete original-chart injectivity/rank covers, continuous actual world-edge lifts, shared generated seam edges and manifold vertex links. A rational torus with unequal U/V domains passes four-face/four-seam corner ownership. Nonlinear rational pcurve composition, preserved multi-span paths and correlated original-parameter UV error bounds are implemented. **172 focused checks pass**, plus both-crate WASM, explicit owned-file formatting, API map182 and catalog audit. General nonlinear alias correspondence and inter-wire contact regularization still keep NURBS-0173 planned; general surface/surface coverage keeps NURBS-0030 planned. [Contract](nurbs-periodic-winding-regions.md), [evidence](../qualification/nurbs-periodic-owned-faces-2026-10-16/native-tests.json). Earlier full native suite totals remain historical snapshots.

## Uniform original-source intersection fibers, 2026-10-16

**198/200**, unchanged. A typed owned Krawczyk operator now proves a unique original-system root for every value in a complete free-parameter interval, with actual weighted contraction, strict box inclusion and original source ownership. Correlated empty boxes, rational roots, knot coverage, actual periodic seam data, tangencies, false preconditioners and singular point-image cases are independently tested. **34 focused checks pass**, plus both-crate WASM, explicit new-file formatting, API map182 and catalog audit. This does not authorize general full-domain coverage, branch-atlas/graph topology or world traces; NURBS-0030 remains planned. [Contract](nurbs-surface-surface-coverage.md), [evidence](../qualification/nurbs-ss-fibers-2026-10-16/native-tests.json).

## Complete original complement coverage around one fiber, 2026-10-16

**198/200**, unchanged. The whole original four-parameter domain can now be qualified around an owned uniform fiber by excluding every closed complement slab and retaining/replaying a complete subdivision tree. A nonlinear cubic case needs64 nodes,35 excluded leaves and190 original rectangles. A second real branch, a real natural-boundary branch, partial free intervals, missing work and forged/dropped proof data cannot inherit completeness. **26 focused checks pass**, plus both-crate WASM, explicit new-file formatting, API map182 and catalog audit. Multi-fiber/chart coverage, physical branch graphs/world traces and trim topology still keep NURBS-0030 planned. [Contract](nurbs-surface-surface-coverage.md), [evidence](../qualification/nurbs-ss-fiber-coverage-2026-10-16/native-tests.json).

## Multiple original root fibers, 2026-10-16

**198/200**, unchanged. Added an owned, replayable union of disjoint same-axis
root tubes with full complement coverage and shared proof budgets. Both branches
of a quadratic example qualify together; duplicate tubes, mixed surfaces,
partial second intervals and exhausted budgets cannot inherit coverage. The old
single-fiber result remains semantically separate. **28 focused checks pass**
(24 integration, one corruption audit, three former Boolean failures), plus
both-crate WASM and catalog/API-map audits. General chart transitions and owned
physical graph/trim admission remain open. Evidence:
`docs/qualification/nurbs-ss-fiber-atlas-2026-10-16/`.

## Exact nonlinear source-family correspondence, 2026-10-16

**198/200**, unchanged. Rust now owns positive projective correspondence between
original rational Bezier curves, exact restrictions at every saved seam interval,
and synchronized source/candidate family homotopies. Coherent quotient winding
accepts these separately replayed receipts and retains the actual original
multiplicity. Different degrees, reversed traversal, nonlinear seams and malformed
receipts are checked without weakening normalized-function identity. General
multi-span source correspondence and inter-wire contact regularization still keep
NURBS-0173 planned. Evidence is under
`docs/qualification/nurbs-projective-homotopy-2026-10-16/`.

The projective path now reaches original-source owned B-rep regions and actual
supporting-surface faces. The exact nonlinear seam fixture produces2 faces and
one shared shore on the unchanged closed square-tube support. Current focused
B-rep19 cases and2 private source-proof audits pass; three former Boolean failures
pass again. Full integration snapshot194 suites/952 cases completed successfully
before these additions; current new code additionally passes the full1371-case
NURBS library and both-crate WASM. These scopes remain separate in evidence.
General multi-span nonlinear source maps and inter-wire contact regularization
keep NURBS-0173 planned; catalog remains198/200.

## Original B-spline phase correspondence, 2026-10-16

**198/200**, unchanged. Exact projective source correspondence now covers
complete clamped multi-span B-spline bases, including actual degree25 and
C0/C1/C2 cases, with independent position/chain-rule oracles and exact exponent
gauges. A source triangle reaches owned actual-support geometry with1 face,
3 edges and3 vertices. Original-control directional homotopy bounds remove the
old axis-box-only anchor refusal without admitting a genuine anchor crossing.
Current gates: full1372-case NURBS library,62 predicates,124 periodic integration
cases and20 B-rep region/face cases; both-crate WASM passes. API ownership now
covers183 public modules after registering the existing surface_collar analysis
module. Historical full integration snapshots remain separately scoped.
Evidence: `docs/qualification/nurbs-projective-bspline-2026-10-16/`.

## Exact original partition functions, 2026-10-16

**198/200**, unchanged. Added a native original-basis restriction predicate and
private source-owned exact partition receipt. C1 B-splines can now be checked
against differently graded/rational pieces on all original intervals, including
cuts inside original knot cells. The proof clears exact de Boor denominators and
compares homogeneous polynomial coefficients; source data and requested accuracy
are unchanged. This is a prerequisite for admitting preserved source contacts,
not a new catalog figure or a completed contact regularizer. Current periodic
regressions128, raw predicate tests63 and the private corruption replay pass;
both-crate WASM and API-map184/catalog audit are checked separately in evidence.
