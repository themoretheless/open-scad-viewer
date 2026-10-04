# Sweep: контракты и критерии завершения

Эта матрица уточняет полный объём из `sweep-coverage-2026-10-01.md`. Она не
заменяет цель набором уже реализованных функций. История проверок остаётся в
исходном документе; строки ниже фиксируют контракт и недостающее доказательство.

## Независимые гарантии

- E: непрерывное расстояние между авторским семейством сечений и сохранённой
  геометрией, включая ошибку вычисления сечений и направленное округление.
- R: ненулевая касательная профиля и невырожденный Якобиан поверхности на всём
  диапазоне. Проверка отдельных контролов/станций недостаточна.
- I: глобальная инъективность, отсутствие запрещённых пересечений и правильное
  взаимное расположение наружной оболочки и отверстий. Валидная incidence
  topology и замкнутый display mesh эту гарантию не дают.
- J: заявленный C0/G1/G2 шов, подтверждённый односторонними производными,
  регулярностью и непрерывной проверкой полного шва.

Состояния сертификата: доказано; нарушение с проверяемым свидетельством;
не доказано с причиной (бюджет, численный диапазон, вырожденность). Исчерпание
бюджета никогда не даёт успешного сертификата или верхней границы для всей
непроверенной геометрии. Для multi-profile запроса `knownProfileErrorUpper`
сохраняет максимум уже доказанных оценок отдельных профилей: он может
обосновать консервативный отказ по бюджету, но не сертифицирует остальные
профили. Полный `continuousErrorUpper` остаётся null, пока не доказан каждый
профиль; scope относится к retained patches и не включает крышки.

## Явные бюджеты progressive конструктора

Ordered profiles: 1..64 curves. Body inputs: 1..16 непустых loops с суммарным
лимитом64 curves/spans; face budget1024 дополнительно ограничивает refinement.
Coarse section budget1025, fine probe budget4097. Miter: 2..17 open sites или
3..16 cyclic sites без повторённого последнего site; steps считаются на каждый
edge, поэтому steps × edges +1 должны укладываться в section budget.
Scalar-law certificate: 0..100000 cells, original nonempty knot spans считаются
ячейками; бюджет0 даёт `Unresolved`, а не успех. Бюджеты поверхности,
пересечений, кадров и общего refinement должны оставаться отдельными и явно
передаваться через все слои при их интеграции.

## Режимы

В графе все рациональные scalar/vector laws имеют собственные knot domains.
Twist: радианы native, градусы Rush/TS. Размерные sites/center/допуски: mm.
Ни одна строка не получает E/R/I/J автоматически из зелёного теста.

