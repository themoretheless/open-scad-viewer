# Приоритетные 200 возможностей NURBS в Rust

Сохранены приоритетные наборы 150 и 180 возможностей; рубеж пользователя 160 достигнут внутри набора 180. Этот реестр сохраняет более широкий план из 200 пунктов. Дальняя цель 1200 сохраняется. Размеры, радиусы, число зубьев, варианты осей и одинаковые алгоритмы под разными именами не считаются отдельными возможностями. В набор входят формы и самостоятельные операции CAD; это не 200 разных примитивов.

Нативная готовность требует типизированного Rust API, независимых геометрических проверок, отказов на некорректных данных и описанных ограничений. WASM, TypeScript, Rush и редактор не являются условиями нативной готовности. Приближённая операция обязана явно различать выборочную оценку, непрерывную границу ошибки и учёт округления.

Текущее состояние каталога: 189 реализованных возможностей (79 integrated + 110 native), 11 planned. Рубеж 160 достигнут внутри сохранённого набора 180.

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
| NURBS-0030 | пересечение поверхностей | planned |
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
| NURBS-0080 | оболочка заданной толщины | planned |
| NURBS-0081 | прямозубая | planned |
| NURBS-0082 | косозубая | planned |
| NURBS-0083 | шевронная | planned |
| NURBS-0084 | внутренняя | planned |
| NURBS-0085 | рейка | native |
| NURBS-0086 | коническая шестерня | planned |
| NURBS-0087 | червяк | planned |
| NURBS-0088 | червячное колесо | planned |
| NURBS-0089 | наружная резьба | planned |
| NURBS-0090 | внутренняя резьба | planned |
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
| NURBS-0163 | пересечение двух пространственных кривых | planned |
| NURBS-0164 | пересечение кривой с поверхностью | planned |
| NURBS-0165 | пересечение луча с поверхностью | native |
| NURBS-0166 | проверка самопересечений кривой | native |
| NURBS-0167 | проверка самопересечений поверхности | native |
| NURBS-0168 | проверка замкнутости UV-контура | native |
| NURBS-0169 | проверка простоты UV-контура | native |
| NURBS-0170 | проверка принадлежности UV-точки trim-области | native |
| NURBS-0171 | trim-область с внутренними отверстиями | native |
| NURBS-0172 | перенос пространственного контура в UV | native |
| NURBS-0173 | разбиение trim-контура на периодическом шве | planned |
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
| NURBS-0188 | очистка самопересечений offset-контура | planned |
| NURBS-0189 | плоская эквидистантная дорожка обработки | native |
| NURBS-0190 | параллельные сечения поверхности плоскостями | native |
| NURBS-0191 | адаптивная тесселяция кривой | native |
| NURBS-0192 | адаптивная тесселяция поверхности | native |
| NURBS-0193 | согласованная тесселяция общих границ патчей | native |
| NURBS-0194 | аффинное преобразование рациональной геометрии | native |
| NURBS-0195 | ограничивающий объём рациональной геометрии | native |
| NURBS-0196 | проверка ориентации нормалей оболочки | native |
| NURBS-0197 | проверка водонепроницаемости набора патчей | planned |
| NURBS-0198 | сертификат непрерывного отклонения с учётом округления | native |
| NURBS-0199 | диагностика плохо обусловленной рациональной геометрии | native |
| NURBS-0200 | нормализация масштаба и единиц геометрии | native |

## Проверенный первый пакет

`engineering_profiles::rounded_rectangle`: полный размер по X/Y, четыре круговых угла, положительные прямые участки, замкнутый рациональный профиль из 17 контрольных точек.

`engineering_profiles::capsule`: горизонтальный паз с двумя полукруглыми концами, длина между центрами и радиус; 13 контрольных точек.

`engineering_profiles::annular_sector`: плоский кольцевой сектор с произвольной нормалью, внутренним/внешним радиусом и знаковым углом в градусах. U рациональный угловой параметр, V линейный радиальный параметр. Это патч, не замкнутое тело.

Профили лежат в плоскости XY, допускают последующее нативное affine-преобразование. Замыкание использует совпадающие концы clamped-базы, не periodic encoding. Круговые участки рационально точны в вещественной арифметике; сертификат погрешности binary64 не заявлен. Недопустимые размеры и вырожденные данные отклоняются.

Проверки 2026-10-01: `cargo test --manifest-path crates/Cargo.toml -p nurbs-core --no-default-features --quiet` — 282 теста и 2 doctest; после усиления проверки знакового угла — повторно 4 профильных теста. Нативный пример `engineering_profiles` выполнен успешно; он строит также стенку корпуса через extrusion. `node scripts/audit-nurbs-catalog.mjs` — 200 записей, ошибок нет. Проверки с codec/transport и новая сборка WASM в этом пакете не выполнялись.