| Режим | Поддерживаемые входы и геометрия | Условия отказа | Текущие E / R / I / J | Проверки и требуемое расширение |
| --- | --- | --- | --- | --- |
| Translation `surface_sweep` | Rational profile + rational path, постоянная ориентация | Некорректные curves, ресурсные ограничения | Точная конструкция; общий R/I/J не доказан | `tests/nurbsTranslationSweep.test.ts`; подключить аудит результата |
| `scaled_sweep` | Рациональный профиль, путь и положительный scalar scale, явный центр | Неположительный scale, несовместимый ввод | Точная rational construction; общий R/I/J не доказан | `tests/nurbsScaledSweep.test.ts`, `tests/nurbsScaledSweepLanguage.test.ts` |
| `twist_sweep` | Рациональный профиль и вращение вокруг заданной оси | Вырожденная ось, лимиты конструктора | Общие E/R/I/J не доказаны | `tests/nurbsTwistSweep.test.ts`; отдельная непрерывная квалификация |
| `two_guide_sweep` | Два authored affine rails, width и transverse axes | Некорректные rails/axes/width | Точная bounded affine family; произвольное соответствие и общий R/I/J не доказаны | `tests/nurbsTwoGuideSweep.test.ts`; arbitrary correspondence correction остаётся |
| `profile_sweep` / `framed_sweep` | Rational profile, RMF path; scalar scale для profile | Несоблюдённый refinement budget, несовместимое замыкание | Sampled E; общий R/I/J не доказан | `tests/nurbsProfileSweep.test.ts`, `tests/nurbsFramedSweepGraph.test.ts` |
| Progressive RMF | Ordered profiles, rational path, совместные scale/twist | Неопределённый frame, бюджет, несовместимая seam | Sampled E; R/I/J не доказаны | `tests/nurbsProgressiveSweep.test.ts`; сертифицировать транспорт кадров |
| Progressive fixed | Те же laws, постоянный authored frame; C0 corners допустимы | Бюджет, плохой ввод/замыкание | Sampled E; R/I/J не доказаны | `tests/nurbsProgressiveSweep.test.ts`; аудит всех corner intervals |
| Progressive fixed_normal | Path tangent + authored normal | Parallel normal, бюджет, seam mismatch | Sampled E; R/I/J не доказаны | `tests/nurbsProgressiveSweep.test.ts`; интервальное отделение cross product от нуля |
| Progressive Frenet | Ненулевая кривизна пути | Undefined principal normal, бюджет/seam mismatch | Sampled E; непрерывная R не доказана | `tests/nurbsProgressiveSweep.test.ts`; доказать speed/curvature на всём пути |
| Progressive corrected_frenet | Principal normal со sign-continuity и RMF fallback | Невозможность вычисления frame, бюджет/seam mismatch | Sampled E; переходы fallback и R/I/J не доказаны | `tests/nurbsProgressiveSweep.test.ts`; квалифицировать переходы непрерывно |
| Progressive authored | Frame axis/normal rational laws независимо от guide tangent | Отсутствующий закон, degenerate axes, бюджет/seam mismatch | Open parameter retained-patch E с округлением/decomposition доказан при полном бюджете; whole-law frame regularity отдельно. Полные body E/R/I/J не доказаны | `tests/nurbsAuthoredPatchError.test.ts`, `tests/nurbsAuthoredPatchDecomposition.test.ts`, `tests/nurbsAuthoredFrameRegularity.test.ts`; arc_length, closed correction и caps остаются |
| Progressive affine laws | Positive axis_scale, center_law совместно с frame/scale/twist | Неположительная ось масштаба, плохие units/laws, бюджет/seam mismatch | В authored open parameter E включены исходные affine laws; guided E тоже включает их и проверен через WASM/Rush. Прочие режимы и полный R/I/J открыты | Независимые domains и rational multispan проверены authored public suite; guided public regression прошёл |
| Progressive orientation/contact guide | Дополнительная rail, shared contact anchor; parameter/arc_length mapping | Parallel guide, incomplete anchor, неподдержанное correspondence, бюджет | Native open parameter orientation-guide retained-patch E включает original frame jets, station rounding и decomposition и участвует в refinement. Orientation-guide WASM/Rush квалифицированы выбранными public cases. Contact E относительно исходного fitted transport включает anchor/fit/station/decomposition и admission; WASM/Rush проверены public suite на SHA42739c3f. Точное непрерывное совпадение с rail, arc_length/closed correction и полный R/I/J не доказаны | `tests/nurbsGuidedPatchError.test.ts`: orientation/contact прошли; correction и непрерывная contact identity qualification остаются |
| Open/closed round polyline | Rational circular corner arcs; spatial sites | Reverse/zero edges, overlap, radius consuming edge | Геометрический радиус проверен; полный E/R/I/J не доказан | `tests/nurbsRoundPolyline.test.ts`; доказать G1, регулярность и отсутствие пересечений |
| Open/closed transition polyline | Quintic blends, endpoint second derivatives zero | Reverse/zero edges, overlap, setback consuming edge | Real-arithmetic endpoint construction; округление и полный J/R/I не доказаны | `tests/nurbsBezierPaths.test.ts`, `tests/nurbsRoundPolyline.test.ts`; весь G2 seam |
| Constant open/closed miter | Retained rational profile loops, polyline; extrusion correspondence | Miter limit, reverse/consumed edge; closed nonzero holonomy | C0 retained seam; R/I и округление не доказаны | `tests/nurbsMiterSweep.test.ts`; sharp corner не обещает G1/G2 |
| Progressive open/closed miter | Polyline, scalar scale/twist, distributed closed holonomy | Law continuity/positivity, phase/refinement/face budget, invalid closure, unresolved E/R | Outward-certified retained section-interpolation E и whole-domain profile/wall R управляют acceptance; полный B-rep E/I/J ещё не доказан | `tests/nurbsProgressiveMiter.test.ts`, native `progressive_miter::tests`; аудит decomposition/caps и глобальной геометрии остаётся |
| Affine/frame/guide miter | Positive axis scale и локальный center с независимыми domains; authored frame и guide variants ещё требуются | Axis positivity, law continuity, closed endpoint agreement, бюджеты E/R и face/refinement | Axis/center подключены к native generator, интервальному E, WASM и Rush; hollow Rush-пример имеет positive retained caps, volume и повторный Solid admission по артефакту. Полный E, frame/guide и live UI ещё открыты | `examples/rush/miter-affine-hollow.r`, 12 публичных тестов на финальном WASM и independent STEP; живые viewport/Solid действия ожидаются |
| Surface/body output | Rational patches; open caps или closed periodic shells, holes | Face/control budgets, invalid contour/cap/closure | Topology проверяется отдельно от R/I/J | `tests/brepRationalSweep.test.ts`; сертификаты геометрии должны сохраняться при B-rep construction |

Каждый progressive режим требует обеих station policies: parameter и arc_length,
open/closed cases, одиночного профиля и ordered outer/inner loops. Матрица
пространства сценариев не считается полной до квалификации каждой применимой
комбинации. Неприменимый режим должен быть явно отказан, а не молча заменён.

## Восемь итоговых ворот

| Требование | Что является доказательством завершения | Текущее состояние |
| --- | --- | --- |
| 1. Матрица | Для каждого режима есть входы, гарантии E/R/I/J, отказы и evidence; новые режимы добавляются сюда | Контракт зафиксирован; полный набор oracle/визуальных свидетельств ещё открыт |
| 2. E | Interval arithmetic покрывает laws, frames, shear, station construction и retained interpolation; acceptance использует только доказанный upper bound ≤ tolerance | Open parameter authored sweep: Rust/WASM bound для retained patches включает original frame/twist/scale/affine laws, rounding и decomposition; публичные регрессии прошли. Полная граница отдельных miter fixtures включает заполненные крышки. Arc-length, closed correction, все режимы и полная body-гарантия остаются открыты. Агрегация partial-profile bounds проверена native/WASM, включая sync/async Rush refusal. Native value enclosure покрывает knot transitions всех шести authored laws и их совместный rational случай; WASM с этим расширением собирается |
| 3. R | Whole-domain profile speed и surface Jacobian certificates, bounded unresolved | Progressive miter profile/wall R подключены к native acceptance и WASM/worker; остальные режимы, caps и configurable budgets открыты |
| 4. I | Bounded pair hierarchy, allowed adjacent boundaries, intra/inter-shell intersections и containment | Native/WASM wall audit с явными chart/pair budgets, pair BVH, sufficient exact shared-plane boundary certificates и whole-chart injectivity criteria квалифицирован; отчёт автоматически сохраняется при сборке miter, contour ownership подключён; strict I-admission, caps и containment ещё открыты |
| 5. J | Проверенные односторонние jets на всех smooth joins/closed seams; sharp miter сохраняет sharp contract | Native/WASM bounded jet audit квалифицирован на translation round/transition и cyclic round; exact G1/G2, moving frames и Rush/viewport отчёт ещё открыты |
| 6. Miter laws | Каждая affine/frame/guide комбинация проходит native → WASM → Rush → preview → Solid и те же E/R/I/J | Не завершено |
| 7. STEP | Независимое CAD-ядро импортирует fixtures и сверяет geometry, orientation, seams, holes и topology в объявленном допуске | 38 выбранных случаев проверены OCCT на предыдущем артефакте, включая moving authored frame + guide + affine + hollow + authored caps; независимая retained-generator volume reference. Новый patch-error WASM требует повторной квалификации. Все применимые комбинации и полная матрица остаются открыты |
| 8. UI | Wide/narrow snapshots и реальные worker cases: preview, refusal, cancel, source replacement, restoration и Solid для полной применимой матрицы | Базовые miter/ordinary/contact проверены; вся матрица ещё открыта |

Основные сквозные проверки: `tests/geometryWorkerRealBoundary.test.ts`,
`tests/geometryWorkerProtocol.test.ts`, `tests/buildCoordinator.test.ts`.
Положительный сертификат E не превращает автоматически открытый profile в solid
и не повышает результат до R/I/J. Текущий полный объём остаётся активным.

Cap contact evidence now includes per-coedge native continuous cap/world-edge
agreement and shell-oriented two-use incidence (`nurbsSweepCapWallAudit.test.ts`).
These are independent diagnostic flags; exact boundary ownership, wall-pcurve
agreement and global shell embedding are still unproved. Shared per-cap product
budget exhaustion leaves the affected edge reports unresolved.

Wall-coedge continuous composition now accompanies cap-coedge agreement in
cap contact evidence. Cell-budget exhaustion is unresolved; a certified mismatch
retains its witness enclosure. Tolerance agreement on both sides still does not
prove exact shared geometry or global embedding.

Exact stored coedge composition is now independently observable through
`sweep_coedge_exact_audit` and per-cap contact evidence. Exact equality,
difference, unsupported representation and resource exhaustion remain distinct.
A positive tolerance result never substitutes for exact identity.

Coordinate-plane cap construction now selects natural-coordinate charts only
when whole-boundary exact composition, continuous deviation and chart R/I all
pass. Other caps, including resource-limited exact candidates, retain bounded
projection and no exact-identity claim. Nonunit clamped domains are supported by
the native bilinear cap deviation verifier.

Exact composition now avoids redundant polynomial products on coordinates
proved identical and constant at all curve/surface poles. Rational weights must
remain positive; a one-ULP difference keeps the general predicate. This reduces
budget consumption without changing geometric acceptance tolerances.

Full wall-side image coverage is independently checked through
`sweep_boundary_coverage_audit` and reported per cap contact. Positive weights,
continuous clamped traversal, exact fixed-coordinate controls and full endpoint
attainment are required. Complete image coverage does not establish injectivity
or trimmed cap ownership.

Coordinate-plane cap versus retained wall-chart contacts now have a combined
native certificate from actual B-rep data: cap R/I + trim region + shell-oriented
edge pairing + exact cap/wall coedge composition + complete side coverage +
whole-wall plane exclusion. Allowed pairs are confined to the common edge.
This is diagnostic in progressive miter reports. Wall/wall embedding, cap/cap
relations and shell containment still prevent a global body guarantee.

Cap/cap separation is now independently audited over full retained charts and
reported through progressive miter sync/stream/Rush construction. Budgets and
unresolved IDs are explicit. Whole-chart overlap is conservative unresolved;
trim-aware refinement for such caps is still incomplete. Full wall embedding
and shell containment continue to block global body certification.

## Проверенные геометрические тела — 2026-10-02

Эти результаты относятся к конкретной retained-геометрии; общий E/R/I/J всех
режимов остаётся открытым. Источник: `external-step-spatial-volume/manifest.json`
и `sweep-spatial-volume-occt.log` в qualification-папке.

| Пример | Общий exact-work / coedge | Сертификат геометрического объёма | Независимое согласие OCCT с материалом |
| --- | --- | --- | --- |
| Progressive scale/twist, отверстие | 831431 / 144 equal | Доказан | Подтверждено |
| Spatial, исходные сечения | 596447 / 104 equal + 8 different | Не доказан | Не заявляется |
| Spatial, явная ограниченная коррекция конечного сечения | 614347 / 112 equal | Доказан | Подтверждено |
| Closed planar, полость | 784604 / 128 equal | Доказан | Подтверждено |

Во всех примерах exact coedge используют один бюджет 1000000 без сброса между
coedge. Spatial-коррекция имеет отдельную доказанную верхнюю границу смещения
кривой 4.370307162299816e-13; она применяется совместно к стенкам и крышке.
Это не доказательство полного authored `continuousBound`. Перенос коррекции
в обычную конструкцию Rush с учётом полного E ещё не выполнен. Инъективность
spatial-граней использует общий лимит 20000 interval cells; контактные и
ориентационные этапы имеют собственные явные бюджеты.

Публичные WASM-тесты требуют boundaryEmbeddingCertified и
solidGeometryCertified исправленного spatial-fixture. Независимый STEP-анализ
подтверждает BRep-валидность, топологию, объём и согласие native shell roles и
ориентаций с OCCT. Эти четыре примера не покрывают всю STEP/UI-матрицу.
