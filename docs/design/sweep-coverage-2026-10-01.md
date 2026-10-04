# Полное покрытие sweep

Цель: собственная CAD sweep-библиотека в текущем проекте. Полный объём не
считается завершённым по наличию отдельных конструкторов или passing unit tests.
Функциональные границы ниже сохраняются до сквозной геометрической квалификации.

Подробные контракты режимов, независимые гарантии E/R/I/J и восемь итоговых ворот: [sweep-contract-matrix.md](sweep-contract-matrix.md).

## Требования и текущие границы

| Область | Что должно быть готово | Текущее доказательство / недостающая работа |
| --- | --- | --- |
| Профили | Рациональные открытые/замкнутые профили; плотные кривые без refit | `profile_sweep` сохраняет веса; `progressive_sweep` разбивает плотные непериодические профили на knot-span patches. Плотные periodic-профили требуют clamped encoding |
| Несколько контуров | Наружный контур, отверстия, согласованное сечение | `MultiSweep`/`approximate_profiles` и Rush `progressive_sweep(profile1,...,path,...)` сохраняют до64 authored curves на общей сетке с общим бюджетом. Проверены rational outer/inner contours и shared boundary controls. Многоконтурные planar caps и shared B-rep topology добавлены native/TS конструктором `progressive_profile_body`; Rush body integration и перенос в Solid проверены |
| Пути | Пространственные рациональные кривые, составные и замкнутые пути | Гладкие пути; fixed orientation принимает C0 corners. Открытые spatial polyline round joins доступны через rational arcs; quintic transition joins добавлены, сквозная квалификация ниже. Замкнутые round/transition joins добавлены с циклической обработкой всех углов; open spatial miter sections и capped hollow body проходят сквозные проверки. Cyclic extrusion miter с согласованными кадрами проверен; native/TS progressive miter исправляет ненулевую holonomy и поддерживает scalar scale/twist. Rush/worker/viewport integration проверена. Continuous real-arithmetic interpolation estimate управляет принятием miter; округление и расширенные affine/frame/guide законы требуют работы |
| RMF | Управляемая начальная ориентация, минимальная крутка | Double reflection и sampled refinement; непрерывная ошибка идеального RMF не доказана |
| Frenet | Tangent/normal/binormal; политика inflections/zero curvature | Строгий Frenet отказывает при неопределённой principal normal. Native corrected_frenet продолжает sampled кадр через перегибы и zero curvature; непрерывная regularity ещё не доказана |
| Фиксированные frames | Fixed world orientation и фиксированное normal-направление | Оба режима есть в progressive ядре; fixed_normal отказывает при параллельности normal/tangent |
| Authored frames | Полные пользовательские frames/ориентационные законы | Native `new_authored`/`with_frame_laws` и JSON orientation=authored: рациональные longitudinal/transverse laws, Gram–Schmidt frame, twist/affine laws и общая сетка. Native qualification прошла; TS/Rush surface/body API подключён; authored hollow body просмотрен в Solid. Непрерывная regularity кадров ещё не доказана |
| Twist | Signed angular law одновременно с RMF/scale, несколько оборотов | Рациональный scalar law в радианах native / градусах Rush; closed endpoints должны отличаться на целое число оборотов |
| Scale | Положительный рациональный scalar law совместно с ориентацией/twist | Есть uniform scale, positive rational axis_scale по трём local axes и center_law offset в local frame; оба векторных закона совместны с twist/RMF и общим refinement budget |
| Параметризация | Законы и станции по параметру либо длине дуги | Есть parameter/arc_length; обратная длина использует interval residual и ограниченный work budget; при исчерпании возвращается отказ |
| Направляющие | Одна/две направляющие, геометрическое соответствие профилю | Текущий two-guide конструктор — authored affine rails. Progressive orientation guide добавляет perpendicular normal и parameter/arc-length correspondence двух curves. Shared contact anchor/width fitting в normal plane прошёл Rust/WASM/TS/Rush/body tests; arbitrary correspondence correction, visual qualification и continuous certification ещё требуют работы |
| Прогрессия | Preview, продолжение, остановка, автоматическое уточнение | Rust `Sweep` iterator выдаёт один level за вызов. Final patches только при принятом sampled budget. Host level API и async surface/body streams подключены к Rush/worker и source viewport; отмена проверена в real worker. Geometry-changing root compositions и визуальная проверка cancellation UX остаются |
| Патчи | Плотные профили и >32 сечений, сохранение доменов и общих границ | Progressive output разбивает U по knots, V на <=32 controls; seam C0. Surface API не выполняет sewing; body constructor сохраняет общие incidence edges |
| Closure | Holonomy, scale/twist/frame seam, геометрия и топология | Closed RMF корректирует holonomy по chord lengths; проверены matching endpoint scale, whole-turn twist и C0 seam. Periodic body использует общие seam edges; G1/G2 не доказаны |
| Точность | Непрерывная ошибка на всей поверхности плюс rounding | Fourfold sampled control comparison. `continuousBound:false`, `roundingCertified:false`; требуется отдельная непрерывная оценка |
| Регулярность | Cusp, singularity, folds, self-contact/intersection | Frame/input refusals есть; полный аудит поверхности и нескольких swept walls ещё требуется |
| Тела | Крышки, holes, manifold solid, shell и толщина | Progressive sweep выдаёт поверхности/patches. В B-rep уже есть bounded polygonal solid sweep (`audited_bent_rmf_sweep`, `audited_parallel_frame_sweep`); `rational_section_loft` и `progressive_profile_body` сохраняют rational walls, caps/holes и authored shared edges для open paths, до1025 sections и64 Bezier spans на section. Замкнутые пути используют periodic_section_loft без крышек, с общими seam edges и inner shells для holes. Глобальное embedding не сертифицировано |
| Экспорт | JSON, mesh formats и STEP round-trip с параметрами/топологией | JSON/OBJ/PLY round-trip новых примеров прошёл. Внутренние STEP v5 round-trips rational bodies/cap holes прошли; независимая STEP conformance ещё требуется |
| Интеграция | Rust → transport → WASM → TS → Rush → editor | Surface/body API проходят Rust/transport/WASM/TS/Rush, включая preview/cancel. Strip, authored/periodic/contact hollow bodies просмотрены в source/Solid viewport; полный visual feature matrix ещё требует проверки |
| Совместимость | Предыдущие sweep/pipe/ribbon сохраняют поведение | Актуальный sweep/pipe/ribbon regression и полный native прогон трёх библиотек прошли |

## Новый progressive контракт

`progressive_sweep::Sweep::new(profile,path,scale,twist,options)` — resumable
Rust iterator. `progressive_sweep::approximate` выполняет все необходимые levels.
Сначала строится initial_sections, затем число интервалов удваивается вплоть до
max_sections. Каждый preview содержит patch set и report. Если budget не достигнут,
итоговый `patches` равен null; budget никогда не расширяется автоматически.

Scale и twist — рациональные scalar curves с controls `[value,0,0]`, положительный
scale, signed twist в радианах. Домены profile/path/laws независимы; laws используют
нормализованную долю выбранного traversal. Начальный profile задаётся в world
coordinates относительно path start. Fixed orientation сохраняет initial tangent;
fixed_normal проектирует normal на normal plane; Frenet использует principal normal.

Максимум 1025 coarse sections, 4097 fine stations и 4096 patches. Arc-length режим
ограничен 257 coarse sections из-за 1024-segment inverse-length API. У каждого
patch максимум 32 controls на ось. Эти лимиты явные и не обходятся ослаблением
точности. Whole-turn closed twist и equal endpoint scale обязательны; Frenet и
fixed_normal также требуют совпадения endpoint orientation.

Rust tests проверяют независимое rational-profile/scale/twist equation, ориентации,
resume/failure semantics, >32 sections, dense profiles, closed seam, nonlinear
rational path с arc-length spacing, corner paths в fixed режиме и tiny open path.
Свидетельства: `docs/qualification/sweep-coverage-2026-10-01/native-progressive.log`.

## Порядок оставшейся работы

1. Завершить визуальную матрицу режимов и live cancellation/restoration UX; базовые surface/body cases уже просмотрены.
2. Поддержать preview geometry-changing root compositions и полную Solid UI parameter editing матрицу.
3. Добавить arbitrary guide correspondence correction и corner joins (miter/round/transition).
4. Доказать continuous frame regularity и G1/G2 closed seams; C0 periodic body topology уже реализована.
5. Добавить непрерывную ошибку с rounding и аудит surface regularity/self-intersections/embedding с honest unresolved states.
6. Проверить внешнюю STEP conformance и весь feature matrix на независимых эталонах; bounded внутренние round-trips уже прошли.

Цель полного покрытия остаётся активной: строки с недостающей работой не считаются
закрытыми, а новый progressive конструктор не объявляется полноценным solid kernel.

## Сквозная проверка этапа

423 packaged WASM tests в13files прошли; typecheck и полный native regression
для nurbs-core/modelgraph-runtime/modelgraph-text прошли. Production UI build
прошёл в отдельный временный каталог. Progressive scale/twist strip построен
через реальный source editor, перенесён в Mesh и визуально просмотрен: один
открытый объект,768triangles. Screenshot и hashes:
`docs/qualification/sweep-coverage-2026-10-01/verification.json`.
Это подтверждает показ одного примера, не visual qualification всех режимов.

## Многопрофильный этап

Native `MultiSweep` выдаёт resumable previews, `approximate_profiles` принимает
результат только при принятом бюджете всех профилей. Промежуточные и итоговые
кривые используют одинаковое число сечений; малая стенка не останавливает
уточнение большой. JSON `surface_progressive_sweep_profiles` и TS
`progressiveSweepNurbsProfiles` сохраняют authored curve order через half-open
`profilePatchRanges`. Rush допускает 1..64 профиля, последний positional argument
всегда path. Старый двухаргументный синтаксис совместим.

Это конструктор стенок. Он не выводит material/holes из nesting и не объявляет
patch set сшитой оболочкой. Суммарный лимит4096 patches действует на весь набор.

Многопрофильный этап проверен:11 native progressive tests, Rush lowering,
TypeScript check и428 packaged WASM tests в13files прошли. В этот прогон включены
новый Rush пример, editor entrypoint и JSON/OBJ/PLY regression. Новый viewport
в этом этапе не просматривался. Логи и hashes:
`docs/qualification/sweep-coverage-2026-10-01/multi-profile-verification.json`.

## Рациональные крышки и B-rep

`brep_core::rational_section_loft` принимает 2..1025 согласованных сечений,
каждое из outer loop и до15 holes. Nonperiodic рациональные кривые раскладываются
без refit на Bezier spans, соответствующие degrees/weights должны совпадать.
Допускается до64 spans на section. Endpoint joins должны совпадать точно.
Две planar caps используют rational UV trims; interval trim-region audit
обязателен, unresolved приводит к отказу. Shared vertices/edges задаются при
построении, итог проходит topology и sampled geometry agreement validation.

`progressive_profile_body` сначала проверяет aggregate sampled budget всех
контуров и только затем строит эти wall/cap faces из retained station curves.
Закрытый путь использует periodic_section_loft без крышек: repeated endpoint section
переиспользует seam vertices/edges; каждый hole становится inner shell.
JSON bridge и TS APIs: `createRationalBrepSectionLoft`,
`createProgressiveBrepProfileBody`. Native twist использует radians, TS degrees.
`globalEmbeddingCertified:false` — manifold incidence не доказывает regularity,
absence of folds или self-intersections всего тела. Rush и перенос native B-rep в Solid проверены; глобальная сертификация остаётся
отдельной незавершённой задачей.

Topology limit основан на общем `MAX_FACES`, а не прежнем polygonal лимите16:
прогрессивный конструктор ограничивает дальнейшее уточнение доступным числом
wall faces, сохраняя исходный deviation budget. Native case33sections /258faces
прошёл. Bent rational path с непараллельными крышками прошёл; STEP AP242 /5
round-trip сохранил topology, rational surfaces и cap holes с учётом
canonical face parameter reversal. Это qualification конкретных случаев,
не внешний независимый STEP conformance test и не сертификат embedding.

Итог этапа:7 новых native body tests,95 аналитических B-rep regression tests,
21 trim tests,11 progressive tests и447 packaged WASM tests в16files прошли.
Проверены cap holes, shared incidence, retained rational weights, bent path,
reverse travel positive mesh volume,33stations и AP242 /5 round-trip.
TypeScript check и targeted diff checks прошли. Новый viewport в этом этапе не
просматривался. Evidence:
`docs/qualification/sweep-coverage-2026-10-01/rational-body-verification.json`.

## Rush и Solid bridge

`brep_progressive_sweep([[outer],[hole,...]],path,scale:...,twist:...,normal:...,
max_deviation:...)` подключён к Rush/schema/runtime и graph builder. Линейные
размеры и angular twist проверяются по единицам; body report сохраняет levels,
profilePatchRanges и `globalEmbeddingCertified:false`. Пример:
`examples/rush/progressive-hollow-body.r`.

Проверено через actual editor entrypoint `parseOpenSCAD`, retained nativeGeometry
kind=brep и `sceneMeshesToSolidDocument`: оба cap holes и authoritative B-rep
сохраняются при переносе в Solid. Новый пример прошёл JSON/OBJ/PLY round-trip.
406 tests в4files и native Rush lowering прошли. Это проверка editor/bridge path,
не визуальный просмотр нового тела в Solid viewport; визуальная квалификация
ещё требуется. Лог: `docs/qualification/sweep-coverage-2026-10-01/rush-body-tests.log`.

## Анизотропный scale и центр

Rust `Sweep::with_affine_laws` / `MultiSweep::with_affine_laws` задают rational
3-vector axis scale и center offset. При изменении законов после preview прогрессия
перезапускается с initial_sections; старое accepted состояние не переносится.
Формула local coordinates: `q_new = uniform_scale * axis_scale * q + center`,
затем twist и frame transport по исходному guide path. Offset использует mm,
axis scale dimensionless, twist degrees в TS/Rush и radians native.

TS `ProgressiveSweepOptions.axisScale/centerLaw`, Rush records `axis_scale` и
`center_law` имеют degree/knots/values (3-vectors)/weights. Domains laws независимы;
parameter/arc-length traversal задаёт нормализованную долю. Если только один закон
задан, второй — identity axis scale или zero offset. Axis-scale controls должны
быть строго положительны; positive rational weights сохраняют positivity между
станциями. На closed seam оба векторных закона обязаны совпадать. Для clamped
endpoint vector laws используются authored controls без лишнего weight round-trip.

Поверхность и capped body используют один transport; B-rep caps/holes не
перестраиваются по mesh. Примеры: `examples/rush/affine-progressive-sweep.r` и
`examples/rush/affine-hollow-body.r`. Последний сохраняет deviation0.001mm,
display4segments укладывает многогранный refined body в20k triangle budget.
Native equation test использует rational profile/law weights и независимые
normalized domains; tests отдельно проверяют closed seam, positive-axis отказ,
restart после accepted preview и endpoint geometry полого эллиптического тела.
Continuous sweep error/rounding/global self-intersections по-прежнему не доказаны.

Итог векторных законов:435 packaged WASM tests в6files,14 native sweep tests,
10 B-rep tests и native Rush lowering прошли; typecheck/catalog/diff checks
прошли. JSON/OBJ/PLY/editor regression обоих новых примеров прошёл. Новый
viewport в этом этапе не просматривался. Evidence/hashes:
`docs/qualification/sweep-coverage-2026-10-01/affine-verification.json`.

## Periodic B-rep: проверка 2026-10-02

Замкнутый progressive sweep автоматически строит periodic walls без caps,
переиспользуя seam topology. Внешний контур и отверстия сохраняются как outer
и inner shells. Доступен прямой TS API `createPeriodicBrepSectionLoft`.
Строгое совпадение repeated sections и endpoint laws обязательно; C0 seam
не означает G1/G2. Closed-path admission учитывает нулевой cap-face резерв:
проверен body с ровно1024 wall faces.

Текущая проверка: 446 WASM tests в7 файлах;13 native B-rep integration tests,
14 progressive sweep regressions и native Rush example test. Typecheck,
geometry/language WASM и production UI build прошли. Свежий preview показал
`closed-progressive-hollow-body.r` в Solid как B-rep (один объект,
16384 source triangles); browser error/warn logs пусты. Снимок:
`docs/qualification/sweep-coverage-2026-10-01/periodic-hollow-solid-preview.jpg`.
Внутренняя полость доказана topology/STEP tests, внешний вид проверен визуально.
Global embedding, непрерывная погрешность и rounding certification не доказаны.

## Authored frames: native этап 2026-10-02

`Sweep::new_authored` принимает независимые рациональные 3-vector laws для
longitudinal axis и transverse direction. Продольная ось нормализуется,
поперечное направление проектируется перпендикулярно ей и нормализуется;
третья ось — right-handed cross product. Кадр может не совпадать с tangent
направляющей. Twist вращает вокруг authored longitudinal axis; axis_scale
и center работают в этом local frame. Initial profile coordinates вычисляются
в initial authored frame. Laws следуют выбранному traversal независимо от
собственных доменов. `with_frame_laws` перезапускает refinement; для него
требуется Fixed orientation. `MultiSweep` использует те же laws для всех curves.

JSON `orientation:"authored"` требует `frame_axis` и `frame_normal` в формате
Curve; direct native helper — `approximate_authored_profiles`. Нулевые или
параллельные направления на проверяемых станциях отказывают. Closed seam
требует совпадения normalized endpoint frames. Проверка остаётся sampled:
непрерывная невырожденность между станциями не сертифицирована.

18 native progressive tests прошли, включая independent rational-frame +
scale/twist/center equation, повторный запуск, shared multi-profile grid,
closed seam, sampled degeneracy refusals и JSON required-field/metadata.
Лог: `docs/qualification/sweep-coverage-2026-10-01/authored-frame-native.log`.
Packaged WASM/TS/Rush/B-rep и visual qualification этого нового режима ещё
не выполнены; прежняя periodic UI проверка их не заменяет.

## Authored frames: TypeScript API

`progressiveSweepNurbsPatches` и `progressiveSweepNurbsProfiles` принимают
`AuthoredProgressiveSweepOptions`: orientation=authored, обязательные
`frameAxis` и `frameNormal` в формате NurbsVectorLaw. Они передаются как
Curve payloads в тот же geometry transport. B-rep options пока не расширены:
добавление authored режима там требует отдельного retained-section подключения.
TS тест проверяет независимый кадр T=X при guide tangent=Z, twist в градусах
и sampled parallel-direction отказ через public WASM API.

Свежая geometry WASM build и TypeScript check прошли. 419 packaged tests
в4 файлах прошли, включая новый authored-frame public API test, прежние
progressive surface/body cases и artifact checks. Логи:
`authored-frame-geometry-build.log`, `authored-frame-typecheck.log`,
`authored-frame-wasm-tests.log` в qualification directory. Для authored кадров
это подтверждает Rust→WASM→TS; Rush, B-rep и viewport остаются непроверенными.

## Authored frames: Rush интеграция

Rush `progressive_sweep(...,orientation:"authored",frame_axis:{...},
frame_normal:{...})` сохраняет rational vector laws через lower/schema/runtime.
Frame values безразмерные; mm controls отказывают. Перед construction требуется
наличие обоих laws. Graph kernel передаёт их существующему TS API.
Пример `examples/rush/authored-progressive-sweep.r` использует longitudinal
axis X при пути Z, projection authored normal и twist 90 degrees.
Generic qualification включает этот пример в editor/mesh/export regression.
B-rep authored transport и непосредственный visual inspection этого режима
ещё не выполнены.

Language WASM build, typecheck и 428 packaged tests в4 файлах прошли.
Логи `authored-rush-language-build.log`, `authored-rush-typecheck.log`,
`authored-rush-wasm-tests.log` сохранены в qualification directory.

## Authored B-rep frames

`progressive_authored_profile_body` передаёт один набор longitudinal/transverse
laws и affine laws одновременно в aggregate admission и retained section
construction. Используются прежние rational walls, audited planar caps
и periodic shells. JSON body transport, TS `createProgressiveBrepProfileBody`
и Rush `brep_progressive_sweep` принимают тот же authored contract;
отсутствие одного кадра отказывает до construction.

Пример `authored-progressive-hollow-body.r` наклоняет longitudinal axis от
Z к Y+Z, масштабирует полый профиль и одновременно применяет twist.
Native body regression:14 tests прошли, включая новую manifold/cap проверку.
Визуальная проверка нового режима и global embedding остаются незавершёнными.

Geometry/language WASM builds, typecheck и443 packaged tests в5 файлах
прошли: public authored hollow-body API, STEP roundtrip, rational caps/holes,
Rush/editor/export example и предыдущие sweep regressions. Логи
`authored-body-*.log` сохранены в qualification directory. Это подтверждает
сквозной construction/transport; новый authored body ещё не просмотрен
в viewport, и global embedding не сертифицирован.

## Authored body: визуальная квалификация

Production preview собран из текущего worktree 2026-10-02 и открыт
в in-app browser на localhost5198. `authored-progressive-hollow-body.r`
построен через source editor: один объект,4168 triangles. Перенос
«Перенести сцену в Solid» сохранил тело как B-rep; ISO viewport просмотрен:
видны наклонённая крышка, отверстие и расширение сечения. Browser error/warn
logs пусты. Screenshot: `docs/qualification/sweep-coverage-2026-10-01/
authored-hollow-solid-preview.jpg`; build log: `authored-body-ui-build.log`.
Это visual evidence конкретного примера; continuous error, global embedding
и весь visual matrix этим не сертифицированы.

## Progressive orientation guide: native контракт

Вторая пространственная кривая может задавать normal: направление от main
path station к соответствующей rail station проецируется перпендикулярно
main tangent и нормализуется. Correspondence по независимым domains использует
parameter fractions или certified inverse arc-length fractions обеих curves.
Aggregate length residual учитывает обе кривые. Это заменяет orientation
на станциях, совместно с twist, scale, affine laws и shared profile refinement.
Closed seam требует совпадения endpoint normals, RMF holonomy correction
к этому режиму не применяется. Fixed/authored modes несовместимы.

`with_orientation_guide` перезапускает Sweep/MultiSweep. Coincident или
tangent rail direction отказывает на sampled stations. Позиционное касание
профиля rail и width fitting ещё не реализованы: этот orientation этап
не закрывает полное two-guide correspondence requirement. Непрерывная
невырожденность, native body и TS/Rush/UI integration также остаются задачами.

21 native progressive tests прошли: независимое projected rational-rail
equation, restart, inverse arc-length correspondence, sampled degeneracy
refusals, closed seam и shared contours. Лог `orientation-guide-native.log`.
Существующие authored/affine/RMF tests также прошли; packaged WASM этого
нового native режима ещё не обновлён.

## Orientation guide: JSON/TS

Surface JSON transport принимает optional `orientation_guide` Curve.
`approximate_guided_profiles` сохраняет aggregate refinement и affine laws.
TS `progressiveSweepNurbsPatches`/`progressiveSweepNurbsProfiles` принимают
`GuidedProgressiveSweepOptions` с `orientationGuide`. Fixed и authored
кадры при совместной guide отказывают; guide задаёт projected normal.
Публичный тест проверяет independent rational correspondence equation,
arc-length residual обеих rails и coincident-rail refusal.
Contact/width fitting, Rush/body integration и visual qualification
по-прежнему не выполнены.

Обновлённый geometry WASM и430 packaged tests в4 файлах прошли,
включая guide public API equation, dual-rail arc-length residual и work-limit
refusal. Native progressive regression:22 tests прошли. Typecheck прошёл.
Логи `orientation-guide-{native,geometry-build,typecheck,wasm-tests}.log`
сохранены в qualification directory. Это Rust→WASM→TS evidence;
Rush/B-rep/visual qualification ещё не выполнены.

## Orientation guide: Rush reference

Rush surface `progressive_sweep` принимает `orientation_guide: rail`.
Lowering сохраняет geometry reference; runtime включает rail в dependency
audit и selection traversal; schema и TS graph kernel разрешают curve.
Пример `guided-progressive-sweep.r` построен через packaged language/geometry
API. Unknown rail reference отказывает. Language build, typecheck и24 tests
в2 файлах прошли; логи `guide-rush-*.log`.
B-rep rail integration, позиционное касание/width fitting, visual qualification
и continuous regularity остаются незавершёнными.

## Contact rail: native retained-station fitting

`Sweep::with_contact_guide(rail,profile_parameter)` выбирает точку рационального
профиля. Initial anchor должен совпадать с initial rail point; соответствующие
rail stations должны лежать в main-path tangent normal plane. Transverse
координата масштабируется по расстоянию до rail с учётом uniform/axis scale
и normal center offset. Side/longitudinal scale сохраняются; ненулевые
side/longitudinal center offsets отказывают. Contact фиксирует orientation,
поэтому nonzero twist несовместим и отказывает. Closed rail требует одинаковой
ширины на endpoint seam; только совпадения normal недостаточно.

Это positional fitting retained stations, а не continuous rail-contact
certificate между ними. Общая сетка нескольких профилей с одним contact
anchor, произвольная коррекция correspondence/normal-plane intersections,
transport/Rush/body integration и visual qualification ещё требуются.

23 native progressive tests прошли, включая independent rational anchor
contact equation, interior profile point, guide/main path endpoints, normal
plane refusal и incompatible twist refusal. Лог `contact-guide-native.log`.
Packaged WASM этого contact режима ещё не обновлён.

## Общий contact anchor нескольких профилей

`MultiSweep::with_contact_guide(rail,profile_index,parameter)` выбирает
один reference profile anchor, вычисляет его world point и задаёт общий
transverse fit всем профилям. Каждый contour сохраняет относительные
normal coordinates; соответствующие границы не подгоняются к rail независимо.
`approximate_contact_profiles` совмещает этот контракт с affine laws
и aggregate admission. Setter перезапускает refinement. Out-of-range index/
parameter и initial anchor, не совпадающий с rail, отказывают.

24 native progressive tests прошли. Новый test использует два профиля
с различными rational weights и widths, сверяет общий contact equation,
перезапуск и invalid anchor/index/domain refusals. Лог `contact-multi-native.log`.
Это native multi-profile evidence; closed outer/hole B-rep qualification,
JSON/TS/Rush/UI contact integration и continuous contact certificate
по-прежнему требуются.

## Native contact hollow-body qualification

`MultiSweep::sections_at` возвращает retained preview curves в authored
profile order на общей сетке. Сначала требуется accepted aggregate report;
проверка body использует только принятое число станций. Rational outer
circle и reverse hole circle сохраняют weights и relative transverse widths
при contact fitting; retained stations передаются `rational_section_loft`.
Результат имеет две audited caps с hole trims и нулевое число boundary edges.
STEP V5 roundtrip сохраняет topology, cap holes и face count.

25 native progressive tests и15 native B-rep integration tests прошли.
Дополнительный closed contact test отказывает при совпадающих endpoint
normals, но разной rail width. Логи `contact-hollow-native.log` и
`contact-hollow-sweep-native.log`. Packaged contact constructor,
JSON/TS/Rush/UI и continuous contact/embedding certification ещё требуются.

## Surface contact anchor: JSON/TS API

TS GuidedProgressiveSweepOptions принимает `contactAnchor:{profileIndex,
parameter}`; profileIndex по умолчанию0, parameter задан в domain выбранной
кривой. JSON использует `orientation_guide`, `contact_profile`,
`contact_parameter` и `approximate_contact_profiles`. Anchor без guide
и profile index без parameter отказывают, незаданные поля сохраняют прежний
orientation-only mode. Общий fit и aggregate admission остаются общими
для всех contours.

Public API regression проверяет contact и relative widths двух profiles
с разными rational weights, nonzero-twist и invalid-profile-index refusals.
Rush/contact B-rep constructor и visual qualification пока не подключены.

Geometry WASM build, typecheck и433 packaged tests в4 файлах прошли.
Проверены общий contact fit, incomplete JSON contact fields, zero-twist
и invalid-reference-index refusals; предыдущие surface/body/export/artifact
regressions прошли. Логи `contact-api-{build,typecheck,tests}.log`.
Это public surface API evidence, не Rush/contact-body/UI qualification.

## Contact anchors in Rush

Rush `progressive_sweep` принимает `orientation_guide: rail`,
`contact_profile: index`, `contact_parameter: parameter`. Anchor parameter
безразмерен и задан в domain выбранного profile. Graph kernel отказывает
при anchor без guide либо index без parameter. Пример
`contact-progressive-sweep.r` использует два weighted profiles с одним
anchor; generic regression включает editor/mesh/export pipeline.
Contact B-rep API и visual/continuous qualification ещё требуются.

Language WASM build, typecheck и441 tests в4 файлах прошли. Contact Rush
example использует два неперекрывающихся rational ribbons: исходные
коллинеарные overlapping ribbons вызывали mesh topology refusal при
OBJ/PLY export, поэтому пример исправлен, проверки сохранены.
Текущий пример прошёл JSON/editor/OBJ/PLY regression. Логи
`contact-rush-{build,typecheck,tests}.log`. Contact body/UI и непрерывное
contact/embedding доказательство остаются незавершёнными.

## Public guided/contact B-rep constructor

`progressive_guided_profile_body` сохраняет body face budgets и принимает
spatial guide с optional `(flattened_profile_index,parameter)` contact anchor.
Aggregate admission и retained sections используют одинаковый MultiSweep
с shared guide/contact/affine settings; accepted station count подаётся
в прежний rational open/periodic constructor.

JSON `brep_nurbs_progressive_profile_body`, TS
`createProgressiveBrepProfileBody` и Rush `brep_progressive_sweep` расширены
optional orientation guide/contact полями. Guide участвует в graph refs
и selection. Incomplete contact или authored+guide combinations отказывают.
Native15-test body regression прошёл; новый public constructor совпадает
по каждой surface control point/weight с независимым retained-section loft.
Новый пример `contact-progressive-hollow-body.r` использует рациональные
outer/hole circles и общий transverse width fit. Visual qualification
и continuous contact/embedding certification остаются незавершёнными.

Geometry/language WASM builds, typecheck и457 packaged tests в5 файлах
прошли. Прогон запущен повторно после terminal geometry packaging: ранний
прогон во время упаковки обнаружил identity mismatch старого manifest
и новых bytes. Итоговая artifact identity проверка прошла.
Проверены public contact body extrema ±6mm (initial radius3mm), rational
caps/holes, closed mesh, STEP roundtrip, incompatible twist refusal и
Rush/editor/JSON/OBJ/PLY integration нового примера. Native regression15 tests
прошёл. Логи `contact-body-{native,geometry-build,language-build,typecheck,
wasm-tests}.log`. Visual inspection contact body ещё не выполнен.

## Contact hollow body: Solid visual evidence

Текущий production preview на localhost5199 построен и просмотрен через
source editor: `contact-progressive-hollow-body.r`, один объект,584 source
triangles. Перенос в Solid сохраняет B-rep; в ISO viewport видны отверстие
и поперечное расширение к ellipse. Browser error/warn logs пусты.
Screenshot `docs/qualification/sweep-coverage-2026-10-01/
contact-hollow-solid-preview.jpg`; build log `contact-body-ui-build.log`.
Конкретный contact body visual case подтверждён. Полная visual matrix,
continuous contact/error/regularity и global embedding ещё не доказаны.

## Independent progressive preview level

`Sweep::preview_at(count)` и `MultiSweep::preview_at(count)` вычисляют
один bounded refinement level без изменения iterator progression. Count
должен быть между configured initial/max sections; fine stations и shared
patch/arc-length limits сохраняются. MultiSweep iterator использует ту же
aggregate implementation, включая profile patch ranges и max deviation
всех contours. Level JSON serialization явно ставит `preview:true`;
report.accepted и continuousBound:false сохраняются.

Это native основа отдельных host requests, позволяющих отмену между
levels. JSON dispatch, worker preview messages, cancellation и live viewport
update ещё требуют подключения; наличие native preview не закрывает
строку streaming UI. Unaccepted previews не являются construction results.

26 native progressive tests прошли: direct level и iterator level имеют
одинаковые reports/control points; внебюджетные запросы отказывают,
progression не сдвигается; false admission и preview metadata сохранены.
Лог `sweep-level-native.log`. Packaged WASM ещё не обновлён.

## Host single-level preview contract

JSON `surface_progressive_sweep_level` принимает ordered `profiles` и
`preview_sections`, остальные orientation/affine/guide/contact options
совпадают с full construction. Запрос возвращает patches, profilePatchRanges,
report и `preview:true`, включая unaccepted levels. TS entrypoint:
`previewProgressiveNurbsProfiles`. Нельзя считать patches construction
результатом только потому, что они присутствуют: admission остаётся
report.accepted и бюджет не меняется.

Это stateless bounded host operation. Worker streaming, cancellation между
levels и editor viewport callbacks ещё не подключены.

Geometry WASM build, typecheck и459 packaged tests в5 файлах прошли.
Direct preview reports совпадают с соответствующими full-history levels;
accepted preview patches совпадают с full result. Проверены nonpromotion
unaccepted previews, count bounds, authored/affine/guide/contact option
preservation и прежние body/STEP/export regressions. Логи
`sweep-level-{build,typecheck,wasm-tests}.log`. Worker/editor streaming
и cooperative cancellation ещё не реализованы.


## Host progressive stream and cooperative cancellation

`streamProgressiveNurbsProfiles` is an async generator over the existing
single-level host operation. It follows the native doubling schedule, clamps
its last level to maxSections, and stops after acceptance. Each yield retains
`preview:true`; the generator return value exposes construction patches only
when the final report is accepted. Budget exhaustion returns null patches and
ranges, together with the complete report history.

An optional AbortSignal is checked before scheduling, before each kernel call,
and when resuming after a yielded level. A timer yields to worker message
dispatch between synchronous calls. One running kernel call cannot be
interrupted by this mechanism. Worker protocol messages and editor viewport
integration remain pending; this host API alone does not close streaming UI.

18 progressive host tests and vue-tsc passed, including exact equivalence with
full construction, a non-doubling terminal budget, and cancellation before,
during scheduling, and between levels. Evidence: sweep-stream-tests.log and
sweep-stream-typecheck.log. No native kernel changes or rebuild were needed.


## Async Rush graph evaluation and parser cancellation

`buildOwnNurbsAsync` resolves reachable progressive_sweep nodes through the
host stream, awaits preview consumers, and retains completed sweep results
for subsequent graph evaluation. Other operations retain their synchronous
implementations. Intermediate suspension is an internal control path, never
an authored node failure. Graph evaluation resumes with accepted sweep data;
refused levels are delivered as previews but still cause construction failure.
Dependent graph evaluation is retried after each completed sweep, so unrelated
synchronous operations may be repeated; this is not a general asynchronous
NURBS evaluator or in-call kernel cancellation.

The Rush own-NURBS parser route now uses this evaluator, forwarding
shouldAbort/onYield and onSweepPreview. GeometryBuildControl exposes the same
preview callback. Worker cancellation probes and heartbeat hooks therefore
operate between progressive levels on this route. Parser aborts retain the
existing AbortedError contract, and its serialized queue remains usable.

95 tests across progressive sweep, parser, and build-engine suites passed,
along with vue-tsc and scoped diff checks. Tests compare asynchronous output
with synchronous output for seven orientation/profile/closure examples,
check callback failure propagation, refused-preview nonpromotion, and parser
cancellation/recovery. Evidence: sweep-async-graph-{tests,typecheck}.log.

Worker preview wire messages and editor rendering remain pending.
brep_progressive_sweep still uses its synchronous body constructor; its
progressive body-level preview route is not implemented by this change.


## Worker preview transport and coordinator admission

Geometry worker protocol v8 adds `sweep-preview`, carrying one node's bounded
mesh, section count, acceptance flag, sampled deviation, and budget. Geometry
worker tessellates each received level at four display segments, using the
same display mesh adapter as final scene publication; buffers are transferred.
Preview meshes carry no nativeGeometry, no source spans, and no authoritative
face IDs. Their entity identity is bounded by a SHA-256 node ID digest.
Wire validation applies the existing aggregate byte/triangle limits, mesh/BVH
checks, bounded node/count fields, and admission/deviation consistency.

BuildCoordinator dispatches previews through onSweepPreview, after its usual
worker-generation, revision, quality, source-hash and current-job admission.
Preview delivery neither invokes onPublish nor advances publishedQuality;
old and superseded jobs cannot deliver previews to this callback.

125 tests in six suites passed, including a real Node worker running the
actual geometry.worker.ts entrypoint. That test verifies multiple transferred
levels, nonauthoritative metadata, cancellation on preview, no cancelled-job
success, and recovery on the same worker. Protocol, coordinator, parser/graph,
and existing worker regressions passed. vue-tsc and scoped diff checks passed.
Evidence: sweep-wire-{tests,typecheck}.log.

The editor has not subscribed to onSweepPreview yet. These meshes are local
to a sweep node, not a reconstructed root scene: downstream transformations,
booleans, and multiple sweep compositions still need an explicit display
policy before viewport publication. Body-level streaming remains pending.


## Scene placement and source viewport subscription

The own-NURBS scene route maps progressive node previews through downstream
affine control-net transforms in evaluation order before its display callback.
The mapper admits a direct sweep root or a transform/display chain; geometry
changing operations such as thicken and booleans are not approximated by a
local node preview. Tests verify noncommuting nested transforms and explicit
suppression for an unsupported root composition. The graph-level callback
remains node-local; scene/parser/worker callbacks now receive scene placement.

App subscribes to coordinator previews using its existing renderer-only
preview path. It checks the current editor revision and avoids overwriting an
active modeling preview. Terminal state, current result publication, and source
changes restore published geometry. Export/CAD scene data is not replaced.
The source drawer previously hid its render canvas; Rush progressive_sweep
sources now show that canvas alongside the editor. Browser inspection found
and fixed clipping caused by drawer overflow before final visual validation.

142 tests in five relevant suites passed; the Vite preview build passed.
The latest vue-tsc run is blocked by a concurrent unrelated missing
CadKernelOps.minkowskiSum2 implementation in solid/brepRecorder.ts:65.
No type errors were reported in the sweep changes. This is not a successful
whole-worktree typecheck. Evidence: sweep-scene-{tests,typecheck,ui-build}.log.
Browser port 5200 rendered the example with one object and 768 triangles;
console warnings/errors were empty. Final scene screenshot:
sweep-scene-editor-preview.jpg. This screenshot verifies final display and
layout, not transient level timing or cancellation restoration visually.

Remaining streaming coverage includes body levels, geometry-changing root
compositions, and live visual proof of intermediate level/cancellation UX.


## B-rep host wall streaming before topology admission

`streamProgressiveBrepProfileBody` now yields ordered multi-profile wall
previews before invoking the existing authoritative body constructor.
Exact Rust curve decomposition determines the common Bezier span count;
levels use the same 1024-face budget as brep-core, reserving two cap faces
for an open path and no caps for a closed path. An initial bounded preview
queries native closedPath classification; this probe is currently recomputed
by the stream. The final body constructor also recomputes retained levels;
this API is cooperative orchestration, not a persistent native body job.

Final output remains conditional on both sampled wall admission and the
existing cap/seam/topology constructor. An accepted wall preview cannot
promote an invalid hole orientation into a body. AbortSignal and worker
cancellation probes run between levels and before final topology construction.
A running synchronous kernel call remains uninterruptible.

39 tests in the sweep and rational-body suites passed. New cases prove exact
body/report equality with the existing constructor, cancellation after the
accepted wall level, null body publication on budget exhaustion, seven levels
ending at 128 sections for eight spans with cap reserve, and cap orientation
refusal after wall acceptance. vue-tsc and scoped diff checks passed.
Evidence: sweep-body-stream-{tests,typecheck}.log. The earlier concurrent
minkowskiSum2 typecheck blocker has cleared in the current worktree.

The main renderer measurement callback now applies the same preview/recovery
state guard already used by selection and hover, preventing temporary mesh
measurements from updating the authoritative scene's measurement state.
Rush graph/worker integration of the new body stream remains pending; this
host API does not yet make brep_progressive_sweep asynchronous in the editor.


## Rush body-stream integration

Async graph evaluation now suspends reachable brep_progressive_sweep nodes,
consumes streamProgressiveBrepProfileBody, and resumes only with the returned
validated body. Completed surface and body sweeps have separate caches;
pending control paths propagate through dependent graph evaluation without
becoming authored-node failures. Surface and body streams share the preview
consumer/heartbeat/abort loop. Native cap and topology refusals still prevent
final scene publication.

Scene preview mapping now accepts brep_tessellate in addition to patch display,
so direct body roots and downstream affine/display chains forward their wall
previews to the existing worker and source-viewport path. These previews remain
walls only: no approximate cap/body topology is exposed as authoritative CAD.
The final nativeGeometry artifact retains the B-rep body and cap holes.

125 tests across six suites passed, including exact async/sync body graph
comparison for five examples, parser cancellation/recovery with a native B-rep
artifact, and the real geometry worker running both surface and contact body
sources. Real worker tests prove transferred nonauthoritative wall previews,
cancellation before body completion, no cancelled-job success, and subsequent
successful construction on the same worker. vue-tsc and scoped diff checks
passed. Evidence: sweep-body-graph-{tests,typecheck}.log.

Live browser observation of body refinement/cancellation and preview support
for geometry-changing composed roots remain outstanding, along with the
remaining mathematical/corner/seam/full-functional coverage requirements.


## Live body preview status and source-to-Solid correction

The source viewport now labels active intermediate geometry as Wall preview,
with section count and sampled deviation/budget in mm. This status disappears
when the renderer restores published geometry; it does not claim continuous
error certification or body-topology admission.

Live browser testing at port 5201 observed the actual intermediate DOM state:
Predprosmotr sten, 3 sections, sampled deviation 0 / 0.001 mm, while the
build was active. After completion this status disappeared and the published
scene had one object, 584 triangles, volume 367.33 and area 392.36. Timings
were observations only, not a performance qualification. Final source-view
screenshot: sweep-body-live-source.jpg. Cancellation remains verified by real
worker tests; it was not captured as a browser interaction in this stage.

The visible To Solid action revealed an integration defect: exactSolidRuntime
expected an OpenSCAD recorded graph and returned zero bodies for an own-NURBS
Rush source, despite its native B-rep artifact. The existing exact-solid
worker route now accepts evaluated native B-rep carriers through the same
scene-to-Solid bridge and receiving-realm document validation. It refuses
non-B-rep display geometry rather than silently returning an empty group;
body and document budgets remain enforced.

20 exact-solid worker/source tests passed, including swept native B-rep cap
holes and manifold incidence after the client roundtrip, and refusal of an
open surface sweep. vue-tsc, Vite build and scoped diff checks passed.
Evidence: sweep-body-solid-route-tests.log, sweep-body-live-{build,typecheck}.log.
The corrected UI action produced one named group with one B-rep body in Solid,
with the visible cap hole retained. Console warnings/errors were empty.
Screenshot: sweep-body-live-solid.jpg. No external publication was performed.


## Corrected Frenet continuation

`Orientation::CorrectedFrenet` and host/Rush `corrected_frenet` add sampled
principal-normal continuation. At each station the principal normal chooses
the hemisphere of the previous double-reflection transported normal. Zero
curvature or a nonunique second derivative uses that transported normal.
For zero initial curvature, the first available principal normal is transported
backwards to seed the starting frame; a wholly straight path uses the authored
normal. Nonfinite curvature still refuses. Strict frenet continues to refuse
an undefined principal normal. Closed corrected frames must agree at their
endpoints; C0 retained patches do not become G1/G2 certified seams.

Native and geometry-bridge transport, TS surface/body options, Rush schema
and language packaging expose the new orientation. Example:
examples/rush/corrected-frenet-sweep.r. Dense guide/contact/affine contracts
remain unchanged. The implementation remains sampled, with continuousBound
and roundingCertified false; continuous frame regularity is not established.

28 native progressive tests passed, including a cubic inflection, a straight
path, and zero initial curvature without a seed-normal jump. Geometry WASM
and language builds finished successfully. 475 packaged tests across five
suites passed, including the actual Rush example, unchanged strict Frenet
refusal, corrected straight rational B-rep/cap topology, and existing sweep,
body, graph and artifact regressions. vue-tsc and scoped diff checks passed.
Evidence: corrected-frenet-{native,build,language-build,typecheck,wasm-tests}.log.
No new live-browser proof is claimed for this mode yet.


## Corrected Frenet boundary qualification

Additional native and packaged tests verify corrected Frenet on a closed
rational circle with one complete twist turn, and on a degree-three C1
straight-to-curved knot join whose second derivative is nonunique. The latter
retains a constant planar binormal profile offset across every retained
station; strict Frenet still refuses. Closed retained endpoint controls and
weights match exactly. Numerical surface evaluation at the two seam boundaries
is compared within 1e-12, since identical geometric/control endpoints can
still evaluate with a tiny floating-point difference; this is not a continuous
rounding certificate. A partial-turn closed twist still refuses.

30 native progressive tests and 44 packaged sweep/body tests passed, together
with vue-tsc and scoped diff checks. Evidence:
corrected-frenet-boundaries-{native,wasm,typecheck}.log. No implementation or
WASM binary changes were required in this qualification stage. C0 path corner
joins and continuously certified frame regularity remain separate requirements.

### Round joins on open spatial polyline paths (2026-10-02)

Added native `paths::round_polyline`, TypeScript `roundPolylineNurbsCurve`, and Rush `round_polyline_curve(points, radius)`. Each corner uses a rational circular arc of the authored radius with tangent line joins; coincident open endpoints, reversals, zero-length edges and overlapping/consumed straight segments refuse. Up to 17 spatial sites are admitted. Parameter intervals scale by endpoint speeds, preserving the rational geometry. No radius shrink, global intersection certificate or G2 join is implied.

Native qualification: three round-path tests and 30 progressive-sweep tests pass. A spatial two-corner path reaches the original 0.01 mm sampled sweep budget with at most 1025 sections; 257 sections were insufficient because profile offsets have a derivative change at the curvature discontinuity. This is sampled control deviation, not a continuous surface-distance certificate. Example: `examples/rush/round-path-progressive-sweep.r`. Public WASM/Rush qualification is recorded separately after packaging. Miter and transition joins, closed-polyline rounding, and swept-profile regularity remain pending.

Packaged qualification completed: geometry and language WASM builds exited successfully; direct Vue/TypeScript checks passed; 50 tests passed across round paths, Bezier paths, progressive sweep and rational B-rep sweep. A capped circular profile follows the spatial rounded path within the 1024-face budget with valid shared topology and zero boundary edges. Global embedding remains explicitly uncertified. Logs: `docs/qualification/sweep-coverage-2026-10-01/round-path-{native,sweep-native,build,language-build,typecheck-direct,wasm-tests}.log`. Browser visual qualification of this new example remains pending.

### Quintic transition joins (2026-10-02)

Native `paths::transition_polyline`, TS `transitionPolylineNurbsCurve`, and Rush `transition_polyline_curve(points, setback)` use retained straight pieces and degree-five polynomial blends. The authored setback is measured along either incident edge. Endpoint controls enforce equal tangent speeds and zero second derivatives; after speed scaling these are C2 joins in real arithmetic. Binary64 output retains C0 knot multiplicities and no continuity certificate. Overlapping transitions and reversals refuse, and setbacks are never silently reduced. Up to 17 open spatial sites are admitted.

Two new native tests independently check quintic Bernstein midpoint/convex-hull behavior and one-sided position, first/second derivatives at every spatial join. Together with round joins all five corner-path tests pass. WASM, Rush and capped-body qualification follows after packaging. Example: `examples/rush/transition-path-progressive-sweep.r`. This does not establish G2 swept-wall seams, global regularity, or a continuous error bound.

For corner P, incoming/outgoing unit tangents a,b, and setback d, transition controls are `[P-da, P-3da/5, P-da/5, P+db/5, P+3db/5, P+db]`. Endpoint first derivatives are `2da` and `2db`; endpoint second derivatives vanish. Scaling each piece interval by its endpoint speed matches the adjacent line derivatives in real arithmetic. The five derivative-control differences are positive multiples of a, a, a+b, b, b. For any admitted nonreversing corner, each has strictly positive projection on a+b; the Bernstein convex combination therefore cannot vanish in real arithmetic. This establishes local transition regularity for the exact authored construction, but is not a binary64 certificate and does not rule out intersections with other pieces or singular swept profiles.

Packaged transition qualification: geometry and language builds completed successfully; direct Vue/TypeScript checks passed; 52 tests passed across round/transition paths, Bezier paths, progressive sweep, and rational B-rep sweep. Rush setback dimensions reject angular input. A capped circular profile follows the spatial transition path within the 1024-face budget, preserving valid topology and zero boundary edges; global embedding remains explicitly uncertified. Native path regression: 8 tests passed. Logs are in `docs/qualification/sweep-coverage-2026-10-01/transition-path-*.log`. Browser visual qualification, miter joins, closed corner joins, swept-wall G1/G2 and continuous bounds remain pending.

### Closed corner joins (2026-10-02)

Added cyclic round and quintic transition construction, including the final corner and last/first adjacency. Native closed constructors accept 3..16 sites without repeating the first point; clamped NURBS output starts immediately after the final corner and shares exact endpoint controls. TS round/transition constructors and Rush nodes now accept `closed: true` (default false). All closed corners use the same authored radius or setback; overlapping or numerically collapsed retained edges refuse instead of silently shrinking the join. A retained-edge margin of 64 binary64 epsilons prevents fully consumed edges from surviving as tiny rounded intervals.

Native path qualification: 10 tests passed, including independent radius checks at all four corners, endpoint tangent/zero-curvature checks, closed RMF budget acceptance and exact retained seam-control equality. TypeScript checks passed. Example: `examples/rush/closed-corner-progressive-hollow-body.r`. Packaged hollow-body qualification is recorded after builds. Closed path tangent continuity does not imply G1/G2 swept-wall seams or global embedding certification.

Packaged closed-corner qualification completed: geometry/language builds and direct TypeScript checks passed; 54 tests passed across corner paths, Bezier paths, progressive sweep and rational B-rep sweep. Both round and transition closed paths preserve two-shell hollow bodies through Rush, the parser and the Solid adapter, with no caps and zero boundary edges. Native path regression remains 10 tests passed, now including both planar and nonplanar closed RMF paths. The example uses two tessellation steps per face to respect the existing 20000-triangle display budget; rational geometry and sampled sweep tolerance are unchanged. Logs: `docs/qualification/sweep-coverage-2026-10-01/closed-corner-path-*.log`. Miter joins, browser visual qualification, arbitrary guide correspondence, continuous error/rounding/embedding and swept-wall G1/G2 remain pending.

### Open spatial miter sections (2026-10-02)

Added `paths::miter_sections`, TS `miterNurbsProfileSections`/`createMiterBrepProfileBody`, and Rush `brep_miter_sweep([[outer],[holes]], points:..., normal:..., miter_limit:...)`. Profiles are retained rational curves in the initial normal plane; the relative numerical planarity tolerance is 1e-10. A minimum-rotation frame carries the transverse coordinates along the piecewise straight path. At each vertex, projection along the incoming segment into the bisector plane constructs one shared oblique miter section. No profile refitting or knot/weight change occurs.

The limit bounds the maximum bisector stretch sec(turn/2). Reversals, exhausted stretch limits and nonpositive longitudinal advances at any corresponding rational control refuse. Positive control advances provide a conservative real-arithmetic exclusion of backward travel along each ruled wall; they do not prove source-profile regularity, global embedding or binary64 correctness. Up to 17 open sites and 64 profiles are supported. Nested loops reuse rational section loft cap audits and shared incidence. Closed miter frames and authored scale/twist laws remain pending. Native path regression: 13 tests passed, including independent right-angle geometry and spatial transverse-location invariants. Example: `examples/rush/miter-hollow-body.r`. Packaged qualification follows after builds.

Packaged miter qualification completed: geometry/language builds and direct TypeScript checks passed; 57 tests passed across miter, round/transition paths, Bezier composition, progressive sweep and rational B-rep sweep. A three-segment spatial hollow miter body has 26 retained rational faces, two audited caps with holes, valid manifold incidence and zero boundary edges. Rush → parser → Solid and internal STEP v5 round-trip retain this body. A derived four-step-per-face mesh agrees within 3% with the independent ideal tube volume `pi*(outerRadius²-innerRadius²)*sum(segmentLengths)`, consistent with its coarse circular tessellation; this is not a certified NURBS volume calculation. Native path regression: 13 tests passed. Logs: `docs/qualification/sweep-coverage-2026-10-01/miter-sections-*.log`. Closed miter, authored scale/twist at miter joins, visual qualification, arbitrary guide correspondence, continuous error/rounding/embedding and swept-wall G1/G2 remain pending.

### Cyclic miter extrusion frames (2026-10-02)

Added native `closed_miter_sections` and `closed: true` to the TS/Rush miter APIs. Cyclic 3..16-site input omits a repeated endpoint. The profile origin is the first site and its plane is normal to the first outgoing segment; the initial shared section is projected into the final/first corner bisector. Every corner is processed, including closing stretch and longitudinal-control advance checks. Minimum-rotation frame closure is checked before exact seam controls are shared. Nonclosing frame holonomy explicitly refuses; distributed twist correction remains necessary for arbitrary closed spatial miter paths. This is not completion of the full closed-miter requirement.

Native path regression: 15 tests pass, including cyclic bisector/extrusion invariants, exact shared seam data, and a spatial nonzero-holonomy refusal. TS body construction routes accepted cyclic sections into periodic rational section loft, preserving nested inner shells without caps. Example: `examples/rush/closed-miter-hollow-body.r`. Packaged qualification follows after builds; global embedding/rounding remain uncertified.

Packaged cyclic-miter qualification completed: geometry/language builds and direct TypeScript checks passed; 59 tests passed across miter, round/transition paths, Bezier composition, progressive sweep and rational B-rep sweep. The cyclic hollow miter body has 32 rational wall faces, two shells with one inner shell, no caps, and zero boundary edges through Rush → parser → Solid and internal STEP v5 round-trip. Its derived mesh volume agrees within 3% with the independent ideal tube volume. A focused five-test miter rerun also validates Rush true/false lowering and rejects a dimensional closed flag. Native path regression: 15 tests passed. Logs: `docs/qualification/sweep-coverage-2026-10-01/closed-miter-*.log`. Nonzero-holonomy correction, continuous scale/twist laws, visual qualification, arbitrary guide correspondence and continuous error/rounding/embedding/G1/G2 remain pending.

### Progressive miter holonomy and scalar laws (2026-10-02)

Added `progressive_miter::Sweep` with rational scale/twist laws and length-distributed closed holonomy correction. It supports skew cyclic paths refused by the constant extrusion miter when their minimum-rotation frame fails to close. Path vertices remain stations at every level; intermediate sections rotate/scale the profile and blend adjacent bisector projection shears. The ideal section family is therefore explicitly defined between corners; no claim is made that a twisted segment remains a straight extrusion. Constant laws and zero correction reproduce the previous extrusion construction.

Scale is positive by its rational controls; signed twist uses radians natively and degrees in TS. Closed scale endpoints must match exactly and twist endpoints differ by whole turns. Corrected endpoint frames are checked before exact seam section data are shared. Profiles retain rational weights, degrees and knots. Levels double per-edge steps, with fourfold probe controls and sampled positive longitudinal advances. Native budgets:1025 retained sections,4097 probes,one million retained controls; probes are evaluated row by row. Continuous error/rounding, source regularity and global embedding remain uncertified.

Native qualification: four tests pass for extrusion equivalence, every-corner agreement on a skew corrected loop, independent scaled full-turn geometry, closed-law mismatch refusals, failed-budget previews and resumable iteration. Public TS full/preview/async-stream interfaces and a face-budgeted accepted-section B-rep constructor are added. Rush/worker/viewport integration and axis-scale/center/frame/guide variants of this miter mode still require work. Packaged qualification follows after geometry build.

Added a progressive-miter phase admission guard after reproducing four-turn aliasing: a 1440-degree law sampled at the initial quarter grid has nearly zero reported control deviation despite unresolved intermediate rotation. Each retained interval now inspects the trimmed rational twist control hull plus holonomy increment; a phase span above pi/2 prevents acceptance, with `phaseResolved:false` in the report. Native regression proves refinement continues and later admits the resolved geometry. This guard does not claim certified continuous error or binary64 rounding. Native progressive-miter tests now total five.

Packaged progressive-miter qualification completed after the phase guard: geometry build and direct TypeScript checks passed; 63 tests passed across six sweep/path suites. A skew closed hollow body formerly rejected for frame holonomy now retains matched seam sections, two rational shells with one inner shell, no caps and zero boundary edges, within the1024-face budget. Full and async host histories match, and cancellation stops between levels. Native qualification: five progressive-miter tests plus fifteen path regressions passed.

Runnable native example: `cargo run --locked --manifest-path crates/Cargo.toml -p nurbs-core --example progressive-miter-holonomy`. It combines nonzero holonomy, one whole-turn twist, and a positive nonconstant closed scale law. Verified output refines1→2→4→8→16→32 steps per edge, admits161 sections with sampled deviation0.0003779465 against budget0.001, and reports correction0.08184121 radians. Exact shared seam controls are asserted. Logs: `docs/qualification/sweep-coverage-2026-10-01/progressive-miter-{native,paths-native,build,typecheck,wasm-tests,example}.log`. Rush/worker/viewport integration, affine/frame/guide extensions, independent STEP conformance and continuous error/regularity/embedding/G1/G2 remain pending.


### Progressive miter: Rush/viewport and continuous real-arithmetic estimate (2026-10-02)

`brep_progressive_miter_sweep([[outer],[hole]], points:..., normal:..., scale:..., twist:..., closed:true, miter_limit:4, initial_steps:1, max_steps:64, max_deviation:0.01mm)` lowers nested rational loops, dimensional sites and angular twist through Rush. `examples/rush/progressive-miter-hollow-body.r` exercises skew-loop holonomy, nonconstant scale and a full-turn twist. The async graph streams retained ruled wall patches and constructs periodic/capped B-rep topology from the accepted sections after a cancellation boundary. There is no synchronous recomputation of the refinement history. Profile patch ranges use half-open indices, matching ordinary sweep previews.

Worker protocol v9 carries optional `phaseResolved` and `continuousErrorUpper`; its admission validator rejects accepted previews with unresolved phase or an excessive continuous estimate, including when sampled error aliases to zero. Preview geometry remains display-only. The source editor keeps the viewport visible for this operation; the status shows the continuous estimate and its uncertified rounding status.

For each retained interval, trim both scalar laws and decompose their rational Bézier spans. Bound the first two scalar derivatives using numerator/positive-weight derivative control hulls and a positive weight lower bound. Let q be the rotating/scaled transverse offset, A(f)=I-t v(f)^T the affine miter shear, and R its original control radius. Bounds are q' <= R(s' + s_max theta'), q'' <= R(s'' + 2s' theta' + s_max(theta'' + theta'^2)). On a smooth local interval the linear-interpolation remainder is bounded by ((1+max|v|)|q''|+2|v'||q'|)/8 in normalized interval coordinates. Across internal law knots use a Lipschitz remainder bound L/2; discontinuous laws are refused. Unchanged positive profile weights transfer the maximum control error to every profile parameter by the rational convex-hull property. Polyline corners are retained, so no error interval crosses a path corner.

`continuousErrorUpper` bounds continuous interpolation error in real arithmetic for the defined analytic miter section family. Acceptance now additionally requires this upper estimate to meet the authored budget. `continuousBound:false` and `roundingCertified:false` remain deliberate: binary64 operations, trims, frame construction and hull computations lack outward-rounded enclosure. This is not a certified global geometry guarantee. Continuous immersion, global injectivity/self-intersection, G1/G2 corner guarantees, affine/frame/guide miter laws and independent STEP conformance remain open.


Preview cancellation qualification exposed a cross-worker delivery race for an already accepted one-level contact body. Protocol v9 therefore supports opt-in preview acknowledgements: BuildCoordinator acknowledges after delivering the preview callback, and the worker awaits acknowledgement (or cancellation) before advancing. A missing acknowledgement fails after five seconds; malformed/stale acknowledgements do not release another node/job. This preserves cancellation issued from the preview handler before topology construction without depending on timer scheduling. Direct clients that do not opt in retain cooperative cancellation semantics.


Final qualification for this increment: geometry and language kernel builds exited0; direct Vue/MCP TypeScript checks passed; seven native progressive-miter tests and123 tests across seven host/graph/worker/coordinator suites passed. The real worker verifies normal completion, cancel-on-preview before topology and recovery for the new miter body and existing progressive/contact modes. The runnable native example admits161 sections with sampled deviation0.0003779465 and continuous real-arithmetic estimate0.0007177125 against budget0.001. The live browser rendered one hollow skew-loop body with2560 display triangles. A retained preview at41 sections displays sampled deviation0.002385 and continuous estimate0.004541 against budget0.01, with rounding explicitly uncertified. At21 sections, sampled error0.009524 already meets the budget while continuous estimate0.018350 correctly requests another level. Narrow source/viewport layout now stacks panes, retaining a usable canvas. Screenshot: `docs/qualification/sweep-coverage-2026-10-01/progressive-miter-viewport.jpg`. New qualification logs use `progressive-miter-{continuous-native,continuous-build,continuous-example,language-build,integration-typecheck,integration-wasm-tests}.log`.

The browser's actual “В Solid” action also completed for this source. The resulting skew-loop body is selectable in the Solid viewport, retaining the native rational B-rep route; screenshot `docs/qualification/sweep-coverage-2026-10-01/progressive-miter-solid.jpg`. Full coverage remains open for certified rounding, continuous regularity/global embedding/G1/G2 and the other matrix gaps.


### Completion contract and scalar-law certification foundation

The eight user-requested completion gates and per-mode E/R/I/J contracts are
fixed in `sweep-contract-matrix.md`; missing evidence remains explicitly open.
`progressive_miter::scalar_certificate::certify` now restricts the original
homogeneous scalar law through outward-rounded blossoms and derives interval
value/first/second derivative hulls. There is no ordinary floating-point trim in
this certificate. Derivatives use the authored knot parameter; progressive miter
scales them to its local interval with outward-rounded domain mapping and
arithmetic. Cell exhaustion or nonrepresentable numeric jets are `Unresolved`
with no partial bounds. The overall sweep report still declares
`continuousBound:false` / `roundingCertified:false`: frame, shear, retained-point
and final interpolation enclosures remain required before whole-sweep
certification and admission can be claimed.

Native regressions include an independent linear law on a nonunit domain,
weighted rational jets on nondyadic probes, a multi-span C0 law, exhausted
budgets, numeric underflow/overflow and a large translated law origin. The
existing progressive-miter geometry/holonomy/alias regressions also pass.


Qualification after integration of the scalar-law enclosure: geometry WASM
build exited0, twelve native progressive-miter/scalar-certificate tests passed,
123 tests across seven packaged sweep/body/protocol/real-worker/coordinator
suites passed, and direct Vue/MCP TypeScript checks passed. The native skew-loop
example admits161 sections with upper estimate0.0007177125336784301 against
budget0.001. Logs: `sweep-scalar-certificate-{native,build,example,wasm-tests,typecheck}.log`
in the existing qualification directory. Full E/R/I/J and completion gates2–8
remain open; this increment supplies certified law jets rather than claiming a
complete geometry certificate.

### 2026-10-02: principal angle enclosure foundation

`progressive_miter::trigonometric_certificate::certify_atan2` encloses the
principal angle of an input rectangle with outward arithmetic, independently
enclosed pi, and an 80-term alternating arctangent series. Reduction uses
reciprocal and pi/4 identities; platform atan2 is used only as a test probe.
Rectangles touching the origin or crossing the negative-axis branch cut return
unresolved (`None`). Away from these cases, constant-sign edge derivatives
place rectangle angle extrema at corners. This provides the missing angle
primitive for closed transport holonomy; its integration into corrected frames
and the full E certificate remains open. No full sweep certificate flag is
promoted by this increment.

Evidence: `sweep-atan-certificate-native.log`, five focused tests passed,
including quadrant/axis rectangles, pi/4 identity and origin/branch refusal.

### 2026-10-02: closed transport principal holonomy

The frame certificate now encloses `atan2(t0 · (end × initial), end · initial)`
with outward cross/dot products and the certified angle primitive. The sign
matches the existing native closing-to-initial correction. Closed transport
requires one additional certificate cell; failure to enclose the angle rejects
the transport certificate and discards all partial vectors. Distributed
correction, station rotation/shear and retained endpoint error still require
integration before full E can be certified. The full sweep flags remain false.

Evidence: `sweep-holonomy-certificate-native.log`, 21 native tests passed. The
skew closed fixture's native correction lies inside the angle enclosure; a
budget ending immediately before the angle cell returns unresolved with no
partial closing normal or angle.

### 2026-10-02: outward chord-length traversal and distributed angle

Frame reports retain outward cumulative edge lengths computed from authored
binary64 sites. `Report::traversal(edge, fraction_interval)` encloses ideal
normalized polyline chord-length interpolation; `correction_at` multiplies it
by the enclosed closing angle (zero for open paths). These methods preserve
unresolved construction and validate the edge/fraction range. Dependency in
cumulative interval subtraction is conservative, not silently discarded.
Rotating the frame, certifying stored stations, and integrating the final E
acceptance bound remain separate pending steps.

Evidence: `sweep-traversal-certificate-native.log`, 22 native tests passed,
including the independent 3/4-edge 5/7 traversal fixture, open zero correction,
invalid indices/fractions, and exhaustion without a partial correction.

### 2026-10-02: rotated-frame enclosure

`Report::rotated_frame` combines an externally certified twist interval with
outward distributed holonomy, then encloses the rotated normal and binormal
using certified sine/cosine and outward vector arithmetic. The twist interval
must cover the requested traversal interval; this method does not infer or
silently sample the law. Wide angle ranges remain conservative enclosures,
not a successful tolerance proof. Unresolved base transport yields no frame.

Evidence: `sweep-rotated-frame-certificate-native.log`, 23 native tests passed.
Checks cover independent zero-angle axes, a continuous twist interval, and
open unresolved transport. Closed skew-loop native frame values at endpoints
and non-dyadic interior sites are contained by the certificate. Stored control
point/shear/interpolation errors remain open; full E flags are unchanged.

### 2026-10-02: interval miter shear

`Report::miter_offset` encloses the ideal shear of a certified world-space
transverse offset: both endpoint plane shifts, their affine blend over an
edge-fraction interval, and the resulting axial subtraction use outward
operations. Missing open endpoint planes contribute exact zero. Cyclic edges
use the closing plane; unresolved base transport produces no offset. Authored
path translation and initial profile projection still need certification and
integration, followed by the retained interpolation remainder. No whole-sweep
certificate or UI acceptance flag is promoted.

Evidence: `sweep-miter-shear-certificate-native.log`, 24 native tests passed.
Independent right-angle cases cover matching incoming/outgoing corner shear,
open endpoint and a full interior fraction interval. Closed skew-loop rotated
normal/shear compositions contain native probes at endpoints and interior
sites, including the last cyclic edge.

### 2026-10-02: station control-point composition and stored error

Frame reports retain the authored sites. `station_point` encloses initial
profile projection, positive scale, corrected rotation, miter shear and path
translation. Supplied scalar ranges must enclose the laws on the same ideal
traversal interval; automated law-domain mapping remains to integrate.
`stored_point_error_upper` bounds the maximum Euclidean distance from a stored
binary64 control point to its ideal enclosure with outward operations. This
term must be combined with an outward interpolation remainder before E may
be promoted. Whole-profile regularity/intersections and B-rep publication
retain their independent obligations.

Evidence: `sweep-station-certificate-native.log`, 26 native tests passed.
Independent translated right-angle stations, scale/fraction ranges and 3/4/5
stored-error fixtures are covered. Native closed skew station control points
are contained at endpoints and interior sites. Full E acceptance is pending;
no WASM rebuild or new browser qualification is claimed for this increment.

### 2026-10-02: certified law-domain mapping into stations

`scalar_certificate::certify_traversal` maps normalized traversal intervals
into each authored knot domain with outward operations. It widens point ranges
within the domain before using the existing rational jet certificate, including
endpoint values. Numeric mapping failure discards all bounds. Frame report
`station_with_laws` now connects ideal chord-length traversal, both independently
budgeted scalar certificates, corrected frame and station composition. Scalar
exhaustion or inability to prove positive scale returns no station enclosure.

Evidence: `sweep-station-laws-certificate-native.log`, 28 native tests passed,
including nonunit-domain endpoints/interior point values, integrated linear
scale stations and scalar-budget refusal. Retained interpolation bounds and
production acceptance integration are still pending; full E remains unclaimed.

### 2026-10-02: outward ideal interpolation remainder

`Report::interpolation_upper` combines certified law jets with outward
chord/domain rates, profile projection radius and miter operator bounds.
On smooth scalar spans it bounds the ideal control trajectory's second
 derivative remainder by `((1+H)q2+2Dq1)/8`; across scalar knots it uses
`((1+H)q1+Dq0)/2`. Affine authored path translation cancels from interpolation
exactly. Independent scalar budgets are preserved. Stored station error must
still be added, and adjacent endpoint ownership and seam construction must be
covered before production acceptance can claim the full E certificate.

Evidence: `sweep-interpolation-certificate-native.log`; the sinusoidal fixture
compares a unit-radius linear-twist midpoint chord error against the certified
bound, and exhausted scalar work yields no remainder.

### 2026-10-02: retained-level certificate composition

`Sweep::certify_level` validates fixed positive-weight profile correspondence
and bounds every retained control trajectory by ideal interpolation remainder
plus the maximum actual stored endpoint error. Both endpoints are compared to
the current edge's ideal construction, covering adjacent corner ownership and
the cloned cyclic endpoint rather than assuming identical floating frames.
Control-interval work and per-law work have independent budgets; any unresolved
cell discards the entire maximum. Production admission is not yet switched.

Initial verification exposed tiny endpoint jet ranges as unresolved. Added
`scalar_certificate::value_traversal`, which evaluates value enclosures with
homogeneous blossoms without width-dependent jets. Station endpoint checks now
use this primitive; interpolation still requires derivative certificates.

Evidence: `sweep-level-certificate-native.log`, 30 native tests passed. The
straight retained-level fixture certifies a small bound, detects a displaced
stored section, rejects changed weights and discards a partial budget result.
Further closed/corner/rational-level qualification precedes E flag promotion.

### 2026-10-02: closed rational-level and law-knot qualification

Extended retained-level tests to a spatial five-edge cyclic path with nonzero
holonomy, rational profile weights, rational scale and a full twist turn.
Scale/twist use independent nonunit knot domains. At8 and16 subdivisions per
edge, certificates resolve and decrease under refinement; nondyadic interior
station/profile probes lie below the certified bounds. A displaced cloned
closing section is charged to the endpoint error. C0 scale knots exercise the
Lipschitz remainder; insufficient law-span budget discards the entire result.

Evidence: `sweep-level-closed-rational-native.log`, 31 native tests passed.
These checks strengthen E coverage; external STEP, whole-domain regularity,
global embedding and the full browser matrix remain independent open gates.
Production acceptance/transport flags have not yet been changed.

### 2026-10-02: native certified admission

Native progressive miter level acceptance now requires `certified_error_upper`
from the retained-level certificate to be present and no larger than the
requested tolerance. Sampled error/phase remain additional diagnostic guards;
the old real-arithmetic estimate no longer controls admission. Reports expose
certified error, cell count and unresolved reason through native JSON/TS types.
Current production certificate limits are100000 control-interval cells and64
cells per scalar certificate; exposing user-selected budgets remains pending.
Full transport-level flags remain conservative until WASM/worker qualification
and retained surface construction audit are complete.

Evidence: `sweep-certified-admission-native.log`, 32 native tests passed.
A translated1e8-coordinate fixture with matching samples is refused at1e-12
because the certified endpoint uncertainty exceeds tolerance, and is accepted
at1e-4. Vue/TS checking passed (`sweep-certified-admission-types.log`). Geometry
WASM build launched; its completion and host tests are still pending.

### 2026-10-02: certified error in worker preview protocol

Geometry worker protocol11 carries optional nullable `certifiedErrorUpper`.
Miter previews populate it from native reports. If present, preview admission
requires a finite certified bound within budget; null cannot accompany an
accepted preview. Ordinary modes without the field retain their prior
contract. NaN and contradictory acceptance envelopes are refused. Added a
public WASM translated-coordinate rounding regression; it awaits the fresh
packed kernel before execution.

Evidence: `sweep-certified-protocol-tests.log`, 20 protocol tests passed;
`sweep-certified-protocol-types.log`, Vue/TS checks passed. Existing geometry
build session5081 is still running after successful release compilation;
packing and new-kernel host/real-worker qualification remain pending.

### 2026-10-02: packed WASM certified-admission qualification

Geometry kernel rebuild completed successfully (release plus native wasm-opt
packing, `sweep-certified-admission-build.log`). Updated packed kernel passed
125 tests across7 suites: progressive miter, constant miter, ordinary progressive
sweep, rational B-rep sweep, worker protocol, real worker boundary and build
coordinator (`sweep-certified-admission-wasm-tests.log`). The public WASM large
translation regression confirms matching samples are refused when the
certified rounding bound exceeds tolerance.

Viewport status now distinguishes a certified wall error bound from an
unresolved certificate; ordinary preview estimates retain their existing
wording. B-rep refusal names unresolved error certification separately from
transport/refinement failure. Vue/TS checks passed
(`sweep-certified-viewport-types.log`). These UI changes still require live
wide/narrow visual inspection. The full contract's R/I/J, miter extensions,
external STEP and visual matrix remain open. Generic full-certification flags
remain conservative pending retained surface-construction audit.

### 2026-10-02: retained B-rep Bezier coefficient path

The B-rep construction audit found ordinary `Curve::decompose` between certified
sections and wall patches. Added an exact coefficient-copy path for clamped,
nonperiodic profiles whose internal knots already have Bezier multiplicity.
Each active span copies its control points and weights directly; normalized
span domains preserve exact real-arithmetic parameter correspondence. General
nonsegmented NURBS still use ordinary decomposition and require an additional
conversion error certificate. Cap projection likewise remains a separate
obligation. Viewport wording now says certified section interpolation bound,
not a certificate for the complete capped B-rep.

Evidence: `sweep-retained-bezier-native.log`, exact coefficients/weights and
nonunit-domain evaluation fixture passed; `sweep-retained-bezier-loft-native.log`,
15 rational-section-loft integration tests passed; Vue/TS checks passed
(`sweep-retained-bezier-types.log`). The new B-rep path has not yet been rebuilt
into WASM or visually inspected. Full certificate flags remain conservative.

### 2026-10-02: whole-domain profile regularity

Added `curve_regularity::inspect`: original-span interval jets separate at
least one tangent component from zero, with adaptive subcells and an explicit
0..100000 created-cell budget. Closed span enclosures cover both one-sided
corner derivatives. Numerical/budget failure is unresolved, never classified
as a singularity. `Sweep::certify_profile_regularity` aggregates all authored
profiles under one shared budget, separately from E. Surface Jacobian and
smooth/seam continuity remain distinct pending certificates.

Evidence: `sweep-curve-regularity-native.log`, two tests passed covering a C0
corner, rational curved profile, stationary interior and exhaustion;
`sweep-profile-regularity-integration-native.log`, 32 progressive miter tests
passed including shared profile-budget integration. A parser ambiguity in
surface_differential's adjacent cast/comparison was fixed with parentheses to
restore compilation. No full R or new WASM qualification is claimed.

### 2026-10-02: retained-wall Jacobian regularity integration

Added `Sweep::certify_wall_regularity`, independent from E admission. It builds
exact stored-coefficient rational ruled patches (unchanged profile weights,
linear section direction) and applies whole-domain `surface_regularity` to
each patch under a shared created-cell budget. Corner sides and the last
cyclic interval are checked independently. Reports identify unresolved
interval/profile pairs; numerical failures and exhausted work cannot promote
regularity. This certifies retained walls, not the unsampled ideal trajectory,
cap geometry, smooth joins or global embedding.

Evidence: `sweep-wall-regularity-native.log`, 32 progressive miter tests passed.
Cases include straight walls, exhaustion after a proved first interval,
collapsed adjacent sections and the closed spatial rational-profile/scale,
full-turn twist fixture at8 and16 subdivisions. The closed fixture proves
all retained wall Jacobians under10000 cells. Public report/WASM/worker R
integration and the broader geometry matrix remain pending.

### 2026-10-02: separate retained R admission and reports

Progressive miter native reports expose profile regularity, nullable retained
wall regularity (null means not run), cell count and unresolved wall indices.
Acceptance now requires certified E, nonzero authored profile tangents and
nonzero retained-wall Jacobians. Wall work is deferred until E/phase/profile
admission succeeds, avoiding expensive singular preview qualification that
cannot yet be admitted. Current independent limits are10000 profile cells and
10000 wall cells; configurable cross-layer budgets remain pending.

Worker protocol12 preserves independent R flags and rejects accepted envelopes
with unproved profile/wall regularity. B-rep errors distinguish tangent and
Jacobian refusal. A public WASM regularity regression is added but waits for
the kernel build (live session18722). Full R for ideal surfaces/caps, global
embedding and continuity are not claimed by these retained-wall flags.

Evidence: `sweep-regularity-admission-native.log`, 33 native tests passed;
`sweep-regularity-protocol-tests.log`, 20 protocol tests passed;
`sweep-regularity-types.log`, Vue/TS checks passed. Singular constant-profile
regression refuses despite a small certified E and leaves wall status null.

### 2026-10-02: packed retained-R qualification and live restoration

Geometry build completed successfully (`sweep-regularity-admission-build.log`).
Updated WASM passed126 tests across7 host/worker suites
(`sweep-regularity-admission-wasm-tests.log`), including public independent E/R
reports and refusal of a stationary profile despite admissible E. Viewport
stores both independent regularity flags and distinguishes proved profile
 tangents, deferred wall work and proved/unproved wall Jacobians. Vue/TS passed
(`sweep-regularity-viewport-types.log`).

Live localhost5202 browser check restored the original closed hollow Rush
source and visibly completed its2560-triangle model. During the new-kernel
build, the accessibility state showed21 sections, interpolation upper0.01835
versus budget0.01, proved profile tangents and wall regularity awaiting E
admission. Final build completed (434ms observed, not a benchmark). Screenshot
`sweep-certified-source-restored.jpg` records the restored source/model;
`sweep-regularity-preview-wide.jpg` captured after preview had already completed
and therefore does not prove preview labels. Narrow R statuses and the complete
visual matrix remain open. Existing unresponsive tab7 was preserved; working
verification tab9 remains available for continuation.

### 2026-10-02: inter-patch separation audit foundation

Added `sweep_pair_audit::inspect` over every distinct retained patch pair.
Positive-weight convex-hull boxes first certify positive clearance; overlapping
boxes use the existing original-domain rational surface-distance hierarchy.
Pair and refinement-cell budgets are independent. Exhausted work retains all
unresolved pair identities. Shared-boundary declarations never remove a pair:
they report pending boundary/interior ownership qualification. A separated
pair report does not certify self-patch injectivity, shell containment or
whole-sweep embedding. Outer-patch BVH scheduling, sweep topology integration,
adjacent-boundary refinement and those remaining I obligations stay open.

Evidence: `sweep-pair-audit-native.log`, focused native test passed: all three
separated plane pairs, pair-budget refusal, explicitly unresolved shared
boundary and coincident patches. No WASM or browser integration is claimed.

### 2026-10-02: outer patch BVH and pair coverage

The inter-patch audit now builds a deterministic median-split BVH over exact
positive-weight convex-hull boxes. Node pairs partition the distinct leaf-pair
universe; separated disjoint groups are certified together. Shared-boundary
pairs force descent and remain unresolved at leaves. The pair budget now
counts outer BVH node-pair visits; exhaustion expands all pending group pairs
into explicit unresolved identities, without deleting or duplicating pairs.
Rational distance refinement retains its independent created-cell budget.
Sorting and axis selection are scheduling heuristics only.

Evidence: `sweep-pair-bvh-native.log`, three tests passed: exact coverage of120
pairs at several exhaustion budgets, shared-boundary/coincident refusal, and
positive separation of tilted parallel rational patches with overlapping
boxes (requiring the inner rational hierarchy). Full sweep topology mapping,
allowed-boundary interior proofs, self-patch injectivity and shell containment
remain pending; no whole embedding flag or WASM integration is promoted.

### 2026-10-02: retained sweep wall pair topology integration

`Sweep::inspect_wall_separation` constructs exact stored-coefficient ruled
walls and connects the outer BVH/inner rational hierarchy to retained section
layout. It validates profile correspondence and exact cyclic seam, derives
shared path-neighbor pairs including last-to-first ownership, and keeps these
pairs unresolved until interior/boundary proofs exist. Cross-profile loop
ownership, self-patch injectivity and shell containment are still separate
pending obligations; no global embedding admission or transport flag changes.
The audit limits this retained wall set to1024 patches and passes independent
pair/cell budgets through to the hierarchy.

Evidence: `sweep-wall-separation-integration-native.log`, 33 progressive miter
tests passed. Three straight intervals prove the nonneighbor separation and
retain both shared-boundary pairs. The closed spatial rational fixture retains
the cyclic shared pair, and modification of its closing clone is rejected as
an invalid exact seam. Public WASM/Rush integration remains pending.

### 2026-10-02: sufficient exact shared-boundary certificate

The pair audit can now certify boundary-only intersection for rational ruled
patches with identical stored boundary control points/weights, a shared
coordinate plane, and all opposite control columns strictly on opposite sides.
Positive rational weights make every interior point strictly off that plane;
therefore the intersection is confined to the matching boundary. The report
separates `boundary_only_pairs`/`all_pairs_compatible` from positive-clearance
`all_pairs_separated`. Matching a boundary alone never proves this condition.
This is a sufficient criterion; arbitrary spatial shared planes, vertex-only
adjacency, patch injectivity and shell containment remain pending.

Evidence: `sweep-adjacent-boundary-native.log`, four pair tests passed, including
opposite-side adjacent patches and folded-back refusal;
`sweep-adjacent-boundary-integration-native.log`, 33 progressive miter tests
passed. Straight retained walls now prove both shared-boundary pairs and their
nonneighbor separation. Closed skew shared pairs remain unresolved as before.
No whole embedding or new WASM qualification is claimed.

### 2026-10-02: sufficient oblique shared-plane certificate

Extended boundary-only qualification to an exact oblique plane defined by
three represented boundary control points. Checked dyadic i128 orientation
proves every shared control is on that plane and all opposite columns have
strict opposite signs. Arithmetic overflow, excessive exponent range, collinear
plane candidates or more than64 control rows leave the case unresolved; no
rounded plane normal or rounded zero is used. This proves the geometry of
stored binary64 data, not an earlier ideal construction. Wider exact arithmetic
and arbitrary boundary/interior subdivision remain pending.

Evidence: `sweep-oblique-boundary-native.log`, five pair tests passed, including
an oblique plane, folded-back refusal and both extreme exponent and sign-bit
shift overflow regressions; `sweep-oblique-boundary-integration-native.log`,
33 progressive miter tests passed. Full patch injectivity, containment, public
transport and global embedding admission remain open.

### 2026-10-02: whole-chart strictly monotone projection

Added `surface_monotonicity::inspect` alongside the existing contraction-based
`surface_injectivity` API. A single fixed signed coordinate projection must have
a positive-definite symmetric Jacobian on every original knot rectangle.
Outward rational jets are converted from local cell coordinates to the authored
parameter domain. Continuity across internal knots and a nonperiodic rectangular
domain are required; budget exhaustion discards the certificate. This is a
sufficient global chart-injectivity criterion, not an intersection witness or
a complete injectivity decision procedure. No sweep embedding flag is promoted.

Three native tests pass: an oblique chart on nonunit parameter domains, a
nonconstant rational-weight chart with reversed projected orientation, and a
piecewise-affine locally regular fold whose endpoint images coincide. The fold
cannot be accepted using different projections on its two spans. Zero and
exhausted budgets remain unproved. Evidence:
`docs/qualification/sweep-coverage-2026-10-01/sweep-surface-monotonicity-native.log`.
Full wall audit integration, inter-shell containment and public I reports remain
open. The completion matrix now reflects the already verified miter E/R admission.

### 2026-10-02: combined retained-wall chart and pair audit

Added `sweep_wall_audit::inspect` and
`Sweep::inspect_wall_geometry`. The latter reuses the same validated retained
wall construction and exact cyclic seam ownership as `inspect_wall_separation`;
chart injectivity receives its own shared cell budget, independent of pair
visits and pair refinement. `charts_and_pairs_certified` requires every chart
and every distinct pair to be certified. Unresolved chart indices are retained.
This flag does not claim shell containment, cap ownership or global Solid
embedding; cross-profile loop adjacency still needs explicit ownership.

The native aggregate test verifies separated regular charts, exhaustion of the
shared chart budget and a folded chart whose distinct-patch separation succeeds
while the aggregate correctly remains unproved. All 33 progressive-miter
regressions pass after factoring retained-wall construction. Evidence:
`sweep-wall-audit-native.log` and `sweep-wall-audit-miter-native.log` in the
qualification directory. Public WASM/worker integration remains pending.

### 2026-10-02: constructor-level combined audit qualification

The existing miter retained-level tests now exercise `inspect_wall_geometry`
on actual constructor sections. An open three-wall chain certifies chart
injectivity and permitted boundary contacts together. Giving the chart budget
zero leaves all three charts unresolved although the pair audit succeeds.
A closed skew rational-law fixture likewise retains every unresolved chart
under zero budget; cyclic contact obligations remain independently unresolved.
Initial monotonicity knot-grid allocation is now bounded by the cell budget
before derivative evaluation, with no partial certificate on overflow.

After this change, three monotonicity tests, the aggregate wall test and all
33 progressive-miter regressions pass. Logs: `sweep-surface-monotonicity-native.log`,
`sweep-wall-audit-native.log`, `sweep-wall-audit-miter-native.log`. This is native
constructor evidence; public I reporting, cross-profile boundary ownership,
containment, caps and the complete eight-gate goal remain open.

### 2026-10-02: adaptive chart monotonicity cells

Monotonicity now bisects unresolved rectangles, alternating parameter axes
with a maximum depth of12. Each derivative enclosure counts against the shared
cell budget. A definitely nonpositive diagonal rejects that projection; an
ambiguous enclosure may refine. All accepted leaves across all original spans
still use one fixed projection. Depth limits, unsplittable floating endpoints
and numerical enclosure failures leave the projection unproved. Splitting
cannot change the chart or discard an unresolved part of its domain.

A separable positive rational reparameterization with tensor weights1,2,2,4
is unproved with one cell and certified with refinement. All four chart tests,
the aggregate wall test and33 miter regressions pass. Existing folded-chart
and reversed-orientation refusals remain covered. Evidence uses the same
`sweep-surface-monotonicity-native.log`, `sweep-wall-audit-native.log` and
`sweep-wall-audit-miter-native.log`. Full embedding and public I integration
remain open.

### Joint B-rep chart injectivity with independent oblique budget

`face_injectivity::inspect_with_linear` preserves the original contraction
report per retained face and adds optional whole-chart linear monotonicity
evidence only for faces not already proven. Its refinement cells use a separate
shared budget bounded by 100000. Exhausted faces remain explicitly present;
original `inspect` keeps its previous behavior with no oblique budget.
`boundary_embedding::inspect_with_linear` consumes the combined face result,
while still requiring exact coedge identities/joins, valid simple oriented trims
and complete pair classification before proving an embedded boundary. Nesting
and material orientation remain additional solid obligations.

Native qualification: all 18 actual faces of a two-span hollow loft prove
injectivity with 10000 oblique cells; contraction reports are unchanged, a
one-cell budget leaves unproved face IDs, excessive budgets are refused and the
input remains unchanged. Three face tests and 35 boundary-related tests passed
(`sweep-joint-face-injectivity-native.log`, `sweep-joint-embedding-native.log`).
A joint-audit test proves every chart but deliberately exhausts exact agreement
and pair traversal: no hull contact certificate is admitted and embedding stays
unproved. Packaged WASM/Rush integration of the new joint API remains pending.

### Joint retained-pair admission qualification

An explicitly authored exact-quadrant single-span solid sweep now exercises
the positive prerequisite path in `boundary_embedding`: exact coedge identities
and joins, simple oriented trims and every chart's injectivity pass together.
The first wall pair is admitted as an authored shared boundary. A one-pair
budget preserves the pending suffix and does not prove embedding.

The full 15-pair traversal classifies the four adjacent wall pairs and separates
the opposite wall pairs and endpoint caps. All eight cap/wall pairs remain
unresolved in this joint API, even though the separate native cap contact audit
supports this geometry. This identifies the missing certificate composition:
cap/wall allowed-contact evidence must enter the joint pair classifier under
its actual model prerequisites. The fixture remains unproved as a whole.
Qualification logs: `sweep-joint-contact-admission-native.log` (pair reasons),
`sweep-joint-contact-regressions-native.log` (36 passed boundary-related tests).
Whitespace check passes. No WASM rebuild was performed in this qualification.

### Native composition of cap contacts into joint pair classification

`boundary_embedding::inspect_sweep` accepts explicit cap IDs and independent
per-cap budgets, recomputes actual native cap contact prerequisites and retains
each complete report. Only when the joint exact-domain prerequisites and the
individual cap prerequisites pass are its proven allowed wall/edge entries
admitted to pair classification as `SharedBoundary::SweepCap`. Unresolved entries
continue through ordinary bounded pair refinement. The existing APIs preserve
their previous behavior. Separate cap reports do not skip pair traversal or
authorize a whole body by themselves.

The six-face exact-quadrant sweep now proves its entire embedded boundary:
all 15 pairs are classified, with eight native cap/wall boundary certificates.
Zero cap exact-work budget removes those certificates and leaves the result
unproved. This proves embedded boundary geometry for this fixture, not general
sweep containment, outward material orientation or the full eight-gate goal.
Qualification: 36 boundary-related native tests passed and geometry-bridge
compilation passed. Logs: `sweep-composed-cap-embedding-native.log`,
`sweep-composed-cap-regressions-native.log`, `sweep-composed-cap-bridge-check.log`.
Packaged WASM/public joint sweep API integration remains pending.

### Packaged public joint sweep embedding audit

`brep_sweep_embedding_audit` and TS `inspectSweepEmbedding` now expose the native
joint audit with explicit contraction, oblique refinement, exact boundary,
trim, pair geometry/domain and independent per-cap budgets. Output preserves
actual pair IDs/reasons, the unvisited pair suffix, unresolved face IDs and cap
contact reports. `boundaryEmbeddingCertified` is independent of
`solidGeometryCertified:false`: material orientation and shell nesting are not
promoted by this operation. Empty or duplicate cap selections are refused.

Qualification: packaged release/wasm-opt build exited zero (11758458 to 10452552
bytes); seven public/worker/coordinator suites passed 80 tests. The new packaged
operation proves the exact-quadrant boundary, returns eight unresolved cap/wall
pairs with zero cap exact-work, preserves the pending suffix with one pair,
refuses malformed selections and preserves the input model. TypeScript and
whitespace checks passed. Logs: `sweep-public-embedding-check.log`,
`sweep-public-embedding-build.log`, `sweep-public-embedding-tests.log`,
`sweep-public-embedding-tsc.log`. Automatic progressive/Rush report integration,
full shell containment and general sweep coverage remain unfinished.

### Progressive sync/stream and Rush retain joint embedding evidence

Open progressive miter construction now automatically audits the retained B-rep
and stores `embedding` in both synchronous and streamed results. Rush
construction reports carry the same evidence. Independent `embeddingBudgets`
control joint and per-cap stages; defaults bound work and preserve unresolved
entries. Existing independent wall/cap reports remain available and a successful
boundary certificate does not promote the global solid flag. Cancellation is
checked immediately before and after the native joint audit in streaming.
Closed no-cap bodies currently retain `embedding:null`: periodic shell audit
and nesting require separate integration rather than an artificial cap choice.

Qualification: six public/worker/coordinator suites passed 84 tests, including
sync/stream equality of the full embedding report, actual retained cap IDs,
open Rush evidence and closed Rush null evidence. TypeScript and whitespace
checks passed. Logs: `sweep-progressive-embedding-tests.log`,
`sweep-progressive-embedding-tsc.log`. The previously qualified packaged native
operation was reused; this integration required no new WASM build. Strict body
admission, closed shell evidence, containment and the other sweep modes remain
open.

### Sweep boundary evidence composed with native volume validity

`volume_validity::inspect_sweep` now runs the joint sweep boundary audit before
shell nesting and authored material orientation. Explicit cap IDs use combined
cap/wall contacts; an empty selection uses the no-cap oblique boundary audit
without inventing caps. Existing volume inspection preserves its behavior.
Nesting and orientation are not attempted when boundary embedding is unproved.
Per-stage budget accounting and unresolved orientation attempts remain visible.

The exact-quadrant sweep proves boundary embedding, consistent shell roles and
outward material orientation, yielding a native volume certificate for this
fixture. Flipping all shell face uses preserves embedded boundary geometry but
proves inward orientation and rejects volume certification. A one-cell/one-domain
orientation budget preserves the boundary certificate but leaves volume unproved.
Zero cap exact work leaves boundary unproved and nesting/orientation unattempted.
A no-cap cube also proves through the same entrypoint; this is not yet evidence
for a periodic miter shell. Four native volume tests passed, including existing
cavity orientation and nested material regressions. Log:
`sweep-volume-composition-native.log`. Whitespace check passes. The new volume
composition is not yet exposed through packaged WASM or Rush; periodic hollow
sweep qualification and the full eight gates remain open.

### Periodic hollow miter chart qualification

A native square-path periodic hollow miter fixture now exercises two actual
shells, 32 retained wall faces and the authored inner-shell role through the
sweep volume entrypoint with no cap selection. The initial candidate projected
long station motion into the transverse row and failed chart injectivity on
all 32 faces. The candidate proposal now removes the station axis from its
transverse row when the station chord is exactly axis aligned. Only proposal
selection changes; every chart still requires the complete outward continuous
monotonicity proof under one fixed projection.

All 32 charts now prove injectivity within the shared 10000-cell budget. A skewed
rational quarter-wall regression proves that the old mixed projection fails
while the transverse candidate succeeds; zero budget stays unproved. Three
native projection tests passed. The final rerun against the current shared
worktree additionally reports exact boundary agreement, full pair classification
and embedded boundary proven. The earlier run left those stages unproved;
that earlier state must not be treated as current. Volume certification still
stays false; nesting and orientation require stage-specific diagnosis next.
Logs: `sweep-periodic-volume-diagnosis-native.log`,
`sweep-periodic-volume-transverse-native.log`, `sweep-transverse-projection-native.log`.
Whitespace check passed. This proposal change is not yet in the packaged WASM;
periodic nesting and orientation qualification remain next work.

### Periodic hollow miter native volume certificate

Stage diagnostics showed shell nesting already proved with parents
`[None, Some(0)]`, but box-centre orientation rays passed through the toroidal
empty region without crossings. Additional bounded candidate origins aim at a
retained chart away from dyadic subdivision seams. Evaluated points propose
rays only: all crossings, trim inclusion and normal signs still require native
interval certificates. Six candidate attempts per shell share the existing
global orientation budgets, preserve exhaustion and reject nonfinite proposals.

The 32-wall two-shell square-path hollow miter now proves its entire native
volume: exact boundary agreement, valid trims, whole-chart injectivity, complete
pair classification, shell nesting and correct outer/inner material orientation.
Both shells certify on the fourth attempt; the original three rays have no
crossings. Tests assert the positive volume result, exact parents, outward true
for the outer shell and false for the inner, shared work bounds and input
immutability. All five native volume tests passed, including inverted orientation
and exhausted-budget regressions. Logs: `sweep-periodic-volume-stages-native.log`,
`sweep-targeted-orientation-off-seam-native.log`. Whitespace check passed.
This is evidence for this periodic hollow miter fixture, not the complete sweep
matrix. Packaged WASM and automatic Rush volume reporting remain pending.

### Public packaged sweep volume certificate

`brep_sweep_volume_audit` and TS `inspectSweepVolume` expose the complete native
volume composition with explicit independent nesting and orientation budgets.
The report separates embedded boundary, shell parent/role evidence, orientation
attempts and global solid geometry. Empty cap selection supports no-cap closed
shells. Unproved boundary leaves nesting null and orientation attempts zero;
orientation budget exhaustion preserves boundary evidence but never certifies
volume.

Qualification: packaged WASM build exited zero; six public/worker/coordinator
suites passed 86 tests. The exact open sweep certifies, reversed material
orientation refuses volume while preserving embedded boundary, and a one-cell
orientation budget stays unproved. The packaged periodic hollow square-path
miter certifies volume with parents `[null,0]` and outward orientations
`[true,false]`, preserving the model. TypeScript and whitespace checks passed.
Logs: `sweep-public-volume-check.log`, `sweep-public-volume-build.log`,
`sweep-public-volume-tests.log`, `sweep-public-volume-tsc.log`. Automatic volume
reporting in progressive/Rush and strict Solid admission remain pending; this
fixture does not close general sweep coverage.

### Automatic progressive/Rush volume evidence

Progressive miter construction now retains `volume` for both open and closed
bodies, through synchronous construction, streaming and Rush construction
reports. Explicit `volumeBudgets` are independent of diagnostic embedding and
cap budgets. Streaming checks cancellation before and after the native volume
call. The original global sweep flag is not promoted: retained solid geometry
evidence remains independent of full authored-family error and smooth-join
guarantees. Closed bodies can now report shell parent and material orientation
evidence without cap selection, even though their separate cap-based `embedding`
field remains null.

Qualification: six public/worker/coordinator suites passed 86 tests; TypeScript
and whitespace checks passed. Tests cover equal sync/stream volume reports,
open/closed Rush payloads and an independent one-pair volume budget that keeps
volume/boundary unproved, preserves the pending suffix and leaves nesting and
orientation unattempted. Logs: `sweep-progressive-volume-tests.log`,
`sweep-progressive-volume-tsc.log`. The current packaged volume operation was
reused. Strict Solid admission, viewport presentation, complete E/R/J and the
full mode/visual/STEP matrix remain open.

### Snapshot-local sweep evidence reaches the display transport

The native geometry artifact for the actual progressive miter source node now
retains compact `sweepEvidence`: volume evidence and separate authored-error,
profile/wall regularity and continuous-bound indicators. Evidence belongs to
that exact serialized source snapshot and is covered by its artifact revision.
Only known display conversions retain source correspondence; derived or edited
nodes do not automatically inherit another body's certificate. This closes the
previous evidence loss between the construction report and display mesh's
native geometry payload without changing the worker envelope contract.

Qualification: four public/worker/coordinator suites passed 78 tests; TypeScript
and whitespace checks passed. A real open Rush build with display tessellation
retains volume evidence exactly equal to its source construction report while
keeping `continuousBound:false`. Logs: `sweep-snapshot-evidence-tests.log`,
`sweep-snapshot-evidence-tsc.log`. Viewport presentation and strict Solid
admission still require integration; snapshot metadata does not replace native
revalidation after geometry edits or external input.

### Viewport status distinguishes retained solid and authored sweep guarantees

The current published scene displays snapshot-local sweep evidence: solid
geometry, full continuous error and profile/wall regularity are separate fields.
Missing or unproved flags never become certified. The panel is hidden during
builds, source replacement and stale publications, and only appears when the
rendered source matches the editor. Edited or unrelated artifacts without source
evidence have no sweep panel. Presentation does not authorize Solid admission.

Qualification: TypeScript and whitespace checks passed; five public/worker
suites passed 79 tests, including positive solid geometry with incomplete full
error, missing evidence and nonboolean values. Logs:
`sweep-viewport-evidence-tests.log`, `sweep-viewport-evidence-tsc.log`.
Live browser inspection was attempted on the existing viewport tab but CDP
focus emulation timed out; no rendered wide/narrow evidence is claimed for this
panel. Visual qualification and strict native Solid admission remain open.

### Progressive source Solid admission recomputes native material geometry

The scene-to-Solid bridge now recomputes volume validity for artifacts whose
actual source node is `brep_progressive_miter_sweep`, using the B-rep after
display placement. Stored viewport certificate flags are not admission inputs.
Open sources select their retained endpoint caps; closed sources use no caps.
Bounded explicit budgets cover exact boundaries, trims, chart injectivity,
contacts, nesting and orientation. An unproved result refuses transfer with the
failed stage and orientation shell IDs. Other source node kinds preserve their
existing behavior; derived-node provenance and full cross-mode admission remain
separate work. This checks retained material geometry, not complete authored E/J.

Qualification: five native/public/worker suites passed 49 tests; final focused
suite passed 3 tests after stage-specific error wording, and TypeScript passed.
A positive source transfers; a reversed-shell B-rep is refused even with a
positive stored certificate flag. Unrelated source types are unaffected.
Logs: `sweep-solid-admission-tests.log`, `sweep-solid-admission-final-test.log`,
`sweep-solid-admission-tsc.log`. Whitespace check passed. Live visual transfer,
edited/derived provenance, remaining modes and full sweep guarantees are open.

### Affine-derived progressive Solid admission

Solid admission now follows actual `transform` input chains to identify a
progressive miter source. Affine B-rep transforms preserve retained face IDs,
so provenance identifies the appropriate cap scope while the certificate is
recomputed on the actual transformed model. No source certificate is inherited.
Cyclic or missing transform provenance is refused. Boolean, fillet and chamfer
provenance still require their own topology-aware scope and are not qualified
by this change.

Qualification: five public/worker suites passed 49 tests; TypeScript and
whitespace checks passed. A translated exact sweep with no inherited evidence
passes native admission; reversed material orientation under the transform
chain is refused, and cyclic provenance is refused. Existing exact Solid worker
and source regressions pass. Logs: `sweep-derived-admission-tests.log`,
`sweep-derived-admission-tsc.log`. This does not prove general affine source
error or arbitrary transformed caps, and live visual/STEP coverage remains open.

### Live viewport status and Solid refusal qualification

A fresh temporary browser tab recovered live access without modifying the
source in existing tabs. The current spatial closed scale/twist hollow miter
renders 2560 triangles and shows unproved solid geometry/full continuous error
with certified profile/wall regularity. The status was inspected at the default
1280x720 viewport and at 640x800. All status fields remain visible in the narrow
layout. Temporary viewport overrides were reset and the temporary tab closed.

The real `To Solid` action refused this source with the explicit failed stage
`boundary embedding`; the existing Solid scene retained its one body. No source
replacement or body deletion was performed. Screenshots:
`sweep-viewport-live-status.jpg`, `sweep-viewport-live-status-narrow.jpg`,
`sweep-solid-live-refusal.jpg`. This closes this status/refusal scenario only.
The complete positive transfer, cancellation, restoration and all-mode visual
matrix remains open.

### Independent STEP material comparison with native volume evidence

The STEP oracle exporter now records the bounded native volume audit per actual
fixture. The OCCT verifier keeps its import/geometry/topology/volume checks
independent and additionally compares material orientation counts and shell roles
when a positive native certificate is present. An unproved native report stays
unproved even when OCCT accepts the exported fixture. A positive claim that
disagrees with imported shell volume signs fails the external verification.

All three current fixtures passed OCCT. The closed planar hollow miter has both
a positive native volume certificate and external material agreement. The
progressive scale/twist and open spatial fixtures remain natively unproved;
OCCT validity is not substituted for continuous native certification. A negative
manifest deliberately declaring the inner shell outward, while keeping STEP
unchanged, was rejected (exit 1, external material agreement false). Positive
fixtures were preserved in `external-step-volume`; the mutation is isolated in
its `negative-native-orientation` subdirectory.
Logs: `sweep-volume-step-export.log`, `sweep-volume-step-occt.log`,
`sweep-volume-step-negative-orientation.log`. Whitespace check passed. The full
mode/law/holonomy STEP matrix remains open.

### 2026-10-02: linear oblique shared-boundary certificate

The sufficient shared-plane criterion now covers two-control rational line
profiles as well as3..64-control profiles. Auxiliary finite represented points
propose planes through the first two boundary controls; checked exact dyadic
orientation validates all shared controls on the plane and every far control
strictly on opposite sides. Proposal rounding is not evidence: only the exact
orientation of the final represented coordinates certifies the plane. Overflow,
collinearity and collapsed/interior-on-plane controls remain unproved.

A rationally weighted oblique common line is accepted with opposite wall
interiors and no pair refinement cells. Same-side foldback and a collapsed
wall are rejected by this sufficient criterion. Six pair-audit tests and33
miter regressions pass. Evidence: `sweep-linear-boundary-native.log` and
`sweep-linear-boundary-miter-native.log`. Generic nonplanar contacts,
cross-profile loop ownership, containment and public global-I admission
remain open; no embedding flag is promoted.

### 2026-10-02: retained per-chart evidence and refusal reasons

Combined wall reports now retain one monotonicity report per input chart in
input order: certificate, projection, work count and unresolved reason.
Aggregate indices remain available. Monotonicity distinguishes numerical
enclosure failure, unsplittable parameter intervals and depth exhaustion
when no projection succeeds; cell exhaustion remains an immediate refusal.
These reasons do not declare a geometric violation. Invalid disconnected
knot multiplicities are rejected by the existing surface validator before
an audit report can be returned.

Four monotonicity tests, two combined wall tests and33 miter regressions pass.
The tests preserve successful projection evidence and the exhausted-budget
reason for a subsequent chart, and verify structured invalid-input rejection.
Evidence: `sweep-surface-monotonicity-native.log`, `sweep-wall-audit-native.log`,
`sweep-wall-audit-miter-native.log`. Public report transport and full global-I
certification remain open.

### 2026-10-02: independent OCCT spatial hollow miter import

Added `scripts/export-sweep-step-oracle.mts` to export the actual public WASM
miter body and hash its STEP file. `scripts/verify-sweep-step-occt.py` imports
that file using the existing `/tmp/cad-roadmap-ocp/bin/python` environment,
checks OCCT BRep validity, face/solid counts and signed volume against the
independent ideal constant tube formula. The oracle writes explicit pass/fail
evidence and refuses an empty manifest or changed STEP hash.

The open spatial hollow miter fixture passes:26 faces,1 solid, valid OCCT
BRep and volume20.570744616526657 mm³ against20.57074461266989 mm³ expected
(relative error1.8748789342306223e-10). Export and external verifier both
exit0. Evidence: `external-step/{manifest.json,open-spatial-hollow-miter.step,
opencascade-sweep.json}`, `sweep-external-step-export.log` and
`sweep-external-step-occt.log`. This covers one fixture's import/topology/volume;
closed/variable-law fixtures, source surface comparison, seam and orientation
checks, holes and shell ownership still require broader external qualification.
Gate7 remains open, as do the other unfinished gates.

### 2026-10-02: closed hollow OCCT STEP and shell orientation

The independent STEP matrix now includes the public constant closed planar
miter body. OCCT verifies32 faces,1 solid,2 shells, no cap-hole faces and
valid BRep topology. Signed shell volumes are31.415926543256955 and
-5.026548246921109 mm³, confirming opposite shell orientations for this
fixture. Net volume26.38937829633585 mm³ agrees with the independent tube
formula26.38937829015426 mm³ (relative error2.342453953533674e-10).
The open spatial fixture independently verifies1 shell and2 cap-hole faces.
The verifier requires exactly one positive-volume shell and the expected
number of negative-volume shells, expected hole-face count and positive
analytic net volume. Both cases pass; export and verifier exit0. Evidence
remains `external-step/opencascade-sweep.json` and the STEP export/oracle logs.
This proves fixture topology/orientation/volume, not general shell containment
or full continuous surface/seam conformance. Variable-law fixtures and the
remaining complete STEP matrix are still open.

### 2026-10-02: OCCT retained-wall positions and first derivatives

STEP export manifests now include25 source evaluations per rational wall,
including all four boundaries. The external oracle matches each wall to a
distinct imported face by maximum sampled position error, then compares
OCCT D1 derivatives against source du/dv at every site. Thus24 open and32
closed walls are covered without assuming STEP face order. Maximum position
error is3.552713678800501e-15 mm for both fixtures; maximum derivative errors
are5.329070518200751e-15 and3.6914915568786455e-15 respectively, below the
1e-8 declared comparison tolerance. Both export and external oracle exit0.

Negative checks use temporary copies and confirm exit1 for a changed source
position, changed derivative and STEP hash mismatch. Evidence:
`sweep-external-step-negative.log` and updated `external-step/opencascade-sweep.json`.
This is discrete source/STEP wall and boundary conformance, not a continuous
certificate for the full parameter domains, cap trims or G1/G2 seams.
Variable-law and other-mode fixtures remain required for gate7.

### 2026-10-02: progressive scale/twist external STEP fixture

Added an accepted public progressive miter fixture on a straight10 mm path
with scale1→1.5 and twist0→30 degrees. Native/WASM admission selects4 steps
(5 retained sections), certifiedErrorUpper0.0051642676028716 mm against
budget0.02 mm, with profile and wall R certified. Full continuousBound and
roundingCertified flags remain false. The exported34-face hollow solid passes
OCCT validity,1 shell,2 cap-hole faces and positive orientation checks.

Independent expected retained volume integrates the quadratic area of each
linearly interpolated rotated/scaled circular section:
A(v)/A0=(1-v)^2*a²+2*v*(1-v)*a*b*cos(deltaTheta)+v²*b².
The per-interval integral is(a²+a*b*cos(deltaTheta)+b²)/3, multiplied by
interval length and the outer-minus-inner base area. This compares retained
geometry, not the author's continuously rotated family. OCCT relative volume
error is1.5388064453283726e-10; maximum sampled position and derivative errors
are8.881784197001252e-16 and2.953300581774766e-15. All three current fixtures
pass; export and verifier exit0. Evidence is the updated external STEP
manifest/report and qualification logs. Closed variable laws, nonzero holonomy,
other frame/guide modes and continuous seam/surface comparison remain open.

### 2026-10-02: full-domain retained-wall STEP coefficient comparison

Manifests now retain source wall NURBS coefficients. For each sampled face
match, OCCT spline degrees, control counts, periodic flags, expanded knots and
weights are compared to the source. All three fixtures retain exactly equal
knot values and strictly positive weights. Corresponding pole displacement
squared is compared with the squared1e-8 mm tolerance using exact Python
Fractions of the represented binary64 values, avoiding a rounded distance
decision. Positive common rational basis functions imply point displacement
at every parameter is a convex combination of pole displacements; therefore
this coefficient test bounds the entire retained wall rectangle, not just
the sample grid. All three fixture wall sets pass. This certifies retained
wall source/STEP distance for those fixtures; it does not prove author-family
error, cap trim conformance, derivative smoothness or global embedding.

Five temporary-copy negative checks reject position, derivative, hash, control
and weight corruption. Export/oracle and negative checks exit0 overall; each
negative oracle run exits1 as required. Evidence is the updated external
manifest/report, `sweep-external-step-occt.log` and
`sweep-external-step-negative.log`. Full gate7 mode coverage remains open.

### 2026-10-02: external STEP oriented-edge and shell closure audit

The external verifier now independently maps OCCT edges to ancestor faces,
requires exactly two face uses for each edge and compares edge counts to the
exported source BRep. Every use pair must contain one forward and one reversed
orientation; every imported shell must be closed according to BRep_Tool.
All three fixtures pass:72 progressive edges,56 open spatial miter edges and
64 closed planar miter edges. The closed fixture has two independently closed
shells. Source geometry coefficient, sampled jet and volume checks remain
required alongside these topology checks. Export/verifier exit0; evidence
is the updated external STEP manifest/report and oracle logs. This verifies
retained fixture manifold incidence and orientation, not G1/G2 continuity
or arbitrary surface/shell intersection exclusion. Full goal remains open.

### 2026-10-02: full-domain STEP boundary-curve preservation

The exporter records every source BRep edge curve and five location samples.
The independent oracle matches source edges to distinct imported OCCT curves,
then compares degree, expanded knots, periodic flag, strictly positive weights
and control displacements. Exact Fraction squared-distance decisions use the
same1e-8 mm tolerance as walls. With the identical rational basis, the convex
combination argument covers the complete curve domain. All192 edges pass:
72 progressive scale/twist,56 open spatial and64 closed planar miter edges.
This includes cap and hole boundary curves as well as wall seams, while cap
loop ownership and pcurve/surface conformance remain separate obligations.

Two temporary-copy negative checks reject altered source edge control and
weight without modifying source STEP files. Export and external oracle exit0;
negative harness exits0 after observing oracle exit1 for both corruptions.
Evidence: updated external manifest/report and
`sweep-external-step-edge-negative.log`. General gate7 coverage, cap trim
conformance and smooth seam qualification remain open.

### 2026-10-02: external STEP boundary-loop ownership and cyclic order

Source manifests record each face's outer and hole edge cycles. The external
oracle maps imported edges back to source curve identities, traverses each
wire using BRepTools_WireExplorer and requires wire closure. Canonical cycles
allow a different starting edge and traversal direction but retain adjacency
order, multiplicities and the distinct outer/hole role. Imported face loop
keys must equal the complete source collection, independently of face order.
All three fixtures pass; this includes the cap annulus loops and closed
inner-shell wall cycles. Export and oracle exit0.

Two temporary-copy negative checks swap outer/hole roles or permute cycle
order without changing curve/surface coefficients. Both correctly yield
exit1 and face_loop_ownership_preserved=false while geometric coefficient
checks still pass. Evidence: `sweep-external-step-loop-negative.log` and
updated external manifest/report. Pcurve/surface agreement, continuous cap
geometry, the broader mode matrix and full sweep guarantees remain open.

### 2026-10-02: imported STEP pcurve/surface and chain-rule samples

External OCCT checks now require SameParameter on every edge and identical
3D-curve/pcurve parameter intervals. For every edge use in every imported
face, seven samples compare S(pcurve(t)) to the spatial curve and compare
Su*du/dt+Sv*dv/dt to its first derivative. Thus384 edge uses are checked at
2688 sites, including cap and hole uses. All three fixtures pass.
Position/derivative errors respectively: progressive1.7763568394002505e-15
and9.48979935823537e-16; open spatial5.329070518200751e-15
and4.385380947269368e-15; closed planar3.552713678800501e-15
and1.5265566588595902e-15. External verifier exits0; updated report and
`sweep-external-step-occt.log` hold the evidence. This qualifies OCCT's
imported pcurve representation at sampled parameters; it is not a continuous
pcurve composition certificate, nor G1/G2 continuity. The complete mode
matrix and remaining eight-gate obligations remain open.

### 2026-10-02: separate whole-boundary native C0 evidence

Combined retained-wall reports now include `declared_boundaries_c0`, exact
C0 boundary pairs and unresolved declarations. For clamped ruled walls with
a common profile basis, equality of endpoint control/weight columns proves
C0 for the full shared parameter range. This sufficient criterion does not
claim G1/G2, regularity or intersection exclusion. The aggregate requires
all declared boundaries to be C0-certified in addition to chart/pair proofs.

Three wall-audit tests pass, including exact common boundaries and a gapped
pair that certifies as separate charts but cannot certify a declared common
seam. Open and cyclic constructor regression assertions were added; their
new native run is recorded separately after completion. Public transport,
smooth seams and the full eight-gate goal remain open.

Constructor qualification completed: all33 progressive-miter regressions pass,
including the open chain's two C0 joins and every retained closed-law join
(including the cyclic seam) despite zero injectivity budget. This verifies
C0 evidence remains distinct from unresolved I. Logs:
`sweep-wall-audit-native.log` and `sweep-wall-audit-miter-native.log`.

### 2026-10-02: read-only bounded smooth-seam jet audit

Added `sweep_seam_audit::inspect` over the existing outward boundary-jet
inspection API. Declarations specify patch IDs, boundaries, requested order1/2,
positive transverse scale and explicit jet tolerance. A seam-count budget
limits inspection; uninspected suffix entries retain no error upper and report
seam-budget-exhausted. Input surfaces are never edited. Reports retain seam
regularity, tangential smoothness, continuous jet deviation upper and the
existing acceptance decision; internal interval work remains governed by the
continuity kernel's bounded policy, not a newly configurable cell budget.

Two native tests pass: planar joined patches qualify at both orders, a changed
second strip fails second jets while retaining first jets, a sharp join fails
first jets, and a singular seam remains unresolved without an error upper.
Budget exhaustion preserves per-declaration evidence. Nonzero jet tolerance
is deliberately named within_jet_budget and never promoted to exact G1/G2.
Evidence: `sweep-seam-audit-native.log`. Actual smooth round/transition sweep
wall integration, exact G1/G2 certificates, public transport and full gate5
remain open.

### 2026-10-02: actual translation corner-sweep jet qualification

Added a constructor-based seam test: actual round/transition paths through
(0,0,0)→(10,0,0)→(10,10,0), with radius/setback2, are swept with a transverse
line profile using surface::sweep. The retained surface is trimmed at actual
path spans, with transverse scaling derived from adjacent authored domain
widths. Transition seams pass regularity and continuous second-order jet
deviation within1e-8. This is qualification at explicit tolerance, not an
exact G2 certificate or a proof for moving-frame sweep variants.

Round seams remain unproved under the existing stronger scaled homogeneous
jet criterion: regularity succeeds but rational weight derivative terms give
wide conservative error bounds. The test retains that refusal instead of
loosening the tolerance; an actual geometric G1 criterion is still required.
All three sweep seam tests pass with these explicit expectations. Evidence:
`sweep-seam-audit-native.log`. Gate5 and full sweep completion remain open.

### 2026-10-02: correlated rational first-jet deviation enclosure

The continuity bound now retains cancellation in the first Euclidean jet
difference. With common boundary homogeneous H and cross-jet difference D,
the numerator is D_xyz*H_w-H_xyz*D_w. The sum over seam basis products
N_i*N_j is enclosed using symmetrized control-pair coefficients before taking
magnitudes; those nonnegative products sum to one. The numerator hull is
divided by the positive boundary weight lower bound squared. All operations
use the existing outward interval arithmetic. The minimum of this bound
and the previous independently valid bound is retained. Additional pair
work is bounded to seams with at most64 controls; larger seams retain the
previous conservative estimate. Second/mixed jet bounds are unchanged.

Actual round translation-sweep seams now pass first-jet deviation within1e-8
with regularity; polynomial transition seams retain their second-jet result.
Three sweep-seam tests and23 continuity regressions pass, including sharp
and singular refusals. Evidence: `sweep-seam-audit-native.log` and
`sweep-correlated-jets-continuity-native.log`. This resolves the previous
round fixture's overly wide first-jet enclosure, not exact G1/G2 or the
full moving-frame sweep family. Public integration and gate5 remain open.

### 2026-10-02: open/closed planar/spatial translation seam matrix

Native corner-sweep qualification now covers eight combinations:
planar/spatial × open/closed × round/transition. Open and sufficiently small
closed paths use the actual translation-sweep constructor followed by span
trims; whole closed transition surfaces exceed its32-V-control limit and
are explicitly refused. Their bounded retained patch coefficient composition
P(u)+Q(v) is separately audited using the actual decomposed path coefficients,
without claiming whole-constructor support. Every ordinary join and cyclic
last→first join is declared with positive authored-domain scale.

Round first-order and transition second-order jet bounds pass at the unchanged
1e-8 tolerance with seam regularity. A zero seam budget yields no upper bounds
for every declaration in every combination. All three sweep-seam tests pass;
evidence: `sweep-seam-audit-native.log`. These results concern translation
patches and explicit tolerance, not exact G1/G2, moving frames or complete
body/viewport integration. Whole closed-transition constructor budgeting
and full gate5 remain open.

### 2026-10-02: public seam-audit transport boundary

Added native `sweep_seam_audit` JSON action and typed `inspectSweepSeams` host
adapter. Public replies retain regularity, tangential smoothness, explicit
jet deviation upper, refusal reasons and the inspected seam count;
`exactG1G2Certified` remains false. Four native seam tests pass, including
transport acceptance, zero-budget null upper and invalid negative scale.
WASM host tests are authored for no input mutation, sharp seam refusal and
unsupported second-order ruled-wall evidence. Geometry WASM packaging has
been launched; public binary/host qualification remains pending until the
build completes and those tests run. Native evidence:
`sweep-seam-audit-native.log`; build log:`sweep-seam-audit-build.log`.

### 2026-10-02: packaged public seam-audit qualification

Geometry WASM build completed with exit0 (wasm-opt11611158→10321453 bytes);
TypeScript check also exits0. New public `inspectSweepSeams` WASM tests verify
accepted regular polynomial seams, unchanged caller inputs, zero-budget null
upper, negative-scale input rejection, sharp seam refusal and unsupported
second-order ruled-wall evidence. Real planar/spatial open round/transition
and closed round seams qualify through public path/sweep/trim/audit adapters
at1e-8; oversized whole closed transition surfaces explicitly refuse.

Nine suites pass136 tests, including progressive/miter/ordinary sweep,
rational BRep, round paths, worker protocol, real-worker boundary and build
coordination. Evidence: `sweep-seam-audit-{build,typecheck,wasm-tests}.log`.
The public flag remains exactG1G2Certified=false; these are bounded jet
qualification reports, not an exact smoothness claim. Rush declaration and
viewport reporting for the new audit, actual moving-frame seams, exact
G1/G2, the full visual matrix and other open gates remain required.

### 2026-10-02: Rush-produced translation surface seam evidence

Added `examples/rush/transition-translation-sweep.r`, a spatial quintic
transition with a fixed transverse profile. A host test compiles this actual
Rush source (and its round-path variant), builds the model and retrieves the
authoritative surface coefficients from the build definitions. It trims at
actual path knot breaks and invokes the public WASM seam audit with authored
domain scaling: first jets for round and second jets for transition, at1e-8.
Both qualify; zero audit budget remains unproved. All four host audit tests
pass, and the direct TypeScript check exits0. Evidence:
`sweep-seam-audit-rush-tests.log` and `sweep-seam-audit-typecheck.log`.
This tests Rush→retained coefficients→public audit; audit declarations are
not yet a Rush language operation or a viewport report. Visual verification
of the new example, moving-frame smoothness and exact G1/G2 remain open.

### 2026-10-02: current admission contract in public authoring guidance

Corrected stale progressive-miter guidance in the public ModelGraph/Rush
authoring prompt and nurbs-core README. The source acceptance condition
requires profile R, wall R, phase resolution, certified frame transport,
sampled error within budget and a nonnull certified retained interpolation
upper within budget. The old real-arithmetic estimate is diagnostic. The
guidance now reflects those exact gates, deferred preview wall regularity
and bounded unresolved refusal, while retaining explicit open full BRep
conversion/cap, embedding, exact G1/G2 and affine/frame/guide extensions.
README also documents the read-only public seam audit's nonzero tolerance
contract and links the full completion matrix. Source checks and diff checks
confirm the correction; no runtime behavior changed or broad completion
claim was introduced. The eight-gate goal remains active and incomplete.

### 2026-10-02: public wall-audit request contract

Added `sweep_wall_audit` to the native JSON dispatch and typed
`inspectSweepWalls` host adapter. The request explicitly carries independent
injectivity, pair-hierarchy and pair-refinement budgets. The response retains
per-chart projections/reasons, C0 boundary evidence and unresolved pair IDs,
while `globalEmbeddingCertified` remains false: shell containment and caps
are separate obligations. Four native wall-audit tests pass, including a
transport regression that independently exhausts chart and hierarchy budgets
and rejects an out-of-range boundary declaration. Direct Vue TypeScript
checking passes. The packed WASM has not yet been rebuilt for this new action;
public binary, worker/Rush/viewport integration and gate4 remain pending.

### 2026-10-02: packed public wall-audit qualification

The geometry WASM rebuild completed successfully (wasm-opt11644276→10352096
bytes). Four new public adapter tests exercise positive chart/pair evidence,
independent zero chart/hierarchy budgets, exact shared boundaries, declared
gaps, overlapping pairs with zero refinement budget, invalid declarations and
budgets, input immutability, and folded charts despite separated patch pairs.
All85 tests pass across six suites: wall audit, seam audit, progressive miter,
worker protocol, real-worker boundary and coordinator. Existing worker cases
are regressions; they do not yet invoke the new wall audit in worker admission.
Direct Vue TypeScript checking also passes. Build/test logs are
`sweep-wall-public-build.log` and `sweep-wall-public-wasm-tests.log`.
The binary/host API is qualified; automatic constructor/worker/Rush/viewport
integration, general adjacent boundaries, caps and shell containment remain
open gate4 obligations.

### 2026-10-02: constructor-derived wall adjacency transport

Added `curve_progressive_miter_wall_audit` and typed
`inspectProgressiveMiterWalls`. Rust validates retained profile correspondence
and derives consecutive/cyclic path-neighbor declarations from station layout
before invoking the combined wall audit. Callers provide all three audit
budgets; they do not construct path adjacency. The report uses the same
serializer as the generic wall action and preserves the false whole-embedding
flag. Five native wall tests pass, including real preview sections through
JSON dispatch, positive chart/pair and C0 evidence for three straight retained
intervals, and rejection of changed weights. Direct Vue TypeScript checking
passes. Evidence: `sweep-miter-wall-transport-native.log`. The current packed
WASM lacks this new constructor action; rebuild/host qualification and
automatic worker admission remain pending. Cross-profile adjacency, caps and
shell containment are still separate unfinished obligations.

### 2026-10-02: packed constructor wall-audit qualification

The rebuilt WASM includes `curve_progressive_miter_wall_audit` (wasm-opt
11649383→10356687 bytes). Two new host tests use actual preview sections: a
three-interval straight sweep proves chart/pair and derived C0 adjacency,
while zero chart budget retains C0 but refuses the aggregate; changed weights
are rejected. A cyclic square with a profile in the initial normal plane
retains all four C0 neighbor declarations including the closing pair; changing
the repeated endpoint section is rejected. The first cyclic test fixture used
a profile outside that plane and was corrected after the constructor properly
refused it. All87 tests now pass across seven suites (constructor wall audit,
wall audit, seam audit, progressive miter, worker protocol, real-worker boundary
and coordinator). TypeScript and diff whitespace checks pass. Logs:
`sweep-miter-wall-public-build.log`, `sweep-miter-wall-public-tests.log`.
This qualifies the constructor API, not automatic worker admission,
cross-profile boundaries, caps, shell containment or complete gate4.

### 2026-10-02: automatic retained wall evidence in body construction

Both synchronous and streamed progressive-miter B-rep construction now invoke
the constructor wall audit after retained E/R admission and before building
topology. `ProgressiveMiterBrepBody.wallAudit` is retained in the Rush
construction report, including the streamed worker build path. Optional TS
`wallAuditBudgets` independently configure all three budgets; current defaults
are1000 cells/visits each, clearance0 and distance tolerance0.001mm. These
budgets are not yet authored Rush arguments. Cancellation is checked before
and after the synchronous native audit in the streaming path. The report is
evidence only: unresolved audits do not yet reject body construction, and
`globalEmbeddingCertified` remains false because general boundaries, caps and
shell containment are unfinished. Full I admission must still be implemented.

83 tests pass across six suites including real-worker preview, cancellation
and recovery. A subsequent three-suite17-test run adds synchronous/streamed
body report equality with all audit budgets zero, verifying explicit
unresolved evidence is retained. Rush construction qualification asserts C0
evidence and the false whole-embedding flag. Direct Vue TypeScript and
`git diff --check` pass. Logs: `sweep-wall-construction-tests.log` and
`sweep-wall-construction-focused-tests.log`. Gate4 and the overall goal remain
open; viewport presentation and full Rush budget authoring remain pending.

### 2026-10-02: shared tensor-boundary coordinate-plane certificate

The native pair audit now recognizes complete clamped tensor boundaries on
either U or V, including different normal parameter axes across the pair.
Exact degree/knot/periodic/control/weight equality proves C0 for the boundary
curve; strictly opposite non-boundary control hulls around a common coordinate
plane independently prove boundary-only intersection. This supports rational
higher-degree profile-side boundaries beyond the prior degreeV1 station-side
criterion. No sampled equality or rounded orientation promotes the result.
The wall C0 audit reuses the same exact boundary correspondence check.

21 native `sweep_` tests pass. The new test covers rational degree2×2 walls,
axis transposition, same-side folding, changed boundary weights and refusal of
an unclamped normal edge. Existing seam and aggregate wall regressions pass.
Evidence: `sweep-tensor-boundary-native.log`. This native extension has not yet
been packed into WASM. General nonplanar boundaries, automatic cross-profile
loop ownership, caps and shell containment remain open gate4 obligations.

### 2026-10-02: explicit retained profile-loop ownership

Added optional `loopSizes` to constructor wall auditing, backed by native
`inspect_wall_geometry_with_loops`. Partition validation requires1..16
nonempty loops covering every profile. Each retained section must have exact
clamped endpoint closure around each loop; a single validated periodic curve
is accepted as a closed loop. Consecutive and closing profile-neighbor pairs
are declared only within that explicit loop, independently of station-side
adjacency. These declarations still require separate C0 and boundary-only
certificates; they do not grant containment or arbitrary contact permission.
Both body builders pass their actual loop partition to the audit.

21 native sweep tests pass. Transport qualification now includes a four-segment
square with all four profile-side C0 declarations, invalid/open partitions,
and two separate four-segment loops with no cross-loop C0 ownership. Vue
TypeScript and diff whitespace checks pass. Evidence:
`sweep-loop-ownership-native.log`. The packed binary still lacks this extension
and the preceding tensor-boundary extension; public/worker qualification
requires rebuilding it. General boundary contacts, multi-boundary patch pairs,
profile decomposition, caps and shell containment remain open.

### 2026-10-02: packaged tensor-boundary and loop-ownership qualification

The WASM build completed with both native extensions. New public adapter tests
qualify rational degree2×2 coordinate-plane shared boundaries (including a
parameter-axis swap), same-side folding refusal, changed boundary-weight C0
refusal, two closed four-segment loops with eight owned profile-side boundaries
and no cross-loop declarations, and invalid/open partition refusal. Both body
construction paths now pass actual contour sizes through the packaged
constructor action; real-worker build/cancellation/recovery regressions pass.
All90 tests pass across seven suites; direct Vue TypeScript and diff checks
pass. Logs: `sweep-loop-ownership-build.log` and
`sweep-loop-ownership-wasm-tests.log`. The new contracts are documented in
the native README. Global shell containment, caps, general/nonplanar contacts,
multi-boundary pairs, retained profile decomposition and strict full-I
admission remain open. No whole-embedding flag is promoted.

### 2026-10-02: oblique shared tensor-boundary certificate

Added a checked-dyadic separating-plane certificate for exact matching
clamped tensor boundaries on either parameter axis. Candidate planes are
proposed from boundary controls, finite represented auxiliary directions and
non-boundary controls; proposals are never evidence. Every boundary control
must have exact zero orientation and every non-boundary control must have a
strict consistent sign, opposite across the pair. Overflow or unresolved
orientation rejects that candidate. This extends profile-side corner evidence
beyond coordinate planes without using sampled intersections.

21 native sweep tests pass. Rational degree2×2 perpendicular wall tests prove
compatibility where the coordinate-plane criterion fails; moving a control
onto the other chart defeats the certificate. The actual four-segment miter
profile now proves all pair compatibility, independently retaining aggregate
false when the separate100-cell chart budget exhausts. An initial test
incorrectly expected aggregate success and was corrected to assert the actual
independent pair and chart outcomes. Evidence:
`sweep-oblique-tensor-native.log`. Diff whitespace checking passes. The packed
WASM has not yet been rebuilt for this extension. General nonplanar/multiple
boundary contacts, caps and shell containment remain unfinished.

### 2026-10-02: constant-coordinate projection pruning and oblique WASM evidence

Whole-chart monotonicity now skips projections containing a globally constant
represented coordinate: positive rational basis combinations keep that
coordinate exactly constant, so such a projection cannot be strictly
monotone. This removes wasteful subdivision of rounding intervals around
zero without relaxing the certificate. A coordinate-plane patch certifies
with one cell; budget0 remains unresolved. Five monotonicity tests pass.
The folded-chart regression now retains the correct refusal for the folded
chart while allowing the subsequent independent regular chart to certify,
rather than asserting the previous wasted-budget cell count.

21 native sweep tests pass; the square-loop constructor now proves both chart
and pair evidence within its100-cell test budget. The WASM rebuild completed
(wasm-opt11669213→10374972 bytes).91 tests pass across seven public/worker
suites, including perpendicular rational degree2×2 profile walls, moved-control
refusal and positive retained chart/pair evidence for two disjoint square
contours. TypeScript and diff checks pass. Logs:
`sweep-projection-policy-native.log`, `sweep-oblique-tensor-native.log`,
`sweep-oblique-tensor-build.log`, `sweep-oblique-tensor-wasm-tests.log`.
These results still do not prove caps or containment; disjoint contours are
not automatically a valid outer/inner shell. Full gate4 remains open.

### 2026-10-02: continuous planar contour-domain foundation

Added native `sweep_contour_audit::inspect` for explicit outer/inner rational
contours lying exactly in a coordinate plane. Dropping that constant coordinate
is exact. The audit combines existing whole-curve simplicity certificates,
positive continuous inter-loop distance lower bounds and homotopy/winding
classification. With simple connected loops and disjoint boundaries, a proved
inside/outside winding query extends to the complete loop. Outer ownership
requires every hole inside loop0; holes must also lie outside one another.
The result is only `cap_domain_certified`, never a swept-shell or cap-surface
certificate. Explicit shared pair/cell budgets return unresolved on exhaustion;
current internal simplicity/refinement policies additionally cap each call at
256 cells. Whole control-hull boxes avoid unnecessary distance refinement.

Two native tests pass (0.01s after pruning/refinement policy); positive rational
square contours with two holes are proved. Outside/nested/touching holes,
nonplanar data, crossed contours and zero/tiny budgets never promote success.
The original uncapped refinement run completed successfully but was slow;
no successful result is inferred from a timeout. Evidence:
`sweep-contour-native.log`. The module is formatted and diff checks pass.
General oblique planes, retained rational decomposition, public transport,
constructor/worker integration, actual cap geometry and global shell
containment remain unfinished. Gate4 remains open.

### 2026-10-02: public continuous contour-domain audit

Added native JSON `sweep_contour_audit` and typed host `inspectSweepContours`
with explicit shared pair/cell budgets. The response retains plane axis, work
counts, refusal reason and separate domain/geometry/embedding flags. Three
native contour tests pass, including JSON transport positive evidence, both
zero budgets and invalid tolerance. The geometry WASM rebuild completed
(wasm-opt11676489→10381531 bytes).89 tests pass across seven suites, including
the new contour adapter, miter wall/audit/body cases, protocol, real worker and
coordinator. Public contour cases cover input immutability, positive two-hole
ownership, outside/nested/touching holes, zero budgets and nonplanar input.
Vue TypeScript and diff checks pass. Logs:
`sweep-contour-transport-native.log`, `sweep-contour-public-build.log`,
`sweep-contour-public-tests.log`. Native README documents the scope.
The new audit is not yet automatically applied to retained sweep sections;
general rational decomposition/oblique planes, actual cap geometry and
global shell containment remain open gate4 obligations.

### 2026-10-02: exact segmented contour coefficients

Contour-domain auditing now accepts fully segmented clamped rational profiles,
including circle primitives. Every interior knot must have degree multiplicity;
Bezier spans are formed solely by cloning existing control/weight slices and
repeating their original domain endpoints. No knot insertion, reevaluation or
rounded reconstruction transfers the certificate. Periodic and unsegmented
representations remain unresolved until an appropriate extraction certificate
exists. The resulting contour segment budget is256 across all loops.

Four native contour tests pass (0.10s), including an actual outer/inner circle
pair, exact quarter-arc coefficient equality and refusal of a validated
unsegmented knot representation. Diff checks pass. Evidence:
`sweep-contour-segments-native.log`. The packaged WASM still contains the
previous contour audit; rebuild and public circle qualification remain
pending. Retained section integration, oblique planes, cap geometry and
global shell containment remain open.

### 2026-10-02: retained miter cap-domain evidence

The rebuilt WASM includes exact segmented contour extraction. Both synchronous
and streamed progressive-miter body constructors audit their actual retained
first/last contour sections and expose `capDomains` in the Rush construction
report. Closed paths have `capDomains:null` because no caps are constructed.
Optional `contourAuditBudgets` supplies explicit tolerance/pair/cell budgets
per cap; current defaults are0.001mm/1000/1000. Streaming checks cancellation
before, between and after the two native calls. Domain evidence does not
prove cap geometry, orientation or whole-shell containment and unresolved
domains do not yet govern full-I admission. Existing B-rep orientation
validation remains independent.

91 tests pass across seven suites, including real-worker cancellation/recovery,
actual circle primitives, successful retained hollow cap domains at explicit
10000-cell budgets, sync/stream equality, zero-cell refusal and closed-path
null reports. The initial body fixture was corrected to reverse hole
orientation; the1000-cell policy correctly returned budget exhaustion for
these holes, and positive qualification explicitly uses10000 rather than
relaxing that refusal. Four native contour tests, Vue TypeScript and diff
checks pass. Logs: `sweep-cap-domains-build.log`, `sweep-cap-domains-tests.log`,
`sweep-contour-segments-native.log`. Full cap geometry, oblique planes, Rush
budget authoring, global containment and the remaining gates stay open.

### 2026-10-02: continuous retained cap-boundary composition bound

Added native `sweep_cap_boundary::inspect` for a retained unit-domain
nonrational bilinear cap surface, rational UV curve and spatial boundary
sharing the same positive rational basis. With R_i summing to one, expanding
S(UV(t))-world(t) in symmetric R_iR_j products yields a convex combination
of outward coefficient error boxes. This accounts for the represented mixed
bilinear term as well as rounded surface/UV controls. UV controls must lie
in[0,1]², guaranteeing the whole rational curve remains in the surface domain.
All n² products must fit the explicit budget; otherwise no upper bound is
returned. Numeric enclosure failure also remains unresolved. The certificate
proves only whole-boundary deviation within tolerance, not cap-domain validity,
regularity, interior coverage or global embedding.

Native qualification passes for a rational quadratic boundary, full-product
budget versus exhausted suffix, moved UV control, warped bilinear corner and
incompatible weights. The source is formatted and diff checks pass. Evidence:
`sweep-cap-boundary-native.log`. Integration into actual B-rep cap construction,
public/WASM reporting and full cap geometry remain pending.

### 2026-10-02: stored B-rep cap boundary admission

Actual rational-loft cap construction now requires the continuous cap-boundary
composition certificate for every retained world/UV curve after orientation
changes. The construction policy uses tolerance1e-9mm and a shared100000
basis-product budget per cap. Unresolved/over-budget boundaries refuse the
cap; coplanarity and trim-region checks remain independent. A private
budgeted helper supports direct refusal qualification while preserving the
existing public constructor API. This does not promote cap interior regularity,
whole domain/surface coverage, or global shell embedding.

One private constructor test proves positive admission and refusal for zero
product budget and unproved zero tolerance. All16 rational-section-loft
integration tests pass, including rational hollow/curved/natural/contact
lofts and consistent shell senses. A rotated polygon at9e5mm passes the
certificate within existing coordinate limits. An initial negative fixture
exceeded the1e6mm input bound; after correcting that fixture, the valid
large-coordinate cap certified successfully, so the test now asserts that
actual outcome. Logs: `sweep-cap-construction-refusal.log`,
`sweep-cap-construction-native.log`. Diff checks pass. The packed WASM still
contains the previous B-rep constructor; rebuild, public/worker regression
and full cap geometry remain pending.

### 2026-10-02: retained cap chart admission and packaged qualification

Actual B-rep cap construction now also requires whole-domain surface
regularity and global complete-chart contraction injectivity for the stored
bilinear cap after orientation changes. These certificates use current
policies1000 regularity cells and1 original injectivity span; zero budgets
and collapsed charts refuse in direct constructor tests. Cap boundary
composition admission remains an independent1e-9mm/100000-product check.
The native unit refusal test and all16 rational-loft integration tests pass.

The geometry WASM rebuild completed (wasm-opt11686086→10390329 bytes).109
tests pass across eight suites, covering rational sweep/loft caps, authored
frame/contact bodies, internal STEP round-trip, progressive miter reports,
real-worker cancellation/recovery and coordinator/protocol behavior. Vue
TypeScript and diff checks pass. All three existing sweep STEP fixtures were
regenerated through the current packaged constructors and passed independent
OCCT verification. Logs: `sweep-cap-chart-native.log`,
`sweep-cap-chart-loft-native.log`, `sweep-cap-chart-build.log`,
`sweep-cap-chart-wasm-tests.log`, `sweep-cap-chart-step-export.log`,
`sweep-cap-chart-step-occt.log`.
These checks do not promote whole sweep embedding or full capGeometry flags:
cap/wall intersection ownership, author-to-retained general decomposition
error, full shell containment and public budget/evidence presentation remain
open alongside the other unfinished goal gates.

### 2026-10-02: public stored cap-boundary evidence

Added JSON `sweep_cap_boundary_audit` and typed `inspectSweepCapBoundary`
with explicit tolerance/product budget. The report preserves whole-curve
error upper, product count, reason and conservative full-geometry/embedding
flags. Native JSON qualification passes, including zero budget returning null
upper. The geometry WASM build completed and104 tests pass across eight
suites. A new public test traverses all16 actual cap coedges of a retained
rational hollow loft, aligns world edge orientation, proves each stored
S(UV(t)) boundary within1e-9mm, refuses incomplete9-product coverage and
moved UV controls, and preserves input immutability. Existing rational sweep
and real-worker regressions pass. Vue TypeScript and diff checks pass.
Logs: `sweep-cap-boundary-transport-native.log`,
`sweep-cap-boundary-public-build.log`, `sweep-cap-boundary-public-tests.log`.
The native README documents the public contract. General cap/wall contact
ownership, shell containment, full evidence presentation and the remaining
goal gates remain open.

### 2026-10-02: cap/wall coordinate-plane interior exclusion

Added native `sweep_cap_wall::inspect`. A cap whose represented positive
rational control hull lies in a coordinate plane permits exact sign tests on
wall controls. A wall strictly on one side is fully separated. A clamped
declared endpoint strip in the plane, with all remaining controls strictly
on one side, permits contact only on that declared parameter boundary.
Positive rational basis combinations prove this over the whole wall, not
just stations. This is interior exclusion only: it does not prove the
boundary belongs to the cap trim, that a contact occurs, or that it is allowed.
Those ownership/correspondence obligations remain separate.

The bounded report preserves separated, boundary-restricted and unresolved
wall IDs; exhaustion explicitly retains all remaining walls. Native tests
pass for nonuniform rational weights, positive separation/boundary restriction,
missing declarations, crossing/folded walls, zero/suffix budgets and refusal
of a non-coordinate-plane cap. Evidence: `sweep-cap-wall-native.log`. Source
formatting and diff checks pass. No public/WASM or constructor integration
yet; oblique caps, general contacts and full shell containment remain open.

### 2026-10-02: public cap/wall interior-exclusion evidence

Added JSON `sweep_cap_wall_audit` and typed `inspectSweepCapWalls`. Native
transport qualification covers positive evidence, zero budget preserving
all unresolved IDs and invalid declaration refusal. The WASM build completed
(wasm-opt11695896→10398929 bytes).102 tests pass across eight suites, including
a public test on an actual two-interval hollow loft: each cap excludes
interiors of all16 retained walls, retaining8 separated and8 boundary-
restricted walls. A two-wall budget preserves all14 uninspected suffix IDs,
missing declarations refuse the aggregate, and input data is unchanged.
Existing cap-boundary, rational sweep and real-worker regressions pass.
Vue TypeScript and diff checks pass. Logs:
`sweep-cap-wall-transport-native.log`, `sweep-cap-wall-public-build.log`,
`sweep-cap-wall-public-tests.log`. Native README documents the public contract.
Boundary ownership, automatic body integration, oblique caps and full
shell containment remain unfinished; no global embedding flag is promoted.

### 2026-10-02: automatic cap/wall contact evidence from B-rep incidence

Added host `inspectSweepCapContacts`, using explicit cap face IDs and shared
edge IDs to derive candidate wall parameter boundaries from stored coedge UV
data. Native full-wall exclusion still independently validates those
declarations. Ambiguous/missing boundary labels remain undeclared; incidence
does not promote geometric ownership. Report entries retain cap/wall face IDs,
declarations and complete native evidence. Duplicate/invalid cap selections
are rejected, and cancellation is checked before/after each native call.

Both progressive-miter body builders now invoke this audit on actual retained
B-rep walls after construction and preserve `capContacts` in the Rush
construction report. Closed paths report null. Optional `capWallMaxWalls`
configures the per-cap budget (default1024). Reports remain diagnostic until
full contact ownership and shell containment admission are completed.

83 tests pass across six suites, covering automatic incidence-derived
boundaries on a hollow loft, uninspected suffix retention, invalid selection,
cancellation, actual hollow miter cap evidence, sync/stream equality, zero
contact budget, closed-null reports and real-worker preview/cancel/recovery.
Vue TypeScript and diff checks pass. Evidence:
`sweep-cap-contact-integration-tests.log`. No Rust/WASM change was required;
the previously qualified public action is used. General ownership, oblique
contacts, shell containment and strict full-I admission remain open.

### Cap coedge incidence and continuous agreement evidence

The progressive miter cap-contact report now retains each cap coedge's edge ID,
wire/index, incident wall faces, shell-oriented pairing result, and native
whole-curve cap/world-edge error bound. `capEdgeAgreementWithinBudget` requires
all cap coedges to pass the outward bound; `pairedOppositeCoedges` requires exactly
one cap and one wall use of every edge, each face occurring once in the same
shell, with opposite traversal after applying shell face reversal. Neither flag
promotes `boundaryOwnershipCertified` or `globalEmbeddingCertified`: wall-pcurve
agreement, exact contact ownership, and whole-body containment remain separate
obligations. The default cap error tolerance is 1e-9 mm; the 100000-product budget
is shared across the coedges of each cap. A failed/budget-exhausted edge remains
in the report, rather than being omitted. This is diagnostic evidence, not yet
an additional body-admission gate.

Regression fixtures use actual two-interval hollow section-loft topology and
cover altered cap pcurves, coedge reversal, shell-face reversal, duplicated edge
uses, insufficient shared product budget, invalid budget, cancellation and input
immutability. The TypeScript adapter indexes incidence once; all continuous
geometric bounds still execute in the native kernel.

### Both sides of cap-edge composition

The native `sweep_coedge_agreement_audit` transport exposes the existing outward
whole-domain composition verifier for general rational wall charts. The cap
contact report now retains the wall face, wire and coedge indices and each wall
composition result alongside the cap-boundary result. World-edge reversal follows
the local coedge; shell reversal is used only for the opposite-use topology
check. `wallEdgeAgreementWithinTolerance` requires exactly one incident wall
coedge per cap edge and a positive continuous result for every such coedge.
The default 1000-cell budget is shared across a cap's incident wall coedges;
zero remaining cells yields explicit unresolved results. Invalid budgets and
nonpositive tolerance are rejected. This does not promote exact ownership or
body admission. Exact contact and shell containment still need proof.

Qualification: native composition suite 12/12 passed
(`sweep-coedge-native.log`); packaged WASM build exited 0
(`sweep-coedge-build.log`); seven public/worker/coordinator suites passed 84/84
(`sweep-wall-coedge-evidence-tests.log`). Actual hollow-loft wall compositions
passed; altered wall pcurves, zero cells and invalid cell/tolerance inputs were
covered. TypeScript checking and `git diff --check` passed. No global embedding
claim follows from these checks.

### Exact coedge composition as separate evidence

The Rust/WASM API now exposes the existing exact rational Bezier composition
predicate, preserving stored binary64 coefficients and normalized traversal.
Each cap and incident wall coedge retains its exact decision in the cap-contact
report. `capAndWallExactIdentityCertified` requires every cap and wall use to
have a positive exact identity decision; it is separate from the tolerance and
shell-oriented pairing flags. Per-cap exact work is shared (default 1000000),
and insufficient work leaves unresolved evidence. Exact equality of these
compositions does not by itself certify boundary coverage, injectivity, or
allowed contact; global embedding and boundary ownership remain false.

The distinction matters for projected cap pcurves: an outward error bound may
pass even when the stored projection does not satisfy an exact identity. Such
geometry must not silently inherit an exact C0/ownership claim.

Qualification: 12 native composition tests passed; packaged WASM build exited
0; seven public/worker/coordinator suites passed 85 tests. Additional focused
assertions passed for the actual hollow-loft fixture: all incident wall coedge
compositions are exactly equal, while some stored cap coedge compositions are
provably different despite passing 1e-9 mm continuous deviation. Consequently
both caps keep `capAndWallExactIdentityCertified=false`. This is evidence of a
cap projection rounding obligation, not an allowed-contact proof. The next
construction step must address that gap rather than promote the tolerance flag.
Logs: `sweep-coedge-exact-native.log`, `sweep-coedge-exact-build.log`,
`sweep-coedge-exact-tests.log`, `sweep-cap-exact-status-tests.log`.
TypeScript and diff checks passed.

### Natural coordinate-plane cap charts

Coordinate-plane caps can now retain the two original coordinate components as
UV controls and use their actual bounds as the surface knot domains. Surface
corners retain the same coordinates, with no inverse frame or unit-domain UV
normalization. Negative trim winding swaps the two coordinates and transposes
the chart, preserving the stored coefficients. Before choosing this branch,
the constructor requires cap regularity, chart injectivity, continuous boundary
error admission and exact coedge composition for every boundary under a shared
1000000-work budget. Unsupported/unresolved exact decisions fall back to the
existing bounded projection constructor; this fallback does not gain an exact
claim. Other cap planes still use the general projected chart.

The bilinear cap-error verifier now supports arbitrary clamped knot domains.
It normalizes UV controls through outward interval subtraction/division before
forming the complete correlated rational product bound. Unit-domain support
and product-budget refusal remain intact. Round-trip regressions compare
positions at corresponding fractions of the actual knot domains, including
orientation reversal about the domain endpoint sum. They no longer assume every
stored cap has a [0,1] chart.

Qualification: native cap certificate tests passed 2/2 (all coordinate axes and
both senses plus budget refusals), rational section-loft regressions passed
16/16, and the generalized cap bound test passed. Packaged WASM build exited 0;
eight public/worker/coordinator suites passed 103/103; TypeScript and diff checks
passed. The actual public hollow-loft fixture has one natural-coordinate cap
with positive exact cap/wall composition evidence and one fallback projected
cap retaining an unproved exact identity. No claim of universal coordinate-cap
coverage follows. Independent OCCT verification passed all three regenerated
STEP fixtures after the chart change. Logs: `sweep-coordinate-cap-unit.log`,
`sweep-coordinate-cap-native.log`, `sweep-natural-cap-bound-native.log`,
`sweep-coordinate-cap-build.log`, `sweep-coordinate-cap-tests.log`,
`sweep-coordinate-cap-step-export.log`, `sweep-coordinate-cap-step-occt.log`.

### Exact constant-coordinate composition reduction

A public diagnostic reproduced the translated hollow cap's refusal: its first
seven world/UV compositions were exactly equal, but the eighth exhausted the
unchanged shared 1000000-work budget (`sweep-cap-budget-diagnosis.log`). No
geometric difference was proved. The exact CAD predicate now checks whether all
source curve and surface poles share exactly the same coordinate. Positive
rational bases preserve that constant independently of their weights; that
coordinate can bypass polynomial composition and cross multiplication. The
comparison uses source expansions and charges the predicate budget. A single
unequal pole keeps the full calculation for that coordinate. This reduction
uses no tolerance and does not weaken unsupported/resource-limit handling.

Native regressions include independent nonuniform rational weights, a one-ULP
curve or surface coordinate change, zero work, every coordinate plane and both
orientations, and an actual translated hollow cap under the original shared
budget. Other axes still require the complete exact composition predicate.

Qualification after reduction: both caps in the public two-interval hollow loft
now use natural coordinate charts and positively certify exact composition on
both cap and incident wall coedges under the unchanged default budgets. The
translated cap's eight native composition proofs consume 849947 work units out
of 1000000 (`sweep-cap-budget-after.log`), replacing the previous exhausted last
edge. Predicate tests passed 3/3, composition tests 13/13, cap certificate tests
3/3 and rational section-loft regressions 16/16. Packaged WASM exited 0; eight
public/worker/coordinator suites passed 103/103; TypeScript and diff checks
passed. All three regenerated STEP fixtures passed independent OCCT verification.
Logs: `sweep-constant-axis-predicate-native.log`,
`sweep-constant-axis-agreement-native.log`, `sweep-translated-cap-native.log`,
`sweep-constant-axis-loft-native.log`, `sweep-constant-axis-build.log`,
`sweep-constant-axis-tests.log`, `sweep-constant-axis-step-export.log`,
`sweep-constant-axis-step-occt.log`. Exact composition still does not by itself
prove full boundary coverage, allowed cap/wall contact or global embedding.

### Complete image coverage of declared wall boundaries

The native boundary coverage audit verifies the missing full-side image
obligation. It validates continuous positive-weight rational UV curves, clamped
endpoints and fixed-coordinate membership on the actual surface knot domains.
Control hull inclusion and endpoint attainment give whole-side image coverage
by continuity, without sampled points or monotonicity assumptions. The contact
adapter stores this evidence per incident wall coedge and summarizes it as
`wallBoundariesCovered`. Boundary labels are now derived from actual wall knot
domains rather than literal 0/1. A partial traversal or a displaced control
stays unproved even when its edge ID matches a cap edge. Injectivity, boundary
ownership and global embedding are not promoted by this evidence alone.

Qualification: native cap/wall suite passed 2/2; packaged WASM build exited 0;
eight public/worker/coordinator suites passed 104/104 (`sweep-boundary-coverage-tests.log`).
Both actual hollow-loft caps retain positive full wall-boundary coverage along
with the existing exact cap/wall composition evidence. Shortening a wall UV
traversal prevents full-side coverage despite unchanged edge IDs. A deliberately
backtracking degree-five traversal still covers the side and explicitly retains
`injectivityCertified=false`. Nonunit domains and both traversal directions are
covered. TypeScript and diff checks passed. This is a coverage prerequisite,
not yet the combined native proof of allowed cap/wall contact.

### Combined native allowed cap/wall contact certificate

`brep-core::sweep_cap_contacts::inspect` and
`brep_sweep_cap_contacts_audit` recompute prerequisites from the retained model.
The model must validate; cap selection and budgets are bounded. The cap chart
must prove whole-domain regularity/injectivity and its actual UV loops must pass
trim-region simplicity, orientation and hole ownership. Every cap coedge must
have exactly two uses in one shell with opposite shell-oriented traversal and
must exactly compose to its stored world edge. Each remaining wall chart is
then either wholly separated from the cap's coordinate plane without shared
incidence, or restricted to one complete side whose pcurve exactly composes to
the same edge and covers that entire side. This proves that every contact with
the trimmed cap is confined to the authored shared edge. Wall/wall embedding,
cap/cap contacts and shell containment remain separate obligations.

The report preserves global wall face IDs for separated, allowed and unresolved
pairs. Shared exact work and wall budgets cannot silently omit suffixes.
`capCertified` describes the cap prerequisites; `allCapWallContactsCertified`
describes only this cap versus the selected whole wall-chart scope. The global
embedding flag stays false. Invalid topology is refused; budget exhaustion or
unproved geometry returns unresolved evidence. The progressive miter contact
report includes this combined result as `native`, while retaining individual
error, exact-identity and boundary-coverage diagnostics. It remains diagnostic
until full body embedding admission is implemented.

Qualification: native actual-hollow-loft test passed, including both caps,
partial wall budget, zero exact/chart budgets, duplicate cap IDs, input
immutability and a one-ULP cap-pcurve change that still passes ordinary model
validation but loses the exact certificate. The corrected WASM build exited 0;
eight public/worker/coordinator suites passed 104/104. Public tests confirm eight
allowed and eight separated walls per cap, unresolved budget suffixes and typed
refusal of malformed B-rep geometry/orientation. TypeScript and diff checks
passed. Logs: `sweep-combined-cap-native.log`,
`sweep-combined-cap-build-fixed.log`, `sweep-combined-cap-tests.log`.

### Cap/cap chart pair evidence

Progressive miter bodies now retain independent `capPairs` evidence for the two
actual endpoint cap faces, in both synchronous and streamed construction and
Rush construction reports. Closed bodies have no caps and retain null evidence.
The existing native chart/pair audit covers full retained cap charts, which are
conservative supersets of their trimmed regions. It uses outward separation
bounds and bounded pair refinement, with no declared common boundaries. Positive
separation of whole charts proves separation of trimmed caps; overlap of whole
charts remains unresolved even when trimming might separate them. Cap trim-aware
pair refinement remains an obligation for those cases.

Injectivity, pair traversal and pair refinement budgets are independent and
configurable through `capPairAuditBudgets`; defaults are clearance 0,
distanceTolerance .001 and 1000 for each budget. Unresolved chart and pair IDs
are mapped back to actual B-rep cap face IDs. Native `pairs` counts hierarchy
work, whereas `separatedPairs` counts successful chart pairs; these must not be
conflated. This report remains diagnostic and keeps global embedding false.

Qualification: seven public/worker/coordinator suites passed 87/87
(`sweep-cap-pair-integration-tests.log`). Cases include real nonunit hollow-loft
caps, coincident charts, zero pair/injectivity budgets, ID preservation,
cancellation, input immutability, sync/stream equality, an open Rush construction
report and null cap evidence for closed Rush bodies. TypeScript and diff checks
passed. The existing packaged native API was reused; no new WASM binary was
required for this integration.

### Retained wall injectivity and supporting-plane vertex contacts

The native linear-projection monotonicity audit proves a single fixed oblique
projection over the whole chart using outward Jacobian bounds. The progressive
miter report now retains `retainedWallCharts` for actual decomposed B-rep walls,
with face IDs, shared cell budgets and unresolved reasons. The hollow two-span
fixture proves all 16 wall charts with a 10000-cell budget; the default is 1000.
The report remains diagnostic and global embedding remains false. Qualification:
`sweep-oblique-injectivity-native.log` (2 tests), packaged build exited zero,
`sweep-oblique-injectivity-tests.log` (89 tests), TypeScript and whitespace checks.

The existing exact hull contact certificate now additionally restricts poles
to simultaneous exact supporting planes. Positive rational convex combinations
attaining an extremal coordinate cannot use off-plane poles. This can reduce a
line-shaped AABB overlap between diagonal retained wall panels to their exact
authored common vertex. Coordinate coincidence with independent vertex/edge IDs
is refused, as is a one-ULP change in the shared vertex position. This certificate
still requires the caller's exact coedge agreement, simple trims and whole-chart
injectivity prerequisites. It is not a standalone body embedding certificate.

Native qualification: 34 boundary-related tests passed, including actual hollow
loft walls with explicitly authored exact rational quadrant endpoints, broken
ownership, one-ULP mutation and unchanged input. Log:
`sweep-retained-support-boundary-native.log`. Ordinary trig-generated circle
profiles did not yield vertex-only certificates in this experiment; general
contact refinement remains required. This change has not yet been rebuilt into
the packaged WASM. Global pair classification, containment and strict admission
remain open.

### Exact-use stage diagnosis for external STEP fixtures

The exporter now retains individual native exact-coedge results and cumulative
work alongside the combined embedding and volume reports. A shared 1000000-work
budget is used in face order; unresolved suffixes are preserved explicitly.
The scale/twist fixture certifies 129 of 144 uses, consuming 999996 work; the
first unresolved use is on cap face 32. The spatial fixture certifies 99 of
112 uses, consuming 999996 work; its first unresolved use is on cap face 24.
Neither fixture reports a different or unsupported use. This is evidence of
budget exhaustion, not evidence that the remaining identities are equal.
The closed planar fixture certifies all 128 uses with 818414 work.

The combined scale/twist report proves trims and all face injectivity, but
retains 96 unresolved contact pairs. The spatial report additionally retains
unresolved face IDs 16 through 23 and an unproved endpoint cap. These are
separate obligations; increasing an exact-identity budget alone cannot prove
volume validity. The next exact-boundary task is to reduce composition work
or support a justified larger shared budget, followed by contact refinement
and spatial wall/cap qualification.

Evidence: external-step-stages/manifest.json and sweep-step-stages-export.log.
The independent OCCT verifier exited zero for all three exported fixtures
(sweep-step-stages-occt.log); native unproved results remain unproved.

### Reduced exact power-polynomial work

The exact rational Bezier composition predicate now removes only exactly zero
trailing power coefficients after Bernstein conversion and multiplication.
This avoids propagating artificial degrees for constant weights and coordinates.
Coefficient comparison explicitly treats omitted coefficients as exact zero
and covers the larger degree, preventing a shortened zip from missing a
nonzero leading term. No tolerance or rounded equality is introduced.

Native qualification: cad-predicates full library tests, curve/surface agreement
tests and B-rep boundary agreement tests passed. A new regression checks degree
elevation equality and differences with either side having a larger true degree.
Logs: sweep-exact-polynomial-trim-tests.log and
sweep-exact-polynomial-trim-boundary-tests.log. Packaged WASM has not yet been
rebuilt; fixture work reduction and resulting admission remain unverified.

### Independent exact-use retries distinguish exhaustion from mismatch

The STEP exporter now retries unresolved coedge identities independently with
a 1000000-work budget. These diagnostics do not replace the shared-budget
embedding/volume certificate. All 14 remaining scale/twist uses are equal
(1627728 additional independent work). The spatial fixture has four equal
and eight different retries (905091 work); therefore its endpoint-cap
boundary construction has a real exact identity mismatch, not merely a
shared-budget problem. The next fix must inspect the cap/world-edge/pcurve
construction rather than raise a budget or admit approximate equality.
Evidence: external-step-independent-exact/manifest.json.

The exact composition predicate additionally reuses successive exact powers
across Bernstein basis terms instead of recomputing prefixes. All 16 native
cad-predicates tests pass (sweep-exact-power-reuse-tests.log). This second
optimization has not yet been packaged into WASM; no fixture gain is claimed.

### Spatial endpoint planarity after natural-coordinate cap candidate

The natural-coordinate graph cap candidate passes four native cap regressions,
including exact oblique edges in both senses. Packaged WASM build exited zero;
12 public embedding/progressive tests passed. However the original spatial
fixture still has eight different endpoint coedge identities. Independent
OCCT verification of all three fixtures exits zero; it does not establish
exact binary64 identities.

Exact Fraction arithmetic on four actual endpoint world-edge control points
proves a nonzero tetrahedral determinant (~1.9860273225978107e-16). Their
stored binary64 coordinates are not coplanar. The expected y+z/2 constant
also varies by 1.7763568394002505e-15. Consequently a replacement planar cap
alone cannot exactly contain every existing endpoint edge. The next change
must preserve a common exact section representation when generating walls
and caps, or deliberately account for a geometric correction with a proven
continuous bound; increasing tolerances cannot establish exact identity.

Evidence: external-step-oblique-cap/manifest.json,
sweep-spatial-endpoint-exact-planarity.json, sweep-oblique-cap-public-tests.log,
sweep-oblique-cap-occt.log and sweep-natural-oblique-cap-build.log.
The closed planar fixture remains volume-certified with all 128 exact uses
and 793004 exact work. The scale/twist fixture still requires shared-budget
work reduction and contact qualification. The full objective remains open.

### Reproducible independent authored-cap planarity diagnostic

The OCCT verifier now additionally audits exact binary64 control-pole
planarity using Python Fraction arithmetic over all boundary edge poles of
each endpoint cap. It constructs an exact plane from independent directions
and checks every pole; a nonzero determinant is retained as a witness. This
is diagnostic and does not substitute for edge/lift composition or change
the tolerance-based STEP import gate. On external-step-oblique-cap the two
scale/twist caps and initial spatial cap are exactly coplanar; spatial face
25 is not. All three OCCT fixture checks still pass. Assertions verified
the expected positive/negative planarity outcomes. Evidence:
sweep-oblique-cap-exact-planarity-oracle.log and refreshed opencascade-sweep.json.

### Exact zero elision qualification

Exact composition convolution and scaled addition now omit products only
when an operand expansion has exact zero sign. Source resolution, nonzero
coefficient comparisons and final budget/deadline checks remain mandatory.
16 native predicate tests, boundary agreement tests, packaged WASM build,
16 public tests and the three-fixture OCCT oracle passed. Closed planar
exact-use work decreases from 793004 to 784604, retaining all 128 equal
uses and the positive volume certificate. Scale/twist and spatial still
exhaust the shared million-work budget (130/144 and 100/112 equal uses);
no new global certificate is claimed. Independent spatial retries still
report the endpoint-cap mismatch. This small reduction does not close
the shared-budget obligation. Evidence: sweep-exact-zero-elision-*.log,
sweep-zero-elision-*.log and external-step-zero-elision/manifest.json.

### Exact affine chart composition path

The composition predicate has a bounded exact affine bilinear path: equal
positive surface weights, a zero mixed control-net difference and identical
curve/pcurve weights are verified first. Pole correspondence is then checked
by cross-multiplying both chart-domain widths, without rounded inversion.
Unsupported prerequisites retain the general predicate. A mismatch is exact
and a zero/insufficient work budget remains indeterminate. Native predicate,
B-rep boundary and curve/surface agreement suites passed. A dedicated oblique
nonunit-domain rational-pole regression proves equality at its exact required
budget and refuses a one-ULP world-pole perturbation. Evidence:
sweep-affine-composition-native.log, sweep-affine-composition-boundary.log,
sweep-affine-composition-agreement.log, sweep-affine-composition-regression.log.
WASM rebuild is in progress; no fixture budget gain is claimed yet.

### Affine composition packaged fixture qualification

The packaged WASM build exited zero. All 144 scale/twist hollow-fixture
coedge uses are now exactly equal in 831431 shared work, under the unchanged
1000000 limit. Both endpoint cap contact reports are certified with no
unresolved walls. Exact boundary and all-face injectivity prerequisites
now hold, enabling 64 allowed shared boundaries; 56 pair contacts remain
unresolved (previously 96). Full boundary embedding and volume remain false.

Spatial uses now finish in 596447 work with 104 equal and eight different
endpoint uses. This exposes the known noncoplanar endpoint mismatch directly
in the combined report. Faces 16..23 remain injectivity-unproved. Closed
planar retains all 128 equal uses and its positive volume certificate.

16 public tests and the three-fixture independent OCCT verifier passed.
A new four-test embedding suite includes the actual scale/twist hollow
fixture, positive exact boundary/caps and a one-work-budget refusal, without
promoting unresolved contacts. Evidence: sweep-affine-composition-public-tests.log,
sweep-affine-composition-fixture-regression.log, sweep-affine-composition-occt.log
and external-step-affine-composition/manifest.json.

### Straight shared-edge plane candidates

The 56 unresolved scale/twist pairs comprise 32 pairs with a shared edge and
24 without a shared edge. The native opposite-sides predicate previously
required three noncollinear poles on the shared edge, excluding straight
edges. For such edges it can now choose a third point from four fixed
authored binary64 candidate points. The candidate itself is not evidence:
all edge poles must lie exactly in its plane, the actual pcurve must cover
the shared chart boundary with exact rational edge identity, every boundary
control must lie in the plane, and every off-boundary pole must have a
strict sign opposite that of the other chart. Original curved-edge behavior
is retained. False candidates conservatively remain unresolved.

Seven native shared-boundary regressions pass, including a new straight-edge
positive case and refusal of same-sided charts, extra off-edge plane contact
and a shifted boundary. Log: sweep-straight-edge-plane-native.log. Packaged
WASM rebuild is running; contact-count improvement has not been measured.

### Straight-edge packaged contact qualification

WASM build exited zero; 38 native boundary/volume regressions and 13 public
embedding/progressive tests passed. With unchanged budgets, all 32 previously
unresolved scale/twist shared-edge pairs are now classified. The fixture
retains 441 disjoint pairs and 96 allowed shared boundaries, leaving 24
unresolved pairs. Each remaining pair has no shared edge and exactly one
coincident endpoint coordinate in the retained source. A shared-vertex
certificate, with actual topological ownership and no extra contact, is the
next obligation; coordinate coincidence alone is insufficient.

Spatial classification also improves from 81 to 65 unresolved pairs, but its
exact endpoint mismatch and unresolved faces remain. Closed planar volume
stays proven. The independent OCCT oracle passes all three STEP fixtures.
A public regression checks that every unresolved scale/twist pair has no
common topological edge; the four-test embedding suite passes. Evidence:
sweep-straight-edge-plane-build.log, sweep-straight-edge-plane-boundary-tests.log,
sweep-straight-edge-public-tests.log, sweep-straight-edge-fixture-regression.log,
sweep-straight-edge-occt.log and external-step-straight-edge/manifest.json.

### Exact supporting-station vertex separator

The hull contact certificate now admits an exact projected separator after
restricting positive rational support to authored extremal station planes.
All nonzero pole signs must be opposite between faces; every zero-sign pole
must be the identical full 3D point. Both faces must own exact curve endpoints
at the same topological vertex. Coordinate coincidence without ownership
still refuses. No sampled separator or proximity tolerance is accepted.
Four native hull-contact tests pass, including a new skewed multi-station
section contact and one-ULP ownership refusal, plus prior independent-topology
and overlapping-interior regressions (sweep-station-vertex-native.log).
WASM rebuild is running; the 24 remaining scale/twist pairs have not yet
been requalified with this implementation.

### Retained scale/twist hollow volume certified

The packaged station-vertex certificate closes all remaining 24 scale/twist
vertex contacts. The actual fixture now has 441 disjoint pairs and 120
allowed shared boundaries, covering all 561 pairs. Exact edge/lift agreement,
trim validity and whole-chart injectivity hold; shell nesting is consistent
and the single material shell is outward after one certified orientation
attempt. Native solidGeometryCertified is true. This is a certificate of the
retained B-rep, not a full authored-family continuous-error certificate.

The independent OCCT verifier confirms native/external material agreement
for both scale/twist and closed planar. Spatial remains unproved with its
endpoint mismatch, unresolved faces and 65 unclassified pairs.
39 native boundary/volume tests, 13 public embedding/progressive tests and
all three independent STEP cases passed. The public fixture regression now
requires positive embedding and volume, while retaining one-work-budget
refusal. Evidence: sweep-station-vertex-boundary-tests.log,
sweep-station-vertex-public-tests.log, sweep-station-vertex-occt.log,
external-step-station-vertex/manifest.json and opencascade-sweep.json.

### Certified Rush-to-Solid positive path

Added examples/rush/progressive-miter-certified-hollow.r for the retained
straight scale/twist hollow fixture. A real parser/kernel/mesh artifact test
requires a positive viewport volume snapshot, continuousBound false, successful
Solid transfer and closed valid B-rep topology. Transfer recomputes native
volume validity instead of trusting snapshot flags.

Construction and Solid previously used different standard volume budgets,
causing an unproved viewport snapshot for a body that passed Solid admission.
DEFAULT_SWEEP_VOLUME_BUDGETS now shares the existing Solid work limits
(10000 linear cells/pairs, 100000 geometry cells, 1000000 domain cells,
1000 cells per pair); Solid uses that same constant. Exact work remains one
million. Explicit smaller budgets still refuse conservatively. Larger
construction defaults permit more audit work; no latency improvement is claimed.
15 tests across progressive, embedding and viewport evidence suites passed;
TypeScript passed. Logs: sweep-certified-rush-solid-tests.log and
sweep-certified-rush-solid-types.log. Live wide/narrow UI proof remains open.

### Live certified scale/twist viewport and Solid qualification

A temporary in-app browser tab ran progressive-miter-certified-hollow.r
through the real editor and worker. The preview exposed interpolation-error
and regularity evidence; the final scene rendered 290 triangles and visibly
reported retained body geometry proven, full continuous error unproved and
profile/wall regularity proven. Wide and 640x800 screenshots were inspected;
all status fields remained visible in the narrow layout.

The actual To Solid button completed successfully and selected Body 1 B-rep
in the CAD scene. Wide and narrow Solid screenshots were inspected. Undo
completed history restoration; the prior source was restored through the
editor and rebuilt (2560 triangles, prior unproved body status). Viewport
overrides were reset, the source panel returned to its initial hidden state
and the agent-created tab closed. This qualifies this positive fixture only;
full mode/law/cancel/restoration visual coverage remains open.
Evidence: sweep-certified-live-wide.jpg, sweep-certified-live-narrow.jpg,
sweep-certified-live-solid-wide.jpg and sweep-certified-live-solid-narrow.jpg.

### Retained wall correspondence closes a decomposition sub-obligation

Progressive miter sync/stream bodies and Rush construction reports now retain
retainedCorrespondence evidence. For clamped, nonperiodic profiles whose
internal knots already have Bezier multiplicity, direct control slicing and
exact affine parameter renaming do not alter the source rational family.
The checker matches every actual wall degree, normalized knot vector, pole
and positive weight to the corresponding two station sections; equal station
weights establish the same linear interpolation family. Positive evidence
therefore proves zero additional retained-wall error without sampled geometry
or rounded extraction. Other knot layouts, mismatches and face-budget
exhaustion return unproved. Caps are deliberately outside this certificate.

The actual scale/twist hollow fixture proves exact correspondence. A modified
retained pole loses it; a one-face budget returns work-limit. 14 embedding/
progressive tests and TypeScript passed. Evidence:
sweep-retained-correspondence-tests.log, sweep-retained-correspondence-types.log
and sweep-retained-correspondence-final-tests.log. Full continuousBound is
unchanged: cap-image correspondence and general rounded extraction remain
separate open obligations.

### Exact retained coordinate-plane cap regions

Progressive miter sync/stream bodies and Rush construction reports now retain
retainedCaps evidence (null for closed no-cap bodies). For coordinate-plane
caps, endpoint Bezier contours are compared exactly to every actual directed
world-edge sequence, including cyclic/reversed traversal and separate holes.
Native cap chart/trim/edge prerequisites are recomputed on the actual model.
The same simple planar outer/hole contours define the same material region;
positive evidence proves zero additional cap retention error relative to the
rounded endpoint section region. It does not bound the endpoint section
against the authored exact family. Oblique caps, rounded decomposition,
contour mismatch and exhausted work remain unproved.

Both scale/twist caps pass. A changed endpoint contour and zero exact budget
refuse. 14 embedding/progressive tests and TypeScript pass. Evidence:
sweep-retained-cap-region-tests.log and sweep-retained-cap-region-types.log.
Full continuousBound remains false pending the complete authored-to-endpoint
material-region error argument; no narrower success criterion is introduced.

### Explicit bounded exact-plane section correction

Added native section_projection::project and the curve_project_section
transport operation with a TS projectNurbsSection wrapper. Projection
candidates round two free coordinates to an explicit grid and construct the
dependent coordinate from an authored graph candidate. Exact orient3d
predicates then verify every stored pole against a noncollinear plane.
Candidate arithmetic alone never establishes planarity.

The displacement bound uses outward interval subtraction, products and sums
and an upward square root. Since all knots and positive rational weights
remain unchanged, the full curve displacement is a convex combination of
control displacements and is bounded by their maximum norm. No correction
is returned when this bound exceeds the explicit tolerance or exact proof
work is exhausted. Input is not mutated. The caller must compose this bound
with its authored-family budget before construction admission.

The native regression uses actual problematic spatial endpoint coordinates,
requires exact corrected planarity and unchanged weights/knots, and tests
zero tolerance/work refusals. It passed (sweep-bounded-section-projection-native.log).
WASM packaging is running. Original spatial construction is not yet changed
and still lacks its full certificate; end-to-end correction qualification
and continuous-error composition are the next steps.

### Spatial transverse projection refinement (2026-10-02)

The bounded section correction fixture passes the public coedge/cap test, TypeScript check and independent OCCT STEP verification across all four fixtures. The spatial correction displacement upper bound is 4.370307162299816e-13; its native exact boundary is certified, while volume admission remains unproved.

Whole-chart linear monotonicity now permits exact integer projection coefficients in -2..2 and proposes a transverse row by removing longitudinal motion from the original row. A rounded candidate never certifies geometry: every original knot cell still requires the outward symmetric-positive-Jacobian proof. Candidate retries share the original cell budget. The station row follows the retained chord ratios rather than coordinate signs.

On the actual corrected spatial fixture, outer wall faces 16..19 are now proved injective. Inner faces 20..23 remain unresolved; pair contacts and full spatial volume remain open. Raising subdivision depth alone consumed the shared work budget and was not retained. Four nurbs-core projection tests and four brep-core face-injectivity tests pass, including folded/collapsed negative cases. Logs: sweep-spatial-transverse-projection-native.log and sweep-spatial-wall-injectivity-native.log. This native result still requires rebuilding WASM and rerunning the public fixture before claiming viewport/Solid integration.

The transverse-projection WASM build completed successfully. Public projection/embedding/progressive suites pass (15 tests); fresh STEP exports and independent OCCT verification pass for all four fixtures in external-step-spatial-transverse. Both original and corrected spatial models now list only faces 20..23 as unresolved for injectivity; scale/twist and closed-planar retain positive native volume certificates. Corrected spatial retains exact cap/coedge certification but still lacks the full volume certificate. Attempts to bias subdivision, increase depth, center before restriction, or suppress exact zero denominator derivatives did not close the inner walls and were removed. Final native face-injectivity suite passes (4 tests). The remaining inner-wall proof needs a stronger enclosure or projection certificate, not another unsupported promotion of the admission flag.

### All retained spatial walls injective in native proof (2026-10-02)

A third fixed integer projection candidate uses midpoint tangent duals rounded to coefficients in -16..16. Midpoint evaluation only proposes a map; the original full rectangular chart still requires outward positivity of the symmetric projected Jacobian. Retries share one cell budget; the third candidate permits depth 14, while earlier candidates retain depth 12.

All 26 faces of the corrected spatial fixture now pass whole-chart injectivity, including remaining inner walls 20..23. Total shared refinement work is 15,286 cells. The 10,000-cell regression remains explicitly unproved; the 20,000-cell regression succeeds. The volume/Solid shared default is updated to 20,000 linear cells. Four brep-core face-injectivity tests pass, TypeScript passes, and the public projection regression now requires all faces injective. WASM rebuild is running; the new public test and independent STEP/native-volume matrix must be rerun after it completes. Cap/wall and wall/wall contacts remain unproved, so this does not yet certify the spatial volume or the full continuous error.

### Exact oblique cap/wall contact exclusion (2026-10-02)

The completed dual-projection WASM build passes all 15 public projection/embedding/progressive tests, including all spatial faces injective. Native cap contact auditing now falls back from coordinate-plane checks to exact oblique control-hull classification. Three noncollinear retained cap poles define a candidate plane; exact orient2d establishes noncollinearity, and exact orient3d verifies every cap pole coplanar. All wall poles must be strictly on one side, or exactly one declared natural boundary row must lie on the plane with all other poles strictly on one side. Existing whole-boundary coverage and exact coedge identity remain required for allowed contact. These predicates charge the report's existing shared exact-work budget.

For the corrected spatial end cap, eight shared-edge contacts and sixteen disjoint walls are proved. Zero budget and a one-ULP cap-pole perturbation are refused. Joint native auditing now has all faces injective, exact coedges, and both caps' full wall-contact proofs. Nineteen wall/wall pairs remain unresolved: [9,16], [9,18], [10,19], [13,20], [14,23], [15,20], [16,19], [16,20], [16,21], [16,22], [16,23], [17,18], [18,20], [18,21], [18,22], [18,23], [20,23], [21,22], [21,23]. Boundary embedding and spatial volume remain unproved. Native regression passes; log sweep-oblique-cap-contact-native.log. The oblique-contact WASM build is running and must be checked before the strengthened public cap regression/STEP matrix are executed.

### Spatial single-vertex supporting planes (2026-10-02)

Exact control-hull contact now also considers a full 3D plane when all three coordinate hull intersections have positive width. Candidate planes pass through a common authored topological vertex and two retained poles or fixed binary64 anchors. Every zero-side control pole must equal that same vertex; all remaining poles in each face must lie strictly on one side, with opposite signs between faces. Both faces must own exact curve endpoints at the shared vertex. Enumeration is restricted to at most 16 poles per face. This is a sufficient proof of a single-vertex contact, with no proximity tolerance or fitted normal.

Four formerly unresolved spatial pairs [9,16], [10,19], [13,20], [14,23] are certified. An extra common off-vertex pole is explicitly refused. The joint native test leaves 15 unresolved wall pairs and no unresolved cap contacts; spatial boundary/volume remains unproved. Native sweep-cap regression and 39 broader boundary/volume tests pass. Logs: sweep-spatial-vertex-plane-native.log and sweep-spatial-vertex-plane-boundary.log. The oblique-cap WASM build has completed; its strengthened public cap test and STEP export are running. The newer single-vertex-plane WASM rebuild is also running and requires its own subsequent verification.

Oblique-cap public tests pass (15) and independent OCCT verification passes across four exports. Manifest inspection found that the STEP exporter still overrode the shared default with maxLinearCells=10000; this left faces 22..23 unresolved in its diagnostic despite the public test's successful 20000-cell proof. The stale override is removed and a fresh export to external-step-oblique-cap-shared-budget is running. The earlier external-step-oblique-cap report remains bounded at the old budget and must not be cited as all-face injectivity evidence.

### Exact hull gaps and shared straight-edge planes (2026-10-02)

STEP exports with the shared 20,000-cell injectivity default pass independent OCCT verification for all four fixtures; both spatial reports now have no unresolved injectivity faces. The single-vertex-plane WASM build also passes 15 public tests and independent OCCT checks.

A new exact control-hull separator enumerates planes through three original poles of either face (nets at most 16 poles per face). Own poles may lie on the separator; every pole of the other face must lie strictly on the opposite side. Positive rational weights make this a whole-surface disjointness certificate. Each attempted plane charges a contact cell; per-pair predicate work is bounded at 100,000, and candidate enumeration reserves half the pair cell budget for the existing interval search. Exhausted budgets retain unresolved status. Tests cover exact oblique gaps, reversed face order, touching, crossing and zero budget.

Straight shared-edge separation additionally tries rounded midpoint plane proposals from the two control nets. All source edge/plane incidences and all off-boundary strict signs remain exact predicates; rounded proposals are not evidence. Four adjacent spatial pairs [16,19], [17,18], [20,23], [21,22] are classified as allowed shared boundaries. Nine control-hull gaps are proved: [16,20], [16,21], [16,22], [16,23], [18,20], [18,21], [18,22], [18,23], [21,23]. Only [9,18] and [15,20] remain unresolved in the native joint test. Seven shared-boundary tests and three face-contact tests pass; broader tests exposed an exhausted-budget search call, which is fixed by retaining unresolved status instead of invoking search with zero work. The broader suite and strengthened joint regression are rerunning. A WASM build was started before that final budget fix; its output needs checking/rebuilding against the final source before qualification.

After the budget fix, all 39 broader boundary/volume tests and the strengthened spatial joint regression pass. The remaining pair set is still exactly [9,18], [15,20]; the spatial boundary and volume certificate remain false.

### Corrected spatial native geometric volume certified (2026-10-02)

A bounded 256-iteration supporting-plane proposal aligns a normal to signed normalized directions from a shared vertex to the two control nets. Binary64 anchors proposed from that normal are not evidence. The existing exact orient3d check still requires every non-vertex pole strictly on one side, opposite between the faces; zero-side poles must equal the common authored vertex, and both faces must own its exact edge endpoint. This closes the last pairs [9,18] and [15,20]. No source pole is changed by this proposal.

The corrected spatial fixture now has no unresolved pairs and a proved native boundary embedding. Native volume auditing subsequently proves the single-shell material nesting and outward orientation (four ray attempts), producing proven=true. Flipping every shell face orientation keeps the boundary proven but yields outward=false and volume proven=false; this negative regression passes. All 39 broader boundary/volume tests pass, the strengthened spatial volume test passes, TypeScript passes and diff whitespace checks pass. Logs: sweep-spatial-volume-native.log, sweep-spatial-vertex-proposal-boundary.log, sweep-spatial-volume-types.log.

The public projection regression now requires both boundaryEmbeddingCertified and solidGeometryCertified for the actual corrected spatial fixture. A WASM build for these latest predicates is running (sweep-spatial-volume-build.log); public tests and independent OCCT/native material agreement must be rerun after completion. This certificate is for the retained BRep with explicit bounded endpoint correction. Automatic correction through authored Rush construction and composition into full sweep continuousBound remain outstanding; no full authored continuous error guarantee is claimed.

### Spatial volume confirmed through public WASM and independent STEP (2026-10-02)

The latest WASM build completes successfully. All 15 public projection/embedding/progressive tests pass, including the corrected spatial boundary and volume certificate. Fresh four-fixture exports in external-step-spatial-volume pass independent OCCT verification. Native/external material agreement is true for scale/twist, corrected spatial and closed planar; the unchanged original spatial remains unproved with eight different cap coedges and receives no material-agreement promotion.

Shared exact coedge totals: scale/twist 831431 for 144 equal uses; corrected spatial 614347 for 112 equal uses; closed planar 784604 for 128 equal uses. Each uses the original shared 1000000 budget. Original spatial uses 596447, with 104 equal and eight different uses. The contract matrix now records these fixture-level results separately from all-mode authored error and continuity requirements. Full continuousBound, automatic bounded correction through Rush, all-mode certification, G1/G2 and the broad STEP/UI matrix remain open.

### Continuous correction bound for the full section interpolation (2026-10-02)

`projectSweepSections` composes explicit section projections with one shared exact-work budget. It requires identical curve degrees, knots, pole counts and positive weights across all stations. For this basis, the displacement of every ruled interpolation point is a convex combination of source-to-corrected pole displacements, so the maximum outward section bound also bounds the entire wall family. Unsupported bases, changed station weights and exhausted work return no sections and no partial upper bound. Input sections are preserved. Caps and the authored sweep-to-original-rounded-section error remain separate obligations.

The public spatial regression passes with input-preservation, changed-weight refusal, zero-work refusal and shared budget exhaustion between two corrections. TypeScript and whitespace checks pass. The STEP exporter now binds the corrected section family to all 24 actual retained wall faces via exact coefficient correspondence before reporting retainedWallCorrectionUpper. Fresh exports in external-step-section-correction-bound retain the positive native spatial volume certificate; correspondence is exact with wallErrorUpper=0, continuous correction upper=4.370307162299816e-13 and shared projection work=9270. Independent OCCT verification passes for all four fixtures. Logs: sweep-section-interpolation-correction-public.log, sweep-section-interpolation-correction-types.log, sweep-section-correction-bound-occt.log.

This closes the correction-to-retained-wall component of E for this compatible basis. It does not promote the complete authored continuousBound; authored station/frame/interpolation error, cap-region correction and ordinary Rush integration still require completion.

### Ordinary miter constructor and authored Rush correction controls (2026-10-02)

`createMiterBrepProfileBody` accepts an explicit optional capCorrection (quantum, tolerance, maxWork). Endpoint planes are proposed from the first/last path chords, and both endpoint sections are corrected in one shared budget before walls and caps are constructed. Exact retained-wall coefficient correspondence is required before returning corrected geometry. Closed paths, unproved correction and insufficient budgets are refused; no global authored rounding certificate is promoted. Default calls retain their existing geometry.

Rush/schema/kernel support cap_correction_tolerance and cap_correction_quantum (lengths), plus cap_correction_max_work (bounded integer). Quantum/work without a tolerance is refused by the graph kernel. New example: examples/rush/miter-hollow-corrected.r. Native text lowering test passes. Constructor/projection/miter suites pass seven tests, including positive volume, zero-work/zero-tolerance refusal and input preservation. TypeScript passes. The subsequent full Rush regression initially exposed stale language WASM, then the separately embedded JSON schema; the schema is regenerated from modelGraphNurbsSchema and the language kernel is rebuilding. Full Rush parser-to-Solid regression must be rerun after that build.

Independent STEP matrix now includes a fifth fixture, open-spatial-constructor-correction.step, generated through the public body constructor with both caps corrected. All five exports in external-step-constructor-cap-correction pass OCCT verification. This does not claim complete continuousBound or full live UI coverage.

With regenerated schema and rebuilt language WASM, all eight public miter/projection tests pass. The new example compiles dimensional correction controls, builds corrected BRep, renders via parseOpenSCAD, converts the actual scene to Solid, and the resulting Solid BRep passes native geometric volume certification. Zero correction work refuses graph construction. This is automated Rush→parser→scene→Solid evidence; live viewport presentation/status and cancellation/source-change UI checks remain to be completed.

### Ordinary miter viewport evidence and independent Solid admission (2026-10-02)

Ordinary miter construction now records a native volume audit and explicit continuousBound=false, so its actual final scene artifact feeds the existing viewport evidence panel. The corrected Rush example reports solidGeometryCertified=true without claiming authored error or separate profile/wall regularity certificates. Missing ordinary regularity fields are explicitly kept false in presentation.

Solid admission now covers both brep_miter_sweep and brep_progressive_miter_sweep, following affine transform provenance and recomputing the audit on the actual transformed BRep. Cached flags do not authorize admission. The unchanged spatial miter example renders but refuses Solid because its boundary embedding is unproved; its old Solid-success assertion is replaced by the required refusal check. Corrected source passes scene→Solid and native volume inspection. Tests also prove ordinary translated geometry is admitted and an inward model is rejected despite a positive snapshot flag. Boolean/fillet/chamfer provenance remains open.

The five-suite targeted run passed 23 tests. After adding ordinary transform/forged-positive coverage, the three focused suites pass 11 tests; TypeScript and whitespace checks pass. Evidence: sweep-ordinary-miter-admission-public.log and sweep-ordinary-miter-admission-types.log. These are automated artifact/scene checks; live wide/narrow viewport, cancellation, source replacement and restoration are still pending for the corrected ordinary mode.

### Ordinary miter live viewport visibility fix (2026-10-02)

Live inspection exposed a source-drawer condition limited to progressive operators: ordinary brep_miter_sweep built successfully (848 triangles, volume 20.04), but its source viewport and certificate remained hidden behind the empty Mesh workspace. The source-preview condition now includes ordinary miter as well as progressive miter. Live hot reload confirms the actual ordinary body and status: geometric volume proved, full continuous error unproved, independent profile/wall regularity unknown. Evidence: sweep-ordinary-miter-live-wide.jpg. A narrow-window capture was taken, but does not yet establish full responsive interaction coverage.

Changing the source visibly invalidates the previous result and removes its certificate until the new build. The original closed progressive source was restored and rebuilt successfully (2560 triangles); Auto was restored on, Solid mode restored, the source drawer hidden, original model/Body 1 preserved and the temporary tab closed. This turn does not establish ordinary live Solid success/refusal or cancellation coverage; those remain in the UI matrix.

### Ordinary miter live Solid success and Undo (2026-10-02)

The corrected ordinary Rush example completes the actual editor's To Solid action: the editor closes, Solid displays the spatial hollow miter as a selected Body 1 B-rep in model, and Undo becomes available. Screenshot: sweep-ordinary-miter-live-solid.jpg. Undo restores the prior model body; selection clears, Undo disables and Redo becomes available. This covers ordinary live conversion success and history restoration. Ordinary refusal, explicit build/conversion cancellation and full narrow-window interactions still require live qualification.

### Live ordinary refusal and build cancellation (2026-10-02)

Removing the explicit cap-correction controls from the same spatial ordinary source causes the real To Solid action to return `Miter sweep Solid geometry could not be proved: boundary embedding.` The source drawer stays open; Solid still has the original model/Body 1 and both history buttons disabled. The previous mesh result remains explicitly labelled as previous, with no stale positive certificate. Screenshot: sweep-ordinary-miter-live-refusal.jpg.

The original closed progressive source was then rebuilt successfully. A second build was explicitly cancelled through the visible Cancel Build button before completion; accessibility state reports `Сборка отменена`, Build is enabled again and the prior 2560-triangle result remains. Screenshot sweep-live-build-cancelled.jpg shows the retained scene after the transient cancellation toast expired. This proves editor worker-build cancellation, not To Solid conversion cancellation. Auto was restored; a fresh post-cancellation build completes with 2560 triangles, volume 1.35 and area 42.34. Original Solid mode/body and hidden source drawer were restored, then the temporary tab closed. Full responsive interactions and conversion cancellation remain open.

### Live To Solid cancellation and retry (2026-10-02)

The corrected ordinary source's To Solid action was explicitly cancelled using its own Cancel button before completion. The source drawer stays open, To Solid becomes enabled again, the previous result is labelled previous with no positive certificate, and the original Solid body/history remains unchanged. A subsequent state observation confirms no late scene replacement. Retrying the same source succeeds, closes the drawer and produces the selected spatial Body 1 B-rep with Undo available. Screenshot sweep-live-solid-after-cancel.jpg records this successful recovery. Undo restores the original body. This closes wide-window conversion cancellation/retry for this ordinary fixture; the full mode/responsive matrix remains open.

### Shared exact work for retained cap correspondence (2026-10-02)

The retained-cap correspondence inspector previously reused the full exact-work allowance independently for each endpoint cap. It now subtracts the first native cap audit's consumed exact work before auditing the second cap. Positive scale/twist cap correspondence still passes at the existing budget; a budget sufficient only for the first audit refuses the pair without a partial capErrorUpper. Four embedding/admission tests pass, TypeScript and whitespace checks pass. Logs: sweep-retained-caps-shared-work.log and sweep-retained-caps-shared-types.log. This strengthens the retained-cap component's budget accounting; it does not prove the authored cap-region displacement or full continuousBound.

### Retained cap consumed-work evidence (2026-10-02)

Retained cap reports now expose accumulated exactWork on both success and refusal, allowing construction evidence to show actual consumption of the shared limit. Regression requires positive work bounded by the original limit and exactly retained consumption when the second cap is refused after the first exhausts that limit. Embedding/admission plus progressive-miter suites pass 14 tests; TypeScript and whitespace checks pass. Logs: sweep-retained-caps-work-evidence.log and sweep-retained-caps-work-types.log. Inspection confirms the native level certificate still separates retained-wall interpolation from cap/decomposition obligations; no full authored continuousBound is promoted.

### Ordinary retained-wall regularity certificate (2026-10-02)

Ordinary miter construction now separately audits every actual decomposed wall chart with the shared default 20,000-cell allowance and retains face-level evidence. The existing outward whole-chart positive symmetric projected Jacobian proves rank two, so a positive chart audit establishes wall regularity independently of cap contacts/material orientation. report.wallRegularityCertified uses this recomputed evidence; missing authored-profile evidence remains false and continuousBound remains false.

The corrected spatial constructor certifies all 24 retained wall charts with no unresolved faces and work within the shared allowance. Three constructor/projection/chart suites pass 10 tests; the strengthened full Rush-to-scene viewport-artifact test passes in the six-test miter suite and requires wallRegularityCertified=true, profileRegularityCertified=false and continuousBound=false. TypeScript and whitespace checks pass. Logs: sweep-ordinary-miter-regularity.log, sweep-ordinary-miter-regularity-viewport.log, sweep-ordinary-miter-regularity-types.log. Live display of this new regularity field and all-mode regularity remain separate qualification obligations.

### Public shared profile-regularity audit preparation (2026-10-02)

The existing native whole-domain one-sided tangent inspector is exposed as sweep_profile_regularity_audit, with one shared 100,000-cell maximum across up to 64 source curves. Reports list unresolved profile indices and consumed cells; continuityCertified remains false even for a regular C0 corner. A TypeScript wrapper and public regressions cover outer/hole profiles, shared exhaustion, zero work, stationary tangents, corners and input preservation. The two native tangent regressions and TypeScript pass. The geometry WASM build is still active; public tests must run against its completed output before ordinary constructor integration or profile-certification claims. Logs: sweep-profile-regularity-native.log, sweep-profile-regularity-types.log, sweep-profile-regularity-build.log.

### Ordinary authored profile regularity integration (2026-10-02)

The geometry WASM build completed successfully. Both public profile-regression tests pass, including invalid input/count/work limits and shared exhaustion. Ordinary miter now retains the native profile audit with one shared 10,000-cell limit and derives profileRegularityCertified from its spanwiseRegular result. The corrected Rush fixture's viewport artifact reports both profile and retained-wall regularity true while continuousBound stays false. Miter/projection integration tests and TypeScript pass; whitespace checks pass. Logs: sweep-profile-regularity-public.log, sweep-ordinary-profile-regularity.log, sweep-ordinary-profile-types.log. This proves nonzero one-sided source tangents, not continuity of profile corners, moving frames or closed seams. Live status qualification and all-mode regularity remain open.

### Six-fixture independent STEP matrix (2026-10-02)

The STEP oracle adds open-straight-hollow-miter.step through the ordinary constructor: ten faces, one shell, two hole-bearing caps, expected volume pi*(0.5^2-0.2^2)*10. Its native volume/profile/wall certificates are positive; shared exact coedge work is 213045 of 1000000. The corrected spatial constructor export retains positive profile/wall certificates and native volume, with 580720 shared exact coedge work. The original uncorrected spatial remains explicitly volume-unproved. All six fresh exports in external-step-profile-regularity pass the independent OCCT verifier. Logs: sweep-profile-step-export.log and sweep-profile-step-occt.log. This extends the fixture matrix; full laws/modes, continuity and authored-error coverage remain outstanding.

### Closed miter retained regularity includes the closing layer (2026-10-02)

The existing hollow closed Rush fixture now requires positive source-profile and retained-wall regularity with explicit evidence for every face 0..31, including the final eight closing-layer walls, within 20,000 shared cells. A collapsed closing face 24 is independently refused by the retained-chart inspector. All six miter tests, TypeScript and whitespace checks pass. Logs: sweep-closed-miter-regularity.log and sweep-closed-miter-regularity-types.log. The STEP exporter now preserves the closed constructor report for subsequent exports. This proves whole-chart regularity and coverage of the cyclic layer; continuity across joins and G1/G2 remain separate and unproved, and continuousBound stays false.

### Sharp closed-miter corners refuse seam smoothness (2026-10-02)

The actual closed hollow Rush-to-Solid B-rep now audits the four path-corner joins, including the cyclic last-to-first join, through the public seam auditor. All four refuse first-order jet agreement at 1e-8, and exactG1G2Certified stays false despite positive whole-chart regularity and closed topology. This distinguishes intentionally sharp miter corners from applicable smooth seams. Ten miter/seam tests, TypeScript and whitespace checks pass. Logs: sweep-closed-miter-corner-seams.log and sweep-closed-miter-corner-types.log. Exact positive G1/G2 for applicable smooth sweep modes remains unfinished.

### Ruled second-order boundary-jet inspection (2026-10-02)

Read-only surface-jet inspection previously required cross degree at least the requested order even for linear ruled strips. It now validates their available clamped first-order layers and supplies an identically zero second homogeneous jet. The existing rational quotient terms remain: linear homogeneous geometry does not imply zero Euclidean second derivatives when weights vary. Surface matching/editing keeps its original degree requirements.

Native seam regressions pass, including a positive planar ruled second-order join and refusal after cross weights change. Public tests now require bounded G2 jet agreement for the planar ruled pair while retaining exactG1G2Certified=false and sharp-join refusal. Geometry WASM rebuild is active; public qualification must rerun after completion. Log: sweep-ruled-g2-native.log; build: sweep-ruled-g2-build.log. This extends bounded jet qualification, not exact G1/G2 certification.

The ruled-G2 WASM rebuild completed. All ten public seam/miter tests pass, including the positive planar ruled pair and a rational pair whose first Euclidean jets agree but second jets differ: order 1 passes and order 2 refuses with jet-deviation-exceeds-budget. Sharp miter corners still refuse. TypeScript and whitespace checks pass. Logs: sweep-ruled-g2-public.log and sweep-ruled-g2-types.log. This confirms quotient derivative terms remain active; exactG1G2Certified remains false and the full exact smoothness objective is still open.

### Exact Bezier boundary-strip jet predicate foundation (2026-10-02)

cad-predicates now contains a bounded sufficient exact homogeneous C1/C2 strip-jet identity over immutable original coefficient leaves. Inward cross jets use exact degree factors and a positive supplied parameter scale; second homogeneous jets of linear strips are exactly zero. Boundary Euclidean coefficients and weights must agree in the common seam basis. Scale is represented relative to an exact unit leaf, preserving dimensionless scale under the predicate engine's common projective normalization. No rounded derivative values enter the identity.

The native predicate regression proves the planar linear C2 strip, detects a one-ULP derivative mismatch and refuses zero work. The broader native continuity regression passes 24 tests. Logs: sweep-exact-strip-jets-native.log and sweep-ruled-g2-continuity-regression.log. This foundation is not yet wired to surface seam admission/WASM; rational/mixed-degree cases, regularity prerequisites and source-basis checks need qualification before any exactG1G2Certified promotion.

### Exact strip-jet rational and mixed-degree qualification (2026-10-02)

Native exact-strip regressions now cover nonuniform positive rational weights along the seam, a parameter scale of two, a wrong-scale refusal, mixed linear/quadratic cross degrees, boundary-weight mismatch, and a second-layer perturbation that preserves exact C1 while refusing C2. All 18 cad-predicates tests pass; the new module is formatted and whitespace checks pass. Log: sweep-exact-strip-jets-rational.log. The predicate is a sufficient homogeneous identity condition; a Different result does not prove absence of general geometric G1/G2. Surface basis admission, whole-seam regularity and WASM integration remain before a positive sweep smoothness certificate.

### Exact original surface-strip admission and seam regularity (2026-10-02)

The native surface inspector now admits only XYZ, nonperiodic single-Bezier charts in both axes with matching normalized along-seam degree. Boundary strips are indexed directly from original poles and weights, including reversal of cross layers at maximum boundaries; no derivative reconstruction enters the exact predicate. Exact identity must additionally pass both existing outward whole-seam regularity certificates. A planar ruled C2 join is certified, a one-ULP mismatch and zero exact work are refused, and coincident jets on singular charts retain exact_identity=true but certified=false. All 25 native continuity tests and whitespace checks pass. Logs: sweep-exact-surface-strip-native.log and sweep-exact-surface-strip-continuity.log. Public seam integration/WASM and wider multispan/general geometric G1/G2 remain outstanding.

### Public exact surface-strip transport preparation (2026-10-02)

surface_exact_strip_jets_audit exposes the native sufficient exact C1/C2 report independently of tolerance-based seam reports. The TypeScript inspectSweepExactStripJets wrapper returns certified, exactIdentity, regularityCertified, work and reason. Native JSON transport regression preserves all positive evidence; both exact-strip native tests and TypeScript pass. Public regressions cover original-input preservation, one-ULP mismatch, zero work, singular jets, unsupported multispan basis and invalid scales/budgets. Geometry WASM compilation has completed; packaging remains active, and these public tests must pass against the completed package before public certification is claimed. Logs: sweep-exact-strip-transport-native.log, sweep-exact-strip-public-types.log, sweep-exact-strip-public-build.log.

### Public exact seam-set qualification (2026-10-02)

The geometry WASM build completed successfully and both public exact-strip tests passed against the packaged kernel. inspectSweepExactSeams now audits every declared seam using one shared exact-work budget, retains each exact identity and regularity result, and lists unresolved seam indices. Only a nonempty set with every seam certified receives exactG1G2Certified=true (sufficient represented C1/C2 on admitted regular single-Bezier strips). A budget sufficient for the first of two seams cannot certify the set: the second reports zero remaining work. A one-ULP mismatch on the second seam also refuses the aggregate. Input preservation, invalid indices/budgets and an empty declaration set are tested. Combined public exact and bounded seam suites pass all seven tests; TypeScript and targeted whitespace checks pass. Logs: sweep-exact-strip-public.log, sweep-exact-seam-set-public.log, sweep-exact-seam-set-types.log. Existing tolerance-based reports remain distinct. This is not an audit of undeclared topology, general multispan G1/G2, moving frames, cyclic seams or complete authored sweep error.

### Exact cyclic seam-set regression and certified order (2026-10-02)

A closed four-patch cubic extrusion with integer original coefficients now verifies exact regular C1 at all joins, including patch 3 to patch 0. Its second jets do not match: requesting C2 refuses all four joins. A one-ULP change in the final patch's closing derivative refuses only seam 3, and an exact-work budget covering the first three joins cannot certify closure. This tests represented cyclic patch joins, not arbitrary authored moving-frame closure. The aggregate report now includes certifiedOrder (minimum requested order on a completely certified nonempty set, otherwise null), preventing a C1 or mixed-order positive from being presented as C2. Both public seam suites pass eight tests; TypeScript and targeted whitespace checks pass. Logs: sweep-exact-cyclic-seams.log and sweep-exact-cyclic-types.log. General multispan geometric G1/G2, authored frame closure and full continuousBound remain open.

### Exact retained-cap planarity preparation (2026-10-02)

Native cap-contact reports now independently expose planarControlHullCertified. A nondegenerate corner-plane proposal is verified using exact orient2d/orient3d on every original binary64 control point. Its work shares the cap report's existing maxExactWork with coedge identities and contact checks. The direct native regression passes for an oblique rational bilinear chart and refuses a one-ULP nonplanarity, exhausted shared budget and collinear anchors. The existing corrected-spatial full cap-contact regression and geometry WASM build remain running; these are not yet qualified. The TypeScript retained-cap correspondence still keeps its coordinate-plane admission until the new packaged kernel and public oblique-cap regression are verified. Logs: sweep-cap-exact-planarity-native.log, sweep-cap-exact-planarity-check.log and sweep-cap-exact-planarity-build.log.

The full corrected-spatial cap-contact regression subsequently completed successfully: both native cap-contact tests pass. Geometry WASM packaging remains active (session 41844); public qualification is still pending.

### Public retained oblique-cap correspondence (2026-10-02)

Geometry WASM packaging completed (wasm-opt 11838391 to 10522251 bytes). Retained-cap correspondence now requires the independent native planarControlHullCertified together with capCertified, instead of accepting only a coordinate-constant control net. A public affine shear produces exact oblique endpoint planes; both cap reports and the retained region correspondence certify them with zero additional cap error. Zero exact work and a warped original cap control point are refused, and inputs remain unchanged. All five public embedding tests pass, including scale/twist shared-cap-budget checks and full volume evidence. TypeScript and targeted whitespace checks pass. Logs: sweep-oblique-retained-caps-public.log, sweep-oblique-retained-caps-types.log, sweep-cap-exact-planarity-build.log. Constructor regressions are still running. This proves equality to the given endpoint contours, not displacement from authored uncorrected caps or complete continuousBound.

The ordinary miter and progressive sweep public constructor regression suites subsequently completed successfully; see sweep-cap-planarity-constructor-regression.log for test counts and timing.

### Authored endpoint-contour error evidence (2026-10-02)

The native progressive-miter whole-level certificate now retains endpointContourErrorUpper, separately bounding both entire rational endpoint contours against the authored interval frame/law family. It reuses existing outward point enclosures on original profile coefficients and unchanged positive weights; no sampling or extra unaccounted certificate work is introduced. Exhausted or unresolved level certificates expose no partial endpoint maxima. A native test moves only the last retained endpoint by 0.125 and proves its reported error is at least 0.125 while the first remains below 1e-10; cell exhaustion returns null. All 33 native progressive-miter tests pass, TypeScript and targeted whitespace checks pass. The field is wired through JSON transport and TypeScript; a public WASM regression is prepared. The new geometry WASM build is active (session 54443); public qualification remains pending. Filled cap-region correspondence to the authored endpoint (including holes and correction displacement) still needs its own proof; continuousBound remains false. Logs: sweep-endpoint-contour-native.log, sweep-endpoint-contour-types.log, sweep-endpoint-contour-build.log.

### Endpoint evidence through worker and Rush artifact (2026-10-02)

endpointContourErrorUpper is now preserved in progressive-miter worker previews and snapshot-local Rush native geometry evidence. The worker validator admits only null or exactly two finite nonnegative bounds; malformed length, negative values, NaN and infinity are refused. Existing preview acceptance remains determined by the whole-wall certificate, not this contour-only evidence. All 20 worker protocol tests and TypeScript pass. Logs: sweep-endpoint-contour-protocol.log and sweep-endpoint-contour-protocol-types.log. The geometry WASM build remains active (session 54443).

#### Filled-cap transfer condition still to implement

For two compact material regions A and B in the same oriented plane, each with certified simple outer contour and correctly nested oriented hole contours, a parameterwise pairing of every corresponding boundary curve within epsilon implies Hausdorff distance of the filled regions at most epsilon. To see this, linearly homotope each paired oriented curve. If a point x has distance greater than epsilon from boundary B, no homotopy segment can reach x: every segment remains in the epsilon tube of its B endpoint. The winding sum at x therefore remains unchanged. A point in A outside B consequently cannot have distance greater than epsilon from B; interchange A and B for the reverse direction. This argument includes holes, does not require intermediate contours to be simple, and requires the endpoint regions' winding sums to represent the same material ownership convention.

This lemma cannot yet be applied merely from endpointContourErrorUpper: the authored and retained endpoint planes can differ, and authored outer/hole ownership and their common oriented plane have not been independently certified at the ideal interval-valued endpoint. Generalizing requires a certified nonsingular plane projection/affine correspondence, bounds for interior displacement under that map, and endpoint region topology. Retained-cap certification proves only the stored region's ownership. Do not promote full continuousBound until these additional obligations and decomposition transfer are verified.

The endpoint-contour geometry WASM build subsequently completed (11838946 to 10522686 optimized bytes), and all ten public progressive-miter tests pass against the packaged kernel. The rounding-bound regression confirms both transported endpoint bounds are finite nonnegative, do not exceed the whole-wall bound, and do not promote continuousBound. Log: sweep-endpoint-contour-public.log.

### Eight-case independent STEP matrix with oblique affine caps (2026-10-02)

A fresh eight-case export and independent OpenCascade verification pass in external-step-oblique-caps. The original six fixtures are requalified against the current geometry WASM. Two determinant-one affine shears are added: affine-oblique-hollow-miter.step retains the ordinary circular constructor coefficients and correctly refuses native volume/retained-cap certification; affine-oblique-dyadic-hollow.step uses original dyadic quarter-arc controls with outer radius 0.5 and hole radius 0.25 and positively certifies retained cap regions and complete native volume. These are transformed retained bodies, not authored affine miter-law support.

The independent Fraction-based boundary-pole oracle detects both nonplanar caps in the first shear, while certifying both caps coplanar in the second. Positive retained-cap evidence now additionally requires this independent agreement in the OCCT verifier. Both STEP imports are valid within the declared tolerance, preserve coefficient/control-net and coedge/loop/holes evidence, have ten faces and one outward material shell, and agree with analytic volume at relative error about 1.562e-10. OCCT tolerance acceptance does not override exact native refusal. The whole matrix has eight passing external cases, with native geometric certification explicitly false on original spatial and rounded oblique-circle cases. Logs: sweep-oblique-step-export.log and sweep-oblique-step-occt.log. Artifacts: external-step-oblique-caps/manifest.json and external-step-oblique-caps/opencascade-sweep.json. General affine/frame/guide miter laws and the complete mode/UI matrix remain open.

### Cap-contact shared exact-work budget preparation (2026-10-02)

inspectSweepCapContacts no longer resets exact work at each cap. Its diagnostic coedge identities share one maxWork across the full cap set; the independently recomputed native contact reports share a separate maxWork across that same set. These are two explicitly distinct audit families, not a single combined expenditure. Per-cap continuous product/cell budgets remain separately scoped. Regression checks sum each family's expenditure and use a budget sufficient for the first cap to require refusal/zero remaining work on the second. TypeScript and targeted whitespace checks pass; cap-contact and progressive-miter public regressions are running. Logs: sweep-cap-contact-shared-work.log and sweep-cap-contact-shared-work-types.log. This does not alter authored affine/frame/guide miter-law support, which still needs native and interval-certificate integration.

Both public regression suites subsequently completed successfully: 13 tests pass, including separate diagnostic/native first-cap budget exhaustion and progressive-miter construction.

### Miter vector-law interval foundation (2026-10-02)

progressive_miter::vector_certificate now encloses original XYZ rational law values and first/second authored-knot derivatives over normalized traversal. Each component uses the existing outward homogeneous blossom certificate with original binary64 coefficients, degrees, knots and positive weights. The three component audits consume one shared cell budget; exhaustion or an unproved positive axis scale discards all partial vectors. Signed center offsets are admissible without positivity, while axis scale requires a positive lower bound on every component. Tests cover nonuniform rational weights, multiple spans, an independent law domain [2,4], original-curve values and available derivative evaluations, budgets 0/2/5 versus required 6, negative center values, scale refusal and invalid traversal/work limits. Both direct tests and all 35 progressive-miter native tests pass; targeted whitespace checks pass. Logs: sweep-vector-law-certificate-native.log and sweep-vector-law-miter-regression.log. This native law certificate is groundwork for station/remainder integration; authored affine miter generation and Rush/viewport/Solid support are not yet implemented, and no sweep guarantee is promoted from this law-only evidence.

### Affine local-station interval composition (2026-10-02)

The certified frame report now exposes station_point_affine and station_with_affine_laws. Local convention is q = scale*(axis.x*x*n + axis.y*y*b) + center.x*n + center.y*b + center.z*t, followed by the same miter-plane projection and path translation. Profiles remain in the initial normal plane, so their axial offset is zero; the positive third axis is validated for consistent XYZ law admission. Center is not multiplied by uniform scale. Axis and center laws preserve their independent authored domains and share one vector-component-cell budget, separate from scalar scale/twist work. Existing scalar-only station methods delegate to identity axes and zero center.

A translated/corner analytic case verifies both open endpoint placement and the corner-plane projection of a nonzero axial center, rejects a nonpositive axis interval, and refuses budget 5 when axis+center require 6 cells. All 36 native progressive-miter tests pass and targeted whitespace checks pass. Log: sweep-affine-station-enclosure-native.log. The station enclosure is not yet wired into the affine generator or interpolation remainder: both must be integrated before exposing authored affine miter in Rush, viewport or Solid. Existing complete continuousBound stays false.

### Native affine miter generation and interpolation certificate (2026-10-02)

Sweep::with_affine_laws now validates continuous XYZ laws, positive axis controls and matching closed-path endpoint values, then applies local axis scale and center to actual stored sections. Whole-level and endpoint certificates use the same affine laws. The interval remainder includes product-rule derivatives of uniform scale times axis scale plus center, rotation derivatives, and the full miter projection field. Retaining the full projection field is essential for axial center: a missing plane at an open endpoint cannot be treated as an implicit tangent plane. Axis/center certificates retain independent law domains and share one component-cell budget. Smooth cells use the second-derivative remainder; cells crossing any scalar/vector law knot use the Lipschitz remainder.

Tests prove an analytic quadratic center remainder of 0.5, the same 0.5 error created by a linearly varying axial center at a collinear miter station, and insufficient vector budget refusal. A generated affine surface with axis domain [2,5] and quadratic center domain [7,9] is accepted at eight subdivisions, refused at one, and its certified bound encloses independent analytic profile/interpolation checks. Native JSON transport preserves affineLawsApplied and interval-affine-law-interpolation while keeping continuousBound=false. All 38 native progressive-miter tests, TypeScript and targeted whitespace checks pass. Logs: sweep-affine-remainder-native.log, sweep-affine-generator-native.log and sweep-affine-generator-types.log.

The TypeScript options/payload and all native progressive-miter transport branches now accept axisScale/centerLaw; missing components use identity/zero laws. The public WASM regression is prepared, and the updated geometry build is active (session 73962). Public qualification and Rush schema/lowering, viewport/Solid tests remain pending. Authored frame/guide laws and full filled-cap/decomposition error are still open.

### Rush affine-miter integration preparation (2026-10-02)

Rush lowering and the strict TypeScript progressive-miter node schema now accept axis_scale and center_law vector-law records. Build arguments preserve both laws through synchronous and streamed body construction. Existing runtime dimensional resolution already treats center_law values as LENGTH and axis_scale values as SCALAR; their knot domains remain independent. The exported JSON schema and language prompt are regenerated from the source schema, and the language WASM rebuilt after the first public test revealed that the runtime still rejected axis_scale against its previously exported schema. The updated geometry WASM build has also completed successfully. TypeScript and targeted whitespace checks pass.

examples/rush/miter-affine-hollow.r exercises an anisotropic hollow body with a moving local center, explicit mm offsets and independent law domains. Public tests check unit rejection, field preservation and positive retained-cap/full volume evidence, in addition to direct affine refinement/invalid-axis/input-preservation checks. The rerun against both new WASM packages is active (session 35154). Live viewport/Solid qualification is still pending; authored frame/guide miter laws and complete continuousBound remain open. Logs: sweep-affine-miter-rush-public.log, sweep-affine-miter-rush-types.log, sweep-affine-miter-language-build.log and sweep-affine-generator-build.log.

The regenerated runtime schema and rebuilt language WASM now accept the Rush affine example. Eleven public tests pass, including the new direct affine WASM and Rush hollow-body checks: correct LENGTH/SCALAR units, unchanged inputs, retained wall/cap correspondence and solidGeometryCertified=true. The existing heavy closed Rush streamed test exceeded its 30-second timeout in that full run; an isolated rerun with a 60-second observation limit passes in 26.86 seconds. This is a test timeout observation, not evidence of a measured causal performance regression or complete green suite.

The scalar-only interval station arithmetic is restored verbatim behind the exact identity-axis/zero-center branch, avoiding added affine arithmetic and enclosure widening for old calls. All 38 native progressive-miter tests pass after that change. A new geometry WASM build is active (session 89654), and public regression on this final package remains pending. Logs: sweep-affine-miter-existing-rush-isolated.log, sweep-affine-scalar-path-native.log and sweep-affine-scalar-path-build.log. Live viewport and Solid tests of the new affine Rush example are not yet performed.

### Final affine package and nine-case STEP qualification (2026-10-02)

The scalar-path geometry WASM rebuild completed (11854052 to 10536798 optimized bytes). All twelve public progressive-miter tests now pass with the normal test timeout, including the prior heavy closed streamed case. The new Rush affine test requests the display artifact and runs actual Solid admission on geometry decoded from that artifact, with solidGeometryCertified=true; report definitions' diagnostic kind field is not sent to the native B-rep validator. TypeScript and targeted whitespace checks pass. Log: sweep-affine-final-public.log (12 tests, 26.78 seconds overall), sweep-affine-final-types.log.

The independent STEP matrix is now nine fresh passing cases in external-step-rush-affine. The ninth is generated directly from examples/rush/miter-affine-hollow.r, records the source SHA-256 and complete construction evidence, and requires a recomputed positive native Solid certificate before export. Its analytic volume is pi*(0.5^2-0.2^2)*10*1.5: the x-axis scale varies linearly from 1 to 2 while the y-axis scale is constant, and local center translation does not change cross-sectional area. OCCT verifies ten faces, one shell, two hole-bearing caps, original edge/wall data and coedge topology, and volume relative error 1.562e-10. The exact Fraction cap-planarity oracle also agrees with retained-cap evidence nested in construction reports. Logs: sweep-rush-affine-step-export.log and sweep-rush-affine-step-occt.log. Authoritative manifests/results: external-step-rush-affine/manifest.json and external-step-rush-affine/opencascade-sweep.json. Live viewport and actual UI Solid actions for this affine source remain unverified; frame/guide miter laws and complete continuousBound remain open.

### Closed affine miter qualification (2026-10-02)

A native and public WASM cyclic affine case now uses a square closed path with independently parameterized quadratic axis and center laws. At eight subdivisions it passes the affine whole-level error and retained-wall regularity checks; the final stored section equals the first. Perturbing the native final retained section by 0.125 is accounted for by both whole-level and final endpoint-contour bounds. Mismatched endpoints of either XYZ law are refused, as is a discontinuous center with excessive interior multiplicity. The public report explicitly preserves seamContinuity=C0 and continuousBound=false, with no smooth cyclic-seam promotion. Inputs remain unchanged. The focused public cyclic test, all 39 native progressive-miter tests, TypeScript and targeted whitespace checks pass. Logs: sweep-affine-closed-native.log, sweep-affine-closed-public.log, sweep-affine-closed-regression-native.log and sweep-affine-closed-types.log. This is a regular closed surface-family test with an open line profile; it does not qualify an affine closed hollow Solid or a live UI action. Those and authored frame/guide modes remain pending.

### Closed hollow affine nesting and Solid admission (2026-10-02)

Fresh native boundary-embedding certificates now permit shell nesting to reuse explicitly disjoint proofs for every cross-shell face pair. Exact witnesses are original clamped edge endpoints; missing pair evidence refuses fallback even when an aggregate flag remains positive. Numerical separation lower bounds remain zero where no quantitative clearance was proved. All six native volume-validity tests and three shell-nesting tests pass; TypeScript and targeted whitespace checks pass. Geometry WASM rebuilt successfully (11857172 to 10539423 bytes). The focused public closed hollow affine test passes against this package, proving two-shell parent/role classification, material orientation and actual Solid admission; flipped outer orientation is refused. Logs: sweep-nesting-reuse-volume-native.log, sweep-nesting-boundary-reuse-native.log, sweep-nesting-reuse-build.log and sweep-affine-closed-hollow-public.log. Broader public regression, closed affine STEP export and live UI qualification remain pending. Full authored continuousBound remains false.

### Ten-case STEP and nesting public regression (2026-10-02)

All 25 public tests in nurbsProgressiveMiter, nurbsSweepEmbedding and nurbsMiterSweep pass against the latest geometry WASM after nesting certificate reuse. The STEP exporter now includes closed-authored-affine-hollow-miter: constant positive anisotropic axes [2,1,1], center [0.125,0,0], independent law domains, 32 faces and two oriented shells, with required fresh native Solid admission. Its independent analytic material volume is pi*(0.5^2-0.2^2)*40*2. A fresh ten-case export and OpenCascade verification pass in external-step-closed-affine, including native/external material agreement and inward cavity orientation. The closed affine volume relative error is 2.3424485684675473e-10. Logs: sweep-nesting-reuse-public-regression.log, sweep-closed-affine-step-export.log and sweep-closed-affine-step-occt.log. This qualifies the ten explicit cases; full mode combinations, authored frame/guide miter, full continuousBound and live affine UI remain open.

### Authored miter frame continuous-jet foundation (2026-10-02)

The new progressive_miter::authored_frame_certificate builds an interval right-handed frame from original rational longitudinal/transverse laws: t=unit(longitudinal), b=unit(t cross transverse), n=b cross t. It encloses value and first/second derivatives with respect to normalized traversal, scaling each law by its own independent knot-domain span. Interval product, cross, norm and quotient derivative rules cover the complete requested traversal interval; no samples are used by the certificate. Both vector laws share one component-cell budget. A nonzero longitudinal axis and nonparallel transverse direction must be proved before any frame is returned; insufficient work or singular/parallel directions discard partial results. Tests independently compare analytic rotating-frame jets, including nonuniform rational weights and all three frame axes, and verify budgets 0/3/5 versus required6. All 42 native progressive-miter tests pass and targeted whitespace checks pass. Log: sweep-authored-frame-jets-native.log. This is a frame-law foundation only: authored-frame station generation, frame-dependent miter projection/remainder, guide frames, transport/Rush/UI integration and full continuousBound remain open. No public sweep guarantee is promoted.

### Authored frame twist and path-miter station composition (2026-10-02)

certify_twisted encloses the rotation of the authored frame with complete first/second product and trigonometric chain rules. Twist retains its own knot domain and shares one cell budget with both vector laws; twist exhaustion discards all partial frame axes. An independent nonuniform rational twist oracle verifies all three axes and their normalized-traversal derivatives, including budget7 success versus budget6 refusal. frame_certificate::Report::station_local_authored_frame now composes this frame, positive scalar scale, local planar profile offsets, original path-miter projection and path translation. All four laws share one budget (8 cells in the analytic test, refusal at7). A frame whose longitudinal axis differs from the path verifies that miter projection uses the path tangent/planes; nonplanar local offsets refuse. All 44 native progressive-miter tests and targeted whitespace checks pass. Logs: sweep-authored-frame-twist-native.log and sweep-authored-frame-station-native.log. This station enclosure is not yet wired into actual authored-frame generation or its interpolation remainder; affine/frame combined offsets, guides, transport/Rush/viewport/Solid and complete continuousBound remain open.

### Authored-frame continuous miter interpolation remainder (2026-10-02)

frame_certificate::Report::interpolation_upper_authored_frame now composes the authored-frame value/first/second jets, positive scalar scale and complete path-miter projection. Normalized-traversal derivatives are scaled by the outward edge/total-length ratio and subinterval width; scalar scale additionally retains its authored knot-domain rate. The projection field uses zero for an absent endpoint plane and h/(path_tangent dot h) otherwise, including axial offsets created when an authored frame differs from the path. The bound includes the mixed derivative of projection times moving offset. Smooth single-span laws use the second-derivative remainder divided by8; any crossed frame/twist/scale knot uses the Lipschitz remainder divided by2. All laws retain the shared station budget. Analytic tests cover sine/cosine midpoint error, refinement rate, a piecewise-linear twist derivative jump and the axial scale/projection product with actual error1/8 bounded conservatively by sqrt(2)/8. All 45 native progressive-miter tests and targeted whitespace checks pass. Log: sweep-authored-frame-remainder-native.log. Actual authored-frame generator and whole-level/endpoint integration, affine/frame combined laws, guide support and Rush/viewport/Solid remain pending; complete continuousBound is not promoted.

### Authored-frame native generator and whole-level integration (2026-10-02)

Sweep::with_frame_laws now validates continuous XYZ authored longitudinal/transverse laws, checks closed endpoint matching and replaces transported orientation during actual station generation. Twist rotates around the authored longitudinal axis; polyline miter projection retains the path tangent/planes. certify_level uses the same authored laws for interval remainders and stored endpoint comparisons, including endpointContourErrorUpper. Original profile-local coefficients are projected through the certified initial path frame as intervals, accounting for construction rounding rather than treating stored rounded offsets as ideal coefficients. authored_frames_applied records the native mode; scalar-only generation remains unchanged. Combined authored-frame/affine laws explicitly refuse in either setter order until their station/remainder composition is implemented.

The generated rotating-frame surface is refused at one subdivision and accepted at16, with retained wall regularity. An independent analytic oracle checks both entire line-profile interpolation and path traversal; a0.125 damaged final pole is covered by whole-level and endpoint-contour bounds. Exhausted law budget and a singular longitudinal axis refuse certification. A value-only twisted-frame certificate avoids derivative division on near-zero endpoint restrictions and conservatively charges all active spans of each component against one shared budget; point samples and budget6 refusal versus7 success are checked. All46 native progressive-miter tests and targeted whitespace checks pass. Log: sweep-authored-frame-generator-native.log. Native transport, WASM/public/Rush/viewport/Solid, affine-frame composition, guide support and full continuousBound remain unqualified.

### Authored-frame miter WASM, Rush and Solid-admission integration (2026-10-02)

All progressive-miter native transport operations now accept paired frame_axis/frame_normal curves, reject an incomplete pair and report authoredFramesApplied with interval-authored-frame-interpolation. TypeScript options use frameAxis/frameNormal and preserve independent vector-law domains. Rush lowering, strict schema, exported language schema/prompt and build arguments accept both dimensionless frame laws; runtime unit resolution rejects dimensioned frame values. The generated native artifact retains authored-frame and affine mode provenance alongside snapshot-local sweep evidence. Geometry WASM rebuilt successfully (11896233 to10576154 optimized bytes), and the language WASM rebuilt after schema regeneration.

All16 public progressive-miter tests pass against both final packages (33.81 seconds). The moving-frame line-profile test proves refinement acceptance, actual rotated section coordinates, retained-wall regularity and unchanged inputs; incomplete frame pairs and unsupported affine/frame combinations refuse. examples/rush/miter-authored-frame-hollow.r uses independent domains [2,5] and [7,9], a constant nontrivial normalized authored frame, and an actual hollow B-rep. The Rush test checks parsed fields and dimension refusal, positive retained caps/full native volume, and Solid admission on geometry decoded from the actual native display artifact. TypeScript and targeted whitespace checks pass. Logs: sweep-authored-frame-transport-native.log, sweep-authored-frame-transport-build.log, sweep-authored-frame-language-build.log, sweep-authored-frame-public.log and sweep-authored-frame-final-types.log. This proves the programmatic Solid gate, not live viewport/UI actions. Authored-frame STEP and cyclic/global mode matrix, combined affine/frame laws, guides, full continuousBound and broad UI qualification remain open.

### Combined authored frame and affine miter implementation (2026-10-02)

Removed the mutual exclusion only after both generation and whole-level certificates gained combined composition. Local offsets use scale times anisotropic axes plus center, followed by authored frame/twist and the original path-miter projection; axial center uses the authored longitudinal axis. Both setter orders produce identical stored geometry. Station value certificates share one budget across frame/twist, scalar scale and XYZ axes/center, with point-safe original-coefficient values. Remainders include complete product-rule mixed derivatives of frame, scale, axes and center with independent authored knot domains; crossing any participating knot uses the Lipschitz branch. Original interval profile-local projection and endpoint comparisons remain in the whole-level certificate.

All47 native progressive-miter tests pass. A moving-frame/linear-axis/quadratic-center analytic oracle verifies actual entire-profile interpolation, refinement acceptance and endpoint damage, and law budget13 refuses when14 cells are required. TypeScript and targeted whitespace checks pass. Logs: sweep-combined-frame-affine-native.log and sweep-combined-frame-affine-types.log. Public moving-law regression and examples/rush/miter-combined-frame-affine-hollow.r with actual Solid-admission regression are prepared. Geometry WASM rebuild is active (session68089); public combined mode is not yet qualified. Guides, complete continuousBound and full STEP/UI coverage remain open.

### Combined affine/frame public and twelve-case STEP qualification (2026-10-02)

Geometry WASM rebuild completed (11906980 to10587233 optimized bytes). All18 public progressive-miter tests pass (50.16 seconds), including moving combined affine/frame laws and the actual combined hollow Rush artifact through native Solid admission. Final TypeScript checks pass. The STEP exporter now adds original Rush-authored frame and combined frame/affine fixtures, recording source hashes and construction reports, with required fresh native Solid certification. Their independent analytic volumes use area multipliers1 and2 respectively; center translation does not change cross-sectional area. All12 fresh exports and OpenCascade checks pass in external-step-combined-frame-affine, including geometry/control-net agreement, topology, caps, shell orientation and analytic volumes. New cases have positive native volume certification. Logs: sweep-combined-frame-affine-public.log, sweep-combined-frame-affine-final-types.log, sweep-combined-frame-step-export.log and sweep-combined-frame-step-occt.log. A temporary live UI tab confirmed the preexisting source/scene baseline and was closed with original source and panel visibility restored; new-mode UI actions are still unqualified. Guides, full continuousBound, all-mode global/smoothness guarantees and complete UI matrix remain open.

### Guide-frame continuous jet and point-value foundation (2026-10-02)

The authored-frame certificate now supports orientation toward an original rational spatial rail minus the polyline path, projected perpendicular to the constant edge tangent. Guide derivatives retain independent knot-domain scaling and subtract the outward normalized-traversal path velocity; the edge path acceleration is zero. Frame normalization, cross products and twist reuse the same derivative rules and shared budget. The analytic rail (1,f,10f) over path (0,0,10f) verifies normalized frame values and both derivatives over the whole interval. A rail crossing the path axis refuses transverse-direction certification, and guide+twist budget3 refuses where4 cells are required. All48 native progressive-miter tests passed this jet integration.

A separate point-safe guide value certificate is then added for actual stored station comparisons, using conservative active-span charging and no derivative division on point restrictions. The added endpoint/interior analytic value regression is running on confirmed live session18516 (cargo/test PID18539/18595 observed). Log: sweep-guide-frame-jets-native.log. Guide station/remainder/generator integration and native/WASM/Rush/viewport/Solid are still pending; no sweep guarantee is promoted from this frame-only foundation.

The point-safe guide value regression subsequently completed successfully: all48 progressive-miter native tests pass, including endpoint/interior rail directions.

### Guide miter generator, remainder and transport integration (2026-10-02)

Sweep::with_orientation_guide validates continuous XYZ rails and closed endpoint agreement, replacing transported orientation with the normalized transverse projection of rail minus the polyline path. Twist rotates around the edge tangent and affine center/axes remain local to this guide frame. Authored frame and orientation guide refuse in both setter orders as conflicting orientation sources. Shared FrameSource machinery now routes station values and interpolation remainders to authored or guide certificates. The guide certificate receives outward path position and normalized-traversal velocity derived from original sites and certified chord lengths; whole-level and endpoint comparisons retain original profile-local enclosures. Guide+affine shares the same mixed-product remainder and all-law budget.

All49 native progressive-miter tests pass, including actual guide+affine surface interpolation against an independent analytic oracle, refinement acceptance, either setter order, budget exhaustion and endpoint damage. Native transport and TypeScript expose orientationGuideApplied and interval-guide-frame-interpolation. Rush lowering/schema/build arguments accept orientation_guide; snapshot-local artifact evidence preserves the mode. TypeScript and targeted whitespace checks pass. examples/rush/miter-guide-affine-hollow.r and a public guide+affine regression are prepared. Geometry WASM build is confirmed active on session34735; language WASM build is active on72018. Public and live qualification remain pending. Contact fitting, general spatial corner/closed guide combinations, full continuousBound and complete STEP/UI/global/smoothness matrices remain open. Logs: sweep-guide-generator-native.log, sweep-guide-generator-types.log, sweep-guide-rush-types.log, sweep-guide-generator-build.log and sweep-guide-language-build.log.

### Guide public and STEP matrix preparation (2026-10-02)

The language WASM build completed (2452466 to2268308 optimized bytes); final TypeScript checks pass. Public tests now include a Rush guide+affine body through actual Solid admission, original field preservation, and explicit refusal when a rail crosses the path axis. The independent STEP exporter includes separate Rush guide and guide+affine hollow fixtures, each requiring fresh positive native volume certification; analytic area multipliers are1 and2. examples/rush/miter-guide-hollow.r complements the existing affine guide source. Geometry packaging remains confirmed active on session34735, with native wasm-opt PID23459 observed running; public and STEP reruns await the packaged kernel. Logs: sweep-guide-language-build.log, sweep-guide-final-types.log and sweep-guide-generator-build.log.

### Guide public qualification revealed Rush dependency omission (2026-10-02)

Geometry WASM packaging completed (11917840 to10595980 optimized bytes). The full public progressive-miter run passes19 tests, including direct guide+affine generation and rail-crossing refusal, but the guide Rush Solid test fails at compilation with Every node must be reachable from root. Runtime dependency traversal and selected-node rewriting recognized orientation_guide only on progressive_sweep/brep_progressive_sweep. Both paths now include brep_progressive_miter_sweep, preserving guide references through reachability and selection. The language WASM rebuild is active on session48128; the Rush/public rerun and expanded14-case STEP matrix remain pending. Log: sweep-guide-public.log and sweep-guide-language-reference-build.log. This is a corrected integration defect, not a positive full-guide qualification.

### Guide Rush dependency correction publicly verified (2026-10-02)

The corrected language WASM rebuild completed, and all20 public progressive-miter tests pass (49.20 seconds). This includes the previously failing guide+affine Rush hollow body through actual native artifact Solid admission. A separate compile observation confirms a five-node graph with rootn5 and rail referencen3 retained. Direct guide+affine surface checks and rail-axis crossing refusal also pass. Logs: sweep-guide-public-final.log and sweep-guide-rush-reachability.log. The fresh14-case STEP export is active on session47734, targeting external-step-guide; external verification remains pending. Full continuousBound, contact guide fitting, general spatial/cyclic guide modes, global smoothness guarantees and live UI matrix remain open.

### Fourteen-case guide STEP qualification (2026-10-02)

All14 fresh STEP exports and independent OpenCascade checks pass in external-step-guide. The two added Rush orientation-guide fixtures (guide alone and guide+affine) each require recomputed positive native Solid certification, preserve the rail source hash/construction evidence, and agree with analytic material volume, topology, hole-bearing caps, original control geometry and shell orientation. Both have relative volume error approximately1.562e-10. Logs: sweep-guide-step-export.log and sweep-guide-step-occt.log; authoritative results are external-step-guide/manifest.json and opencascade-sweep.json. These are14 explicit fixtures, not the full spatial/cyclic/contact/global smoothness matrix. Full continuousBound and live UI actions remain open.

### Live wide guide+affine UI qualification (2026-10-02)

An isolated background IAB tab tests examples/rush/miter-guide-affine-hollow.r through the real editor/worker/viewport. Build completes with336 triangles, displayed tessellated volume12.86 and area69.96; the UI separately shows body geometry proven and full continuous error unproved. These mesh metrics are not analytic volume proof. Actual To Solid succeeds and produces one selected B-rep in the existing model group. Undo restores the prior CAD body/selection. Cancel Build shows Build cancelled; Cancel To Solid returns without replacing CAD geometry. Changing the rail to cross the path axis marks the old preview as previous result; To Solid then refuses with Miter direction must be finite and nonzero and leaves CAD unchanged. Original source and Auto ON are restored, and original model rebuilding is in progress. Evidence images: sweep-guide-affine-live-wide.jpg, sweep-guide-affine-live-solid-wide.jpg and sweep-guide-affine-live-refusal-wide.jpg. Responsive narrow coverage and the remaining mode/UI matrix are still pending.

Restoration subsequently verified: original source builds2560 triangles with displayed volume1.35 and area42.34; Auto is ON, CAD has its original one group/one unselected B-rep, source panel is hidden, viewport reset and temporary tab closed.

### Live narrow guide+affine UI qualification (2026-10-02)

A temporary background IAB tab with explicit 640x800 viewport completes the guide+affine Rush editor/build/viewport/Solid path. Build:336 triangles, displayed mesh volume12.86, area69.96,917ms. Body geometry is proven while full continuous error remains explicitly unproved. Actual Solid creates one selected B-rep; the screenshot confirms visible geometry and usable wrapped toolbar. Undo restores the prior CAD body. Build cancellation reports cancellation; Solid cancellation preserves CAD state. Source replacement marks the prior result stale. A rail crossing the path axis refuses To Solid with Miter direction must be finite and nonzero; no CAD replacement occurs. Original source and Auto ON are restored; original build completes2560 triangles, volume1.35, area42.34,5034ms. Source is hidden, preview view reset, viewport override reset, temporary tab closed. Evidence: sweep-guide-affine-live-solid-narrow.jpg and sweep-guide-affine-live-refusal-narrow.jpg. This closes wide/narrow live checks for this guide+affine fixture, not the complete all-mode UI matrix. Full continuousBound and the previously listed geometric/global/smoothness work remain open.

### Progressive miter retained cap correction foundation (2026-10-02)

ProgressiveMiterOptions now accepts explicit capCorrection (quantum, tolerance, maxWork). Synchronous and streamed body builders share the existing ordinary-miter endpoint-plane projection, retain the original approximation unchanged, expose sectionCorrection separately, and run wall/cap/correspondence/chart/contact/embedding/volume audits on the corrected sections and actual retained model. Failed projection or inexact retained correspondence refuses construction. Closed paths refuse cap correction. Correction displacement bounds retained ruled walls relative to the original rounded section family only; authored full error and filled-cap-region transfer are still separate obligations. Endpoint planes currently follow endpoint path sites/tangents; frame/center axial offsets need authored ideal-plane composition before full error admission. No continuousBound promotion. Public tests cover hollow straight-path correction, original input preservation, synchronous/stream body and audit equality, and exhausted-work refusal. All27 ordinary/progressive miter public tests pass; vue-tsc and targeted whitespace checks pass. Logs: sweep-progressive-cap-public.log, sweep-progressive-cap-regression.log and sweep-progressive-cap-types.log. Rush/schema transport of correction and full continuous error composition remain pending.

### Progressive cap correction Rush integration (2026-10-02)

Strict progressive-miter schema and Rush lowering now accept cap_correction_tolerance/quantum/max_work. Existing runtime length resolution applies tolerance and quantum; kernel requires explicit tolerance when other correction controls are present and passes correction into sync/stream body assembly. Construction reports preserve sectionCorrection. Language WASM rebuild succeeds (2453993 to2269837 optimized bytes); vue-tsc and targeted whitespace checks pass. A real Rush public test compiles a guide hollow body with correction, checks correction evidence and exact retained correspondence, parses the source into viewport scene meshes and admits one body into Solid. Exhausted work and missing displacement tolerance refuse. Logs: sweep-progressive-cap-language.log, sweep-progressive-cap-rush-types.log, sweep-progressive-cap-rush-public.log. Full continuousBound and authored frame/center cap-plane composition, cap interior error and live corrected UI/STEP qualification remain open.

### Eighteen-case corrected progressive STEP qualification (2026-10-02)

Four explicit Rush fixtures now exercise progressive cap correction: guide, guide+affine, authored frame and frame+affine. All18 fresh exports pass independent OpenCascade checks in external-step-progressive-correction, including original control geometry, topology, analytic material volume, cap hole ownership, full-domain retained edge/wall distance checks and shell orientation. The four new cases recompute positive native Solid volume evidence and preserve source hashes/construction correction reports. Guide/frame-only correction displacement is2.5724002394829305e-13 mm; combined affine cases bound0.25000000000000017 mm, explicitly projecting a0.25 mm axial center offset to endpoint path planes with0.3 mm correction tolerance. This is an intentional bounded geometric change, not an authored full error proof. Manifest inspection confirms exact retained correspondence and two corrected endpoint sections for every added case. Exporter now requires this evidence for corrected Rush fixtures. Logs: sweep-progressive-cap-step-export.log and sweep-progressive-cap-step-occt.log. These18 fixtures do not close general spatial/cyclic/contact/full continuousBound/G1/G2 or the remaining live UI matrix.

### Corrected progressive wall error composition (2026-10-02)

Both body builders now expose retainedWallErrorUpper only after exact retained correspondence. The certified original whole-wall approximation upper and correction wall displacement upper are added with an outward binary64 successor; zero contributions are preserved exactly, invalid/nonfinite/overflow input refuses. Corrected bodies must fit the composed bound within max_deviation, rather than accepting the original approximation alone. Actual corrected retained-wall chart regularity is required and construction/viewport artifact evidence uses the retained chart audit; snapshot evidence preserves the composed upper and sectionCorrection separately from the original bound. Filled-cap interiors remain unproved, so continuousBound remains false. Combined affine corrected examples now request max_deviation0.3 mm to accommodate their explicit0.25 mm correction;0.01 mm is tested to refuse.30 regression tests pass; final focused sync/stream/Rush/artifact/refusal tests pass after the last chart/evidence changes, as do vue-tsc and whitespace checks. Fresh18-case STEP export and independent OpenCascade verification pass in external-step-progressive-composition. Logs: sweep-corrected-wall-composition-types.log, -public.log, -regression.log, -final.log, -step-export.log and -step-occt.log. Full cap-region transfer and general all-mode proofs remain open.

### Corrected filled-cap admission prerequisite (2026-10-02)

Corrected progressive bodies now require a fresh exact filled retained-cap region certificate in both synchronous and streamed construction. The certificate checks original retained boundary coefficients, planar control hull, trim chart and region validity with shared endpoint exact-work accounting. Exhausted cap audit work refuses construction rather than returning a corrected body with unresolved filled regions. Focused public tests verify both sync and stream refusal with maxExactWork0, successful correction, Rush/Solid transport and composed-wall error refusal; all3 pass. vue-tsc and whitespace checks pass. Logs: sweep-corrected-filled-cap-types.log and sweep-corrected-filled-cap-public.log. This proves validity/equality to corrected endpoint contours only; authored ideal-plane, domain transfer and filled-cap displacement remain separate open obligations, and continuousBound remains false.

### General planar filled-cap transfer lemma (2026-10-02)

The earlier same-plane winding lemma now extends to distinct planes with nonsingular orthogonal projection. Boundary pairing within epsilon bounds all interior heights by epsilon (affine height extrema occur on the boundary), projected material regions retain ownership under an affine homeomorphism, and winding transfer bounds their planar Hausdorff distance by epsilon. Orthogonal decomposition gives full3D Hausdorff error at most sqrt(2)*epsilon. Proof and exact runtime prerequisites are in sweep-filled-cap-transfer.md. This removes the mathematical requirement for coincident ideal/retained planes, but not the runtime obligations: ideal cap/profile ownership and a certified nonzero plane-normal dot product remain unimplemented. No continuousBound promotion.

### Native cap-plane projection prerequisite (2026-10-02)

progressive_miter/cap_projection_certificate.rs now conditionally certifies nonsingular orthogonal projection from original ideal/retained normal interval enclosures. Outward dot-product bounds must separate from zero; nonzero dot also proves both normals nonzero. Positive/negative separation records preserved/reflected orientation respectively; zero/uncertain normals and zero cell budget return no positive certificate. Reflections require complete boundary orientation reversal before using the filled-cap theorem. Three native tests cover parallel/oblique/reflected, perpendicular/uncertain/zero-budget, near-perpendicular scaled analytic enclosure and malformed intervals. All52 progressive-miter library tests pass with cargo test --locked --manifest-path crates/Cargo.toml -p nurbs-core --lib progressive_miter; authoritative terminal log sweep-cap-projection-lib.log. Earlier broader filtered Cargo invocations passed their unit tests but were terminated during unrelated integration-binary processing; no full integration-suite claim. Native module and whitespace checks pass. Original endpoint frame-to-normal construction, retained plane normals, ideal material ownership and WASM transport of this prerequisite remain pending; no full continuousBound promotion.

### Original ideal endpoint cap normals (2026-10-02)

Sweep::certify_endpoint_cap_normals derives both open ideal endpoint plane normals from original polyline transport and original authored frame/guide laws. Ordinary normals use certified original path tangents; authored normals use certified normalized longitudinal axes with transverse/twist validation; guide normals use original endpoint rail minus exact endpoint site and certified path tangents. Open endpoints have no miter plane projection. Positive affine scales/axes preserve the plane normal and local center translates it only; a combined affine authored-frame regression verifies invariance. One shared budget charges original transport plus both endpoint frame certificates; unresolved second endpoint returns no partial normal pair. Closed paths return no cap result. Native tests connect tilted authored normals to the projection prerequisite, independently enclose analytic1/sqrt(2), test guide modes, zero/insufficient shared budget and affine/center invariance. All53 progressive-miter library tests pass; log sweep-cap-endpoint-native.log. Both new certificate modules are formatted and whitespace checks pass. Retained original plane normal extraction, ideal local material ownership and WASM/whole-cap transfer remain pending; continuousBound stays false.

### Original retained cap planes and integrated endpoint projection (2026-10-02)

Exact original-control retained plane auditing now lives in nurbs-core progressive_miter/cap_retained_plane_certificate.rs and is reused by brep-core cap contacts with the same consumed exact-work accounting. Original corner vectors form an outward unnormalized plane-normal enclosure only after exact noncollinearity and all-pole orient3d coplanarity. Warped one-ULP poles, degenerate planes and exhausted exact work provide no normal. Sweep::certify_endpoint_cap_projection joins original ideal normals with two retained surface plane normals, proves both projection determinants nonzero, records reflection, and shares frame-cell/projection and exact-plane budgets across both endpoints without partial positive pairs. This remains conditional on cap ownership and parameterwise pairing, which it does not inspect. All55 progressive-miter library tests pass, including oblique analytic cross-product enclosure, ideal/retained projection, original inputs, shared second-endpoint cell/exact-work refusals. Both brep-core cap-contact tests pass, including corrected spatial cap exclusion and prior oblique exact-planarity guards. Logs: sweep-retained-plane-core.log, sweep-retained-plane-native.log and sweep-endpoint-projection-integrated.log. New module formatted and whitespace checks pass. WASM/TypeScript transport, ideal local material ownership and complete cap error transfer remain pending; no continuousBound promotion.

### Endpoint projection transport and body evidence integration (2026-10-02)

Native dispatch now accepts curve_progressive_miter_cap_projection with exactly two retained cap surfaces and shared frame-cell/exact-plane budgets, preserving normalDots, reversesOrientation, work, refusal reason and continuousBound:false. TypeScript exposes inspectProgressiveMiterCapProjection and separate capProjectionBudgets. Synchronous/stream progressive bodies audit both original retained cap surfaces; Rush construction and snapshot-local viewport evidence retain this conditional projection report without promoting full error. Native JSON dispatch and all55 progressive-miter library tests pass; vue-tsc and whitespace checks pass. Public regression is prepared for worker transport, shared second-cap work/cell refusal, warped cap rejection, input preservation, body evidence and artifact propagation. Geometry WASM build is confirmed active on session12250, PID53962, cwd repository; wasm-opt child55232 is active. A separate older build PID51002 belongs to /private/tmp/open-scad-viewer-cad-resume and was preserved. Public/WASM qualification remains pending until this build completes. Logs: sweep-cap-projection-transport-native.log, -types.log and -build.log. Full ideal-region topology/cap transfer and remaining objective scope stay open.

### Endpoint projection JSON boundary qualification (2026-10-02)

Native JSON transport tests now verify one-cap cardinality refusal, negative exact-work rejection, maxCells above100000 rejection and zero-cell-budget refusal without partial positive normalDots. All55 progressive-miter library tests pass; log sweep-cap-projection-dispatch-boundaries.log. The existing repository geometry WASM build remains confirmed live on session12250 with wasm-opt PID55232 using approximately100 percent CPU after4m19s. It was not restarted; public prepared tests remain pending against the packaged kernel.

### Endpoint projection packaged WASM/public qualification (2026-10-02)

The original geometry build session12250 completed successfully; wasm-opt reduces11923554 to10601377 bytes and packaging completes. Public regression session2061 runs against the new packaged kernel and succeeds across progressive/ordinary miter and outward error composition. This includes endpoint projection worker transport, shared cell/exact-work refusal at the second cap, warped cap refusal, original input preservation, body diagnostic equality and snapshot-local Rush artifact evidence. vue-tsc and targeted whitespace checks pass. Authoritative logs: sweep-cap-projection-transport-build.log, sweep-cap-projection-public-regression.log and sweep-cap-projection-public-types.log. The report remains a projection prerequisite only: no ideal region ownership or filled-cap error has been fabricated; full continuousBound and other goal requirements remain open.

### Exact original profile-to-local material domain (2026-10-02)

Sweep::certify_local_profile_domain now proves original material ownership transferred into the ideal initial local plane. Proposed source-plane anchors are checked by exact noncollinearity/all-pole coplanarity predicates; the shared retained-plane utility accepts arbitrary original point arrays and retains the existing surface wrapper. A separated original normal component permits exact coordinate-drop projection by copying poles/weights, with no rounded UV fit. A nonzero original-source-normal versus certified initial-tangent dot proves the local projection is an affine homeomorphism. Existing continuous contour simplicity/disjointness/nesting audits then establish local outer/hole material ownership. Transport/projection/contour cells share one budget; exact source-plane work and pair limits are explicit. Huge loop sizes are rejected before summation. All56 progressive-miter library tests pass: coordinate and oblique hollow domains, budget exhaustion, outside hole, self-crossing contour, original input preservation, and tiny source warp accepted by constructor tolerance but refused by exact plane proof. Both brep-core cap-contact tests pass after plane-utility reuse. Logs: sweep-ideal-profile-domain-native.log and sweep-ideal-profile-plane-contact-regression.log. New native module formatted and whitespace checks pass. This is local-domain geometry, not shell orientation or full endpoint region/error; endpoint-map transfer, transport/WASM integration and complete continuousBound remain pending.

### Ideal endpoint cap material-domain transfer (2026-10-02)

Sweep::certify_ideal_cap_domains combines certified original-to-local material ownership with both certified endpoint frames. Strictly positive original rational scale/axis controls and positive weights prove transverse scaling is nonsingular; orthonormal certified endpoint frames, twist and center translation preserve the material domain, including holes. Open endpoints have no miter shear. The shared cell budget conservatively charges local transport/contour/projection and endpoint frame work; failed second-stage proof returns no endpoint normal pair or positive ideal_cap_domains_certified. All57 progressive-miter library tests pass, covering affine-only, moving authored-frame+affine, guide+affine, shared work refusal and singular frame with otherwise valid local domain. Log: sweep-ideal-endpoint-domain-native.log. Module formatted/whitespace checked. The filled-cap theorem now additionally records a per-loop winding-magnitude argument for one-outer/disjoint-hole domains, avoiding any invented global shell orientation guarantee. Native material ownership transfer is proven here; transport, filled-cap Hausdorff composition, full continuousBound and other goal matrices remain pending.

### Ideal endpoint domain transport and filled-cap composition (2026-10-02)

Native dispatch accepts curve_progressive_miter_cap_domains with explicit loop partition and shared original frame/contour/cell/exact-work budgets. TypeScript inspectProgressiveMiterIdealCapDomains, synchronous/stream body reports, Rush construction and snapshot evidence preserve local/ideal ownership and endpoint normal results. Native JSON tests verify positive combined frame/affine material ownership, exhausted cell refusal and malformed loop partition rejection; all57 library tests pass. filledMiterCapErrorUpper composes endpoint contour error with bounded correction outward, then uses conservative2*epsilon for the proven distinct-plane sqrt(2)*epsilon filled-region transfer. Positive ideal material ownership, actual exact retained cap regions and nonzero plane projection dots are mandatory; missing premises/overflow return no partial cap pair. Both body paths expose filledCapErrorUpper; continuousBound remains false until the complete construction error audit is finished.2 composition unit tests and vue-tsc pass. Public regression is prepared with an independent corrected affine annulus interior sample and both Rush combined sources. New geometry WASM build is active on session69773, repository PID72815, wasm-opt73640; public runner session47829 automatically waits for its completion before executing the four focused suites. Logs: sweep-ideal-domain-transport-native.log, -types.log, -build.log, sweep-filled-cap-composition-types.log, -unit.log and pending sweep-ideal-domain-public-regression.log. Full global/smoothness/STEP/UI scope remains open.
### Filled-cap packaged qualification and boundary error union (2026-10-02)

The geometry WASM build recorded in sweep-ideal-domain-transport-build.log completed successfully: wasm-opt reduces11931990 to10609091 bytes. The initial public run found a test sampling bug: normalized cap sample parameters were passed directly to a cap with physical knot domains. The test now maps the normalized sample into each actual active knot domain. The corrected independent annulus interior sample passes for both guide/affine and authored-frame/affine corrected Rush sources; this was a test repair, not a geometry change.

Both synchronous and streamed builders now expose boundaryErrorUpper as the maximum of the certified retained wall bound and both certified filled-cap bounds. Missing open-cap proof returns null; a closed path uses the complete retained-wall bound. This certifies only the union of boundary sets, without promoting solid-volume, shell-orientation or continuousBound flags. Rush construction reports and snapshot-local geometry evidence preserve the aggregate and boundaryErrorWithinBudget. The latter is false for both corrected affine examples: their conservative filled-cap bound is approximately0.5mm, above max_deviation0.3mm, even though the retained wall bound fits. A tighter proven parallel-plane transfer is still required to close that budget; the current report does not conceal this limitation.

All35 tests in the four focused progressive/ordinary miter and numerical error suites pass; log sweep-boundary-error-composition-public.log. After adding the explicit budget flag, the public ideal-domain/corrected interior regression passes again; log sweep-boundary-error-budget-public.log. The full continuousBound, broader global guarantees, smoothness and STEP/UI matrices remain unfinished.


### Outward sqrt(2) cap transfer and exact parallel-plane lemma (2026-10-02)

The general filled-cap bound now uses sqrt(2)*epsilon instead of2*epsilon. sqrtTwoCertifiedErrorUpper uses the exact binary64 rational6369051672525773/2^52, whose square exceeds2, and a successor after multiplication. It preserves zero and refuses invalid/overflowing bounds. A separate integer dyadic oracle verifies output squared is at least twice input squared for subnormal, normal and large inputs; no floating sqrt oracle is used. The corrected Rush interior regression still passes, with cap bounds below0.353554mm instead of approximately0.5mm. The0.3mm full boundary budget still fails and continuousBound remains false.

The transfer proof now establishes the sharper epsilon bound for exactly parallel translated planes: their constant height consumes part of each paired boundary error, leaving a matching tangential budget; planar winding transfer and Pythagoras recover epsilon for the complete material region. Runtime use requires exact ideal-axis/retained-plane parallelism, which remains pending; a nearly-unit interval normal dot is not substituted for this proof. Five selected tests across three suites, vue-tsc and targeted whitespace checks pass. Logs: sweep-filled-cap-sqrt2-public.log and sweep-filled-cap-sqrt2-types.log. Full objective remains active.


### Exact original endpoint-axis/retained-plane parallelism (2026-10-02)

cad-predicates exposes direction_dot3d over four original points, retaining exact coordinate differences and products through the existing binary64/interval/expansion stages. A cancellation regression proves a negative exact -2^-104 dot where rounded multiplication would give zero; exhausted work refuses. The full19 predicate unit tests pass.

The native retained-plane parallel audit first proves exact planarity and noncollinearity, then proves both independent retained plane directions orthogonal to the original axis. Exact oblique parallelism passes; a one-ULP axis tilt, warped cap, zero axis and insufficient shared work refuse. Sweep::certify_endpoint_cap_parallelism obtains the unnormalized authored endpoint axis directly from clamped rational end control points, or original path endpoint differences in ordinary/guide modes. Certified original endpoint frames remain mandatory. Both caps share one exact-work budget and failures return no partial pair. Tilted authored frames report a complete nonparallel pair; ordinary, clamped authored and guide cases report parallel pairs. All58 progressive-miter library tests pass after formatting the two certificate modules; targeted whitespace checks pass. Logs: sweep-cap-parallel-predicate-regression.log and sweep-cap-parallel-endpoint-native.log.

This closes the native parallelism prerequisite, but transport, packaged WASM and use of the epsilon cap bound are still pending. Existing runtime cap bounds remain sqrt(2)*epsilon and full continuousBound remains false. All broader global/smoothness/STEP/UI goal requirements remain active.


### Endpoint parallelism transport and sharper cap composition (2026-10-02)

Native curve_progressive_miter_cap_parallelism accepts exactly two retained cap surfaces and explicit frame-cell/exact-work budgets; it returns complete parallel flags or no pair, work, reason, method and continuousBound:false. JSON tests cover positive clamped authored endpoints, tilted nonparallel endpoints, shared second-cap exact-work exhaustion and cardinality rejection. All58 progressive-miter library tests pass; log sweep-cap-parallel-transport-native.log.

TypeScript exposes inspectProgressiveMiterCapParallelism. Both synchronous and streamed body builders inspect the actual retained caps; Rush construction and snapshot-local viewport evidence preserve capParallelism. filledMiterCapErrorUpper now uses the sharper epsilon bound only for endpoints with exact parallel=true, retaining the independent ownership/retained-region/projection/correction premises. Missing or nonparallel proof uses the general outward sqrt(2)*epsilon bound. Six numerical/cap composition tests pass, including mixed endpoint status and missing ideal ownership despite positive parallelism; log sweep-cap-parallel-composition-unit.log. Both vue-tsc runs and targeted whitespace checks pass.

Public regression is prepared for direct WASM parallel transport, shared frame/exact-work refusal, warped-cap rejection and input preservation. The independent corrected guide/affine and authored-frame/affine interior tests now require capParallelism [true,true], cap bounds below0.250001mm and boundaryErrorWithinBudget:true at0.3mm; these assertions are pending packaged qualification and are not claimed passing. Build session55020 is confirmed active, repository node PID97716 with optimizer child98643 using approximately99 percent CPU after1m15s. Release compilation already finished in1m38s; log sweep-cap-parallel-transport-build.log. Build is preserved, not restarted. Full continuousBound/global/smoothness/STEP/UI objective stays active.


### Boundary-budget STEP qualification gate and preserved active build (2026-10-02)

The independent STEP exporter now requires complete boundaryErrorWithinBudget:true, a finite boundary upper bound, both filled-cap bounds, exact positive endpoint parallelism and certified ideal endpoint material ownership for all four corrected Rush fixtures, in addition to retained correction and correspondence. This gate is prepared for the new packaged certificate and has not yet passed an export run. The OCCT interpreter remains available at /tmp/cad-roadmap-ocp/bin/python.

Build session55020 is still live: repository node97716 and optimizer child98643 are present after6m46s/5m07s, respectively. No rebuild was started. Public regression session8797 waits on that exact confirmed-live build PID before executing the four suites; its log is sweep-cap-parallel-public-regression.log. It refuses to start tests if the build log lacks an optimization result; the build session terminal result must still be checked independently before reporting packaged success. Targeted whitespace checks pass. Full continuousBound and the broader goal remain unachieved.


### Packaged parallel-cap boundary budget and renewed independent STEP qualification (2026-10-02)

Build session55020 completed successfully; wasm-opt reduces11941891 to10617736 bytes. Public session8797 completes with37 passing tests across four suites, against this packaged kernel. Coverage includes exact endpoint parallel transport, tiny authored-axis tilt returning nonparallel despite nearly aligned normals, shared second-cap frame/exact-work refusal, warped retained cap rejection, original input preservation, numerical outward composition and independent corrected cap interior samples. Both corrected affine Rush artifacts preserve capParallelism [true,true] and boundaryErrorWithinBudget:true. Authoritative logs: sweep-cap-parallel-transport-build.log, sweep-cap-parallel-public-regression.log and sweep-cap-parallel-public-types.log.

The strengthened STEP exporter passes all18 fixtures into external-step-parallel-cap-bound. All four corrected Rush fixtures pass the complete boundary-error qualification gate. Actual aggregate upper bounds: guide-only2.72212915786225e-13mm; authored-frame-only2.686932269450182e-13mm; guide/affine0.25000000000002254mm; authored-frame/affine0.2500000000000164mm. The latter two fit max_deviation0.3mm without weakening the budget or replacing the input geometry. Source hashes, original construction proof reports and exported geometry remain in manifest.json.

Independent OpenCascade verification completes successfully for all18 imported fixtures, including topology and analytic volume; opencascade-sweep.json passed:true. Logs: sweep-cap-parallel-step-export.log and sweep-cap-parallel-step-occt.log. This proves the selected matrix, not all geometric mode combinations; OCCT explicitly leaves surface/seam/containment qualification separate. Whitespace checks pass. The runtime still reports continuousBound:false pending the complete construction-certificate admission audit and broader declared modes. Global guarantees, moving-frame/multispan/cyclic smoothness and the complete STEP/UI goal matrices remain open.


### Viewport numerical boundary-budget presentation (2026-10-02)

Rush snapshot evidence now includes the original boundary error budget. The presentation-only reader preserves finite nonnegative boundary bounds and budgets, and only displays a positive/negative budget status when the boolean exactly agrees with the numerical comparison. Missing/malformed/inconsistent status remains unproved; this reader does not admit Solid geometry. App viewport status now displays the approximate boundary upper bound, budget and within/over/unproved result alongside the independent full continuousBound and regularity flags. It does not promote the full construction guarantee. Three selected viewport/public ideal-domain tests pass; log sweep-boundary-viewport-public.log. Whitespace checks pass. Current CUA inventory has no in-app browser tabs; live wide/narrow rendering of this new status is pending. Typecheck is recorded in sweep-boundary-viewport-types.log. All broader goal requirements stay active.


### Live frame/affine boundary status wide/narrow verification (2026-10-02)

The pending viewport vue-tsc run completes successfully. The local server responds200 at127.0.0.1:5202. A temporary CUA tab builds the actual corrected authored-frame/affine Rush fixture with explicit correction, holes and max_deviation0.3mm. The rendered status reads solid geometry certified, full continuous error unproved, profile/wall regularity certified, approximate boundary bound0.250000mm / budget0.300000mm within budget. Source editing immediately removes the old final evidence and marks the prior mesh stale; the successful build publishes the new snapshot status.

Both1280x900 and640x800 screenshots were viewed and saved as frame-affine-boundary-status-wide.png and frame-affine-boundary-status-narrow.png. The complete numerical status wraps legibly in both layouts. This checks viewport presentation, not a fresh Solid import or the complete UI scenario matrix. The original source, Auto enabled, Solid mode, hidden editor, original one unselected B-rep body/group and default viewport are restored through the UI. The test does not replace the original Solid group. Full continuousBound and all other outstanding goal scope remain active.


### Exact constant authored-axis endpoint identity without knot clamping (2026-10-02)

The native endpoint parallelism certificate now accepts identical original longitudinal poles under valid nonclamped rational knot domains and unequal positive weights. Rational basis normalization preserves that exact original vector throughout the active domain; endpoint direction is therefore obtained without rounded evaluation. Existing certified endpoint frame/nonzero-axis, actual retained exact-plane and shared exact-work conditions remain mandatory. Nonconstant nonclamped encodings still refuse this sharper certificate rather than inventing an original endpoint axis. General cap transfer remains available through the separate nonsingular projection certificate.

The native regression uses nonclamped knots[1,2,5,6], identical poles[0,0,2] and unequal weights[0.5,3], confirms valid endpoint normals and both exact parallel flags, then changes the second pole by epsilon and verifies original-endpoint-axis-unproved with no positive pair. All58 progressive-miter library tests pass; log sweep-cap-parallel-constant-native.log. Public WASM regression is prepared for the same identity/refusal and awaits packaging. vue-tsc and targeted whitespace checks pass; log sweep-cap-parallel-constant-types.log. Geometry build session55779 is confirmed live on repository node18729; log sweep-cap-parallel-constant-build.log. No packaged qualification is claimed yet. Full objective remains active.


### Constant-axis packaged qualification and original-span decomposition error (2026-10-02)

Geometry build session55779 completes successfully, optimizing11942078 to10617926 bytes. Public runner4691 reports39 passing tests across five suites, including nonclamped constant rational axis identity with unequal weights and refusal after a nonconstant control perturbation. Logs: sweep-cap-parallel-constant-build.log and sweep-cap-parallel-constant-public.log.

The full continuousBound admission audit identifies an actual remaining decomposition gap: inspectSweepRetainedCorrespondence only proves zero extra error for already-segmented Bezier sections, refusing ordinary source NURBS with lower interior knot multiplicity. A new native curve_decomposition_certificate independently bounds one original NURBS knot span versus the actual retained rational Bezier. Original interval blossom controls preserve source coordinates/weights and every coefficient operation rounds outward. Correlated homogeneous Bernstein cross products bound the rational difference over the entire normalized span; positive denominator lower bounds and complete product-pair budgets are mandatory, with no partial upper on refusal. Degrees through25 are supported; periodic encodings still need a separate route.

Two native tests pass: nonuniform rational multi-span extraction, exact independent0.125 translation, source input preservation, insufficient product budget, invalid source span, and degree25 with complete676 versus insufficient675 product-pair budget. Log sweep-decomposition-native.log. The first compile attempt used private Segment fields in the test; it was corrected to the public definition accessor before successful qualification. Module formatted and targeted whitespace checks pass. Proof and wall/cap transfer obligations are recorded in sweep-decomposition-error.md. This new decomposition contribution is native-only: transport, binding all retained faces/endpoints, shared aggregate budgets and body error composition remain pending. It was added after release compilation of the constant-axis package and is not claimed exposed by that packaged transport. Global/smoothness/STEP/UI objective remains active.


### Decomposition error transport and shared complete-span budget (2026-10-02)

Native dispatch now exposes curve_decomposition_audit with original curve, source knot-span index, actual retained Bezier and maxProducts. It preserves errorUpper, products, reason and conditional continuousBound:false. Native JSON tests verify a small rational decomposition bound, insufficient pair budget with no upper, and negative budget rejection. Both decomposition native tests pass; log sweep-decomposition-transport-native.log.

TypeScript exposes inspectNurbsDecomposition and inspectNurbsDecompositionBatch. The batch maximum uses one shared product-pair budget across at most2048 correspondences; an unproved last pair invalidates the whole upper rather than returning a partial maximum. Empty/invalid/excessive budgets refuse. Public tests are prepared for both original rational spans, independent0.125mm translation, source preservation, invalid span/budgets, complete18 versus insufficient17 shared products and a failed final pair. vue-tsc and targeted whitespace checks pass; log sweep-decomposition-transport-types.log.

Geometry build session60326 is confirmed active on repository node25939; release compilation finished in1m08s and optimization is pending. Public runner13727 waits for that exact live build before six-suite qualification; log sweep-decomposition-public.log. Packaging and public results are not yet claimed. Actual retained face binding, endpoint decomposition/cap transfer, complete aggregate admission and all broader goal requirements remain pending.


### Actual retained-wall decomposition binding and body error composition (2026-10-02)

inspectSweepRetainedDecomposition now binds every actual ruled face to both original section knot spans. It checks all profile/loop partitions, unchanged original degree/knots/weights between stations, complete wall face coverage, actual Bezier U / linear V layouts, finite controls and equal positive weights along V. Each face is therefore a linear blend of its endpoint retained curves; the complete shared-budget endpoint decomposition maximum bounds its whole wall. Missing faces, unequal V weights, source correspondence or a failed final native span yield no wall upper.

Both synchronous and streamed progressive miter body paths use this numeric certificate when exact retained Bezier correspondence is unavailable. The decomposition contribution is added outward to original interpolation/station and correction error, with explicit retainedDecompositionBudgets. Numeric correspondence remains distinct from retainedCorrespondence.exact; no zero error identity is invented. Rush construction and snapshot-local evidence preserve retainedDecomposition. Caps still need their own decomposition ownership/pairing transfer before full boundary admission.

Geometry build60326 completes successfully: wasm-opt11950867 to10625853 bytes. The six-suite run passes39 existing tests but cannot parse the new decomposition test because of an extra closing parenthesis; it is not recorded as a fully passing run. After that test repair, both decomposition transport/batch and actual-wall/body tests pass against the packaged kernel; log sweep-decomposition-focused-public.log. Independent actual-wall checks cover low-multiplicity multi-span source, complete/insufficient shared products,0.125mm retained displacement, invalid V weights, missing face, preserved inputs and a progressive body with finite wall bound while full continuousBound staysfalse. Logs: sweep-decomposition-transport-build.log, sweep-decomposition-public.log and sweep-retained-decomposition-public.log. Source vue-tsc and whitespace checks pass; log sweep-retained-decomposition-types.log. Full endpoint/cap decomposition integration and broader global/smoothness/STEP/UI objective remain active.

### Retained cap decomposition continuation (2026-10-02)

Added original-span decomposition certification for both actual retained cap
contours, preserving outer/hole roles and sharing the product budget. Integrated
the conditional cap errors into synchronous and streaming progressive bodies
and snapshot-local Rush evidence. Original retainedCaps.exact stays false for
numeric decomposition; the new retainedCapDecomposition report is separate.

Focused tests cover both endpoints, a hole, role permutation, changed actual
edge, exhausted second-endpoint budget and complete refusal without partial
upper bounds. Filled-cap composition includes decomposition before transfer.
Source vue-tsc passed. Evidence logs: sweep-cap-decomposition-public.log,
sweep-cap-decomposition-holes.log and sweep-cap-decomposition-types.log.

The original ideal profile domain is still unproved for the low-multiplicity
ring fixture; therefore its complete boundary bound remains null. This closes
a retained-cap correspondence gap, not the full continuousBound/global goal.

### Exact insertion for original NURBS profile ownership (2026-10-02)

Added a checked dyadic-arithmetic path for clamped nonperiodic profiles with
low internal knot multiplicity. Every homogeneous knot insertion operation and
deprojection must be exactly representable, established by i128 mantissa
comparisons; rounded operations refuse. Existing continuous contour simplicity,
separation, winding and hole-ownership audits run on the resulting exact curves.

Native evidence: the new insertion/domain/hole test, all four contour audits,
and all 58 progressive-miter tests passed. Overflow, underflow, inexact addition
and division tests refuse without granting identity. Arbitrary rounded
decompositions still need an original-span topology proof; full continuousBound
and global embedding admission remain separate and unproved.

WASM release compilation passed. Optimization/packaging remains live in exec
session 96338 (build log sweep-exact-decomposition-build.log); do not restart
this build based on an observation timeout. Public tests with the new expected
ideal-domain and hollow-bound results must run after packaging completes. Source
vue-tsc passed (sweep-exact-decomposition-types.log). These public expectations
are not yet verified against the newly packaged WASM.

### Public exact-profile proof and affine endpoint gap (2026-10-03)

The previous WASM build completed: 11958883 -> 10632987 bytes. Public tests
passed: 35 tests across five suites, including complete boundary bounds for
the low-multiplicity original NURBS profile and its hollow version. Evidence:
sweep-exact-decomposition-public.log. This supersedes the earlier null-bound
status of that fixture, without promoting full continuousBound.

Preparing four new independent STEP modes exposed ordinary affine endpoint
refusal for vector laws authored on [0,1]. station_with_affine_laws was asking
for derivative jets on an almost zero-width interval at zero. Station evaluation
now uses the existing point-safe original-law value certificate, sharing the
axis/center cell budget. Entire-interval interpolation continues to use jets.
Both endpoint values and exhausted budgets are regression-tested; all 59 native
progressive-miter tests pass (sweep-affine-endpoint-value-native.log).

Four original-NURBS hollow Rush examples parse successfully. The STEP exporter
now requires actual mode application, complete wall/cap decomposition proofs,
Rush/public-constructor B-rep equality and analytic volume 31.25 mm^3 (plain)
or 62.5 mm^3 (affine/frame-affine/guide-affine). First runs refused ordinary
affine endpoint enclosures as above. The expanded 22-case matrix is not yet
qualified; new endpoint-fix WASM optimization/packaging remains live in exec
session 91995. Re-poll that handle, do not restart on a timeout.

UI: the new original-NURBS guide+affine Rush sample built a separate exact
Solid group successfully. Viewed and saved at 1280x900 and 640x800:
unsegmented-guide-affine-solid-wide.png and -narrow.png. Original model group
and Rush source were preserved. Stale AX indexes after screenshots/resize
affected UI settings; cleanup used explicit DOM names, removed only the test
group, restored grid size 10, exited isolation, restored hidden editor/default
viewport, verified the original 546-character cyclic Rush source and one
unselected model Body 1, and closed temporary tabs. Restoration screenshot:
unsegmented-guide-ui-restored.png. This verifies this success/restoration case;
the complete failure/cancellation/source-change UI matrix remains open.

### Qualified 22-case STEP extension (2026-10-03)

The affine endpoint-fix WASM build completed (11958872 -> 10632976 bytes).
The full public run passed 34 of 35 tests; the streamed Rush topology test
exceeded its 30-second limit while STEP export ran concurrently. Its isolated
repeat passed in 23.03 seconds without changing the test timeout. Evidence:
sweep-affine-endpoint-value-public.log and sweep-affine-endpoint-streamed-isolated.log.
All 35 distinct public checks therefore have passing current-artifact evidence,
including the new ordinary-affine zero-domain endpoint/body regression.
Source vue-tsc passed (sweep-affine-endpoint-value-types.log).

STEP export completed with 22 cases. All four low-multiplicity source-profile
modes passed native Solid and complete boundary-budget gates, applied their
actual laws and retained identical B-rep geometry through Rush/public APIs.
Independent OCCT verification passed all 22 cases. For the new cases it checked
material roles, shell orientation, face-loop ownership, full-domain stored wall
and edge coefficient preservation, exact cap-pole coplanarity and volumes.
Measured volumes: 31.250000000000004 mm^3 (plain), 62.50000000000001 mm^3
(affine/frame-affine/guide-affine), relative error 1.1368683772161603e-16.

The independent verifier now recognizes positive retainedCapDecomposition
region evidence without relabelling original retainedCaps.exact. Its actual
cap-face references and exact pole planarity must agree. A deliberate metadata
corruption pointing the positive region certificate at faces [0,1] was rejected
while actual cap planarity and independent material agreement stayed true.
Evidence: external-step-unsegmented-modes/manifest.json and
opencascade-sweep.json; sweep-unsegmented-step-export-final.log,
sweep-unsegmented-step-occt-regions.log and sweep-unsegmented-step-negative-region.log.

This supersedes the pending-build/pending-22-case status above. No process from
this continuation remains live. General source-span topology for rounded
decomposition, full continuousBound, all-mode global/smoothness proofs and the
remaining UI failure/cancellation/source-change matrix are still open. The
22 selected STEP fixtures do not by themselves prove those wider requirements.

### Retained wall-domain and body ownership gaps (2026-10-03)

Found and closed two missing premises in both retained-wall error routes.
Surface coefficient matching now additionally requires full unit-square UV
trim coverage without holes, using four exact clamped rational linear sides,
and complete unique face ownership by the single body's closed shells. Cyclic
wire starts and reversal remain valid for the face-set bound. Omitted/repeated
faces, wrong owners, clipped trims, added holes, missing sides and nonpositive
pcurve weights refuse rather than returning partial upper bounds. Work is
bounded by the requested face limit before ownership walks.

Focused evidence: seven tests passed (sweep-retained-face-coverage-public.log),
including exact and numerical routes, input preservation and trim/ownership
mutation regressions. Typecheck passed. Broad parallel regression passed 35 of
36 tests, with the streamed topology case exceeding 30 seconds again; a final
single-worker run includes embedding compatibility without raising timeouts.
Full continuousBound, world-coedge/embedding and all-mode smoothness remain
separate outstanding obligations.

Final validation of these coverage changes: six public suites / 41 tests pass
with maxWorkers=1, unchanged test timeouts, including the formerly timed-out
streamed Rush case and nurbsSweepEmbedding compatibility. Source vue-tsc and
git diff --check pass. Evidence: sweep-retained-coverage-serial-regression.log
and sweep-retained-coverage-final-types.log. No live verification process
remains from this continuation.

### Retained wall world-coedge binding (2026-10-03)

Both retained wall routes now require exact native surface/pcurve/stored-world
edge identity for all four unit-square coedges. The shared per-face exact work
budget is 16384; unresolved, unsupported, exhausted and malformed identities
refuse the wall-domain premise. Shell embedding remains separate. A changed
world edge now refuses; a consistently translated wall plus its edges still
returns the appropriate nonzero source correspondence bound. Seven focused
tests in two suites and vue-tsc pass. Evidence:
sweep-retained-world-coedge-public.log and sweep-retained-world-coedge-types.log.
Full continuousBound and the remaining all-mode requirements remain open.

### World-coedge regression and exact-work correction (2026-10-03)

The initial 16384 per-face work limit was insufficient for rational coedges:
the six-suite regression produced seven refusals, including corrected modes.
The final bounded per-face shared budget is 1000000, preserving exact identity
and refusal on exhaustion. No numerical tolerance was substituted. Exact-route
mutation regressions now cover changed world poles, wrong traversal direction
and wrong edge references. A changed surface reports the earlier world-domain
failure before coefficient mismatch; the embedding test expects this reason.
Final six-suite run: 41/41 tests pass, unchanged timeouts, 51.96 seconds.
vue-tsc and scoped diff-check pass. Evidence:
sweep-world-coedge-regression-final.log and sweep-world-coedge-mutations.log.
The earlier failed regression log is retained as diagnostic evidence.

### Shared retained world-coedge work budget (2026-10-04)

Both retained wall routes now share one maxExactWork budget over all wall faces
(default/hard maximum 1000000), instead of resetting it for each face. Invalid
budgets and exhaustion refuse without a partial bound. Tiny-budget regressions
cover both routes. Six public suites / 44 tests pass; vue-tsc and scoped diff
check pass. Evidence: sweep-world-coedge-shared-budget.log. Exact geometric
identity is computed in Rust; current B-rep ownership/domain orchestration is
TypeScript and native composition remains an outstanding migration task.

### Native retained-body ownership premise (2026-10-04)

Added Rust transport operation sweep_retained_body_coverage_audit. It checks
single-body ownership, closed shells, unique shell references, complete unique
face coverage and explicit boolean face orientation, bounded to 1026 faces.
Native regression covers omitted/repeated faces, wrong shell owner, open shell,
face work limit and malformed orientation. Focused cargo test passes; evidence
sweep-retained-body-native.log. This is a combinatorial coverage premise only;
shell winding and embedding remain separate. WASM rebuild and replacement of
TypeScript orchestration are not yet completed.

### Native ownership integration in progress (2026-10-04)

The TypeScript body ownership walker was replaced by the native transport call
sweep_retained_body_coverage_audit. Faces are represented by count placeholders;
shell/body ownership references are passed unchanged. Source typecheck and
scoped diff-check pass. geometry-wasm release compilation completed in 1m20s;
optimization/packaging remains active in build session 17180. Public validation
must use the newly packaged artifact before this integration is qualified.
Evidence: sweep-native-body-wasm-build.log. No claim of full continuousBound.

### Native inner-shell ownership regression (2026-10-04)

Two focused Rust tests now pass: the added case accepts complete unique outer
and inner shell coverage, and refuses duplicate shell owners, cross-shell face
duplication and omitted inner owners. Evidence: sweep-retained-body-native-final.log.
Build session 17180 remains live in wasm-opt (PID 42538 verified), so public
artifact validation remains pending; do not restart or call integration complete.

### Public native ownership fixture prepared (2026-10-04)

Added nurbsRetainedBodyCoverage.test.ts for the actual WASM operation: unique
inner-shell coverage, count cutoff, duplicate faces/owners, open shell, malformed
orientation, negative/fractional/out-of-range references and input preservation.
Typecheck and scoped diff-check pass. Public fixture execution remains pending
the newly packaged WASM. Session 17180 / wasm-opt PID 42538 was revalidated
live at 4m14s CPU 99.2%; no restart was performed.

### Native transport revalidation during packaging (2026-10-04)

An attempted current-source transport regression initially failed compilation
because Gregory-patch call sites and their closure signature were temporarily
inconsistent in the shared worktree. After the shared source changed to consistent
two-argument calls, the same command passed both native ownership tests. Evidence:
sweep-native-body-transport-regression.log (initial diagnostic) and
sweep-native-body-transport-regression-final.log (passing repeat). Packaging
session 17180 remains live in wasm-opt; public WASM qualification is still pending.

### Raw release WASM ownership ABI verified (2026-10-04)

While optimization remains live, instantiated the compiled release geometry
WASM directly and used the actual binary request ABI (encodeBinary, abi_alloc,
abi_request, decodePacked, abi_free). Positive unique inner-shell coverage,
duplicate-face refusal and malformed-orientation error all pass. No JS geometry
fallback was used. Evidence: sweep-native-body-raw-wasm-abi.log. This proves the
native operation crosses the WASM binary boundary, but does not substitute for
packaged public API/Rush regression, which remains pending packaging.

### Native body ownership integration qualified (2026-10-04)

Build session 17180 completed successfully; wasm-opt 12057880 -> 10719343 bytes.
The packaged public artifact passes seven suites / 45 tests in 57.08s, unchanged
timeouts, including the direct native ownership API, corrected progressive miter,
streamed Rush, retained decomposition/caps and embedding compatibility. Evidence:
sweep-native-body-public-regression.log and sweep-native-body-wasm-build.log.
This supersedes the pending ownership integration entries above. The TypeScript
ownership walker is now removed in favor of Rust transport. Wall-domain/B-rep
composition migration, full continuousBound and all-mode qualification remain open.

### Native retained wall-domain composition (2026-10-04)

Added Rust sweep_retained_wall_domain_audit: exact complete four-sided unit-square
UV domain, positive clamped linear pcurves, no holes, and all four exact stored
world-edge identities against the actual surface under one bounded work budget.
Returns consumed work for composition without declaring embedding. Native fixture
passes positive coverage and rejects exact-work cutoff, changed world pole,
added hole, clipped UV and wrong reversal; input preservation checked.
Evidence: sweep-native-wall-domain.log. Native cargo check passes. WASM build
started; TypeScript wall-domain walker is still active until public integration
is qualified. Full continuousBound is not asserted.

### Native wall-domain integration and raw ABI (2026-10-04)

Removed TypeScript geometric unit-square/coedge auditing; host now resolves
B-rep references and passes surface/holes/coedges/remaining shared budget to the
Rust operation, consuming native reported work. Source typecheck and diff-check
pass. Native regression additionally accepts cyclic starts, reversed traversal,
and exact consumed-work budget (one less refuses). Direct compiled release WASM
binary ABI passes positive rectangular coverage, cutoff and changed-world-edge
refusal. Evidence: sweep-native-wall-domain-final.log, sweep-native-wall-raw-abi.log,
sweep-native-wall-types.log. Build session 68275 release compilation completed
in 1m17s; optimized packaging/public regression remains pending. Full boundary
composition and continuousBound still require further work.

### Public wall-domain transport fixture prepared (2026-10-04)

Added nurbsRetainedWallDomain.test.ts: native work-count cutoff at exact consumed
work and one less, valid cyclic/reversed wires, changed world pole, clipped UV,
hole and excessive-budget refusal, with input preservation. Typecheck and scoped
diff-check pass. Public execution still awaits optimized package session 68275;
its wasm-opt PID 53688 was verified live (2m05s, 98.9% CPU). No restart.

### Rational coedge parameterization regression (2026-10-04)

Native wall-domain regression now accepts positive unequal rational weights when
UV and stored world-edge parameterization agree, and refuses mismatched world
weights or a nonpositive UV weight. Both native tests pass. The public fixture
includes the matching/mismatching rational cases. Evidence:
sweep-native-wall-rational-parameterization.log. Packaged-public execution is
still pending session 68275, whose wasm-opt PID 53688 was revalidated live at
4m29s / 99.9% CPU. This is a verified wait, not a failed/stopped build.

### Native outward error composition primitives (2026-10-04)

Added numerics::error_upper add/multiply/sqrt_two primitives in Rust. Zero terms
remain exact; invalid/overflowing bounds refuse; nonzero nearest-rounded results
are enclosed by one binary64 successor, including subnormals. The sqrt(2)
coefficient's squared inequality and returned normal/subnormal output bounds
are independently verified with exact u128 dyadic arithmetic. Two native tests
pass; evidence sweep-native-error-upper.log. These primitives are not yet wired
to cap/boundary admission and do not establish certificate provenance or full
continuousBound. Wall-domain package session 68275 remains live; no restart.

### Native conditional filled-cap/boundary composition (2026-10-04)

Added sweeps::filled_cap_error: compose endpoint, correction and decomposition
bounds with native outward arithmetic; require both ideal domains, exact retained
regions and nonzero signed projection intervals; apply the general sqrt(2)
transfer unless parallel planes are certified. Missing explicit decomposition
premises refuse. Boundary union requires both filled caps for open paths and
uses a maximum without extra rounding. Two native tests pass, including missing
premises, invalid bounds, parallel/nonparallel transfer and open/closed coverage.
Evidence: sweep-native-filled-cap-error.log. This is conditional composition,
not provenance/admission/embedding; transport and host replacement remain pending.
Wall-domain packaging session 68275 is still live; no restart.

### Native wall-domain public integration and cap transport (2026-10-04)

Wall-domain package session 68275 completed, optimized 12064164 -> 10724480 bytes.
Eight-suite public run passed 45 checks; the added rational fixture initially
used equal weight order for reversed world coedges. Corrected fixture supplies
reversed world weight order when use.reversed is true; its focused repeat passes.
All 46 distinct public checks now have passing evidence; no test timeout changed.
Evidence: sweep-native-wall-public-regression.log (45 pass + fixture diagnostic),
sweep-native-wall-public-fixture-final.log, sweep-native-wall-final-types.log.
Rust wall-domain operation now supplies actual retained domain/coedge binding;
the TypeScript geometric walker is removed. This supersedes pending entries.

Added sweep_filled_cap_error_upper and sweep_boundary_error_upper native transport
operations. Missing decomposition refuses; malformed paired input errors; both
return continuousBound:false. Three native composition/transport tests pass
(sweep-native-cap-transport-tests.log). A new WASM build for these operations
is active; public cap composition still uses TypeScript until migrated/qualified.

### Cap/boundary host composition migrated (2026-10-04)

Replaced filledMiterCapErrorUpper and certifiedSweepBoundaryErrorUpper arithmetic
with native transport calls. Host preserves null refusal on invalid input,
explicit zero decomposition for the legacy omitted contribution, and absence of
cap obligations for closed paths. Native results remain conditional estimates;
no continuousBound admission is inferred. Typecheck passes; optimized WASM
session 88481 and public regression are still pending.

Cap-composition release WASM compilation completed in 1m28s. Direct binary ABI
validation passes zero/correction transfer, overflow and missing decomposition
refusals, open/closed boundary obligations and continuousBound:false. Evidence:
sweep-native-cap-raw-abi.log. Optimized package session 88481 remains live;
public host/Rush regression must still pass before this migration is qualified.

### Native cap composition ABI parity and public contract (2026-10-04)

Direct release WASM binary ABI matches previous host outward arithmetic bitwise
for 144 normal/subnormal/zero/correction/decomposition/parallel combinations.
Evidence: sweep-native-cap-abi-parity.log. Added public native transport tests
for conditional cap estimates, missing/invalid premises, malformed pair shape,
and required open-path cap coverage; continuousBound remains false in outputs.
Typecheck passes; public execution remains pending optimized package session
88481, revalidated live without restart. Arithmetic parity does not prove
ideal/retained certificate provenance or the full continuousBound obligation.

### Current source scope refresh (2026-10-04)

Fresh inspection finds constructor-owned SweepBoundaryCertificate composition,
including weak ownership/provenance snapshots in brep.ts, and public Rush/Solid
fixtures for authored axis + guide + affine (corrected and uncorrected) that
expect continuousBound:true. Thus earlier blanket statements that every complete
boundary certificate remains false are stale for the current shared worktree.
The kernel conditional cap/boundary scalar transport still returns false itself;
constructor-owned assembly can certify the boundary-set scope separately.
All-mode library completion is not established by these selected fixtures.
After cap package session 88481 completes, include sweepBoundaryCertificate.test.ts
in the public regression and revalidate these current constructor paths.

### Native cap/boundary composition public integration qualified (2026-10-04)

Package session 88481 completed successfully: wasm-opt 12069345 -> 10729423 bytes.
Ten public suites / 53 tests pass in 64.75s, unchanged timeouts. The set includes
native cap/boundary transport, constructor-owned complete boundary certificates,
work-exhaustion refusals, moving-axis guide affine nonparallel caps, retained
wall/body audits, decomposition, corrected and combined miter/Rush, and embedding.
Source typecheck and scoped diff-check pass. Evidence:
sweep-native-cap-composition-public-regression.log,
sweep-native-cap-composition-final-types.log. This supersedes pending integration
entries. No build/test session remains live from this cap migration.

The full library goal is still unproven across all original requirements:
all declared mode/topology/smoothness combinations, independent STEP geometry
and topology matrix, and broad/narrow UI failures/cancellation/source restoration
must be audited against current source and evidence. Selected positive complete
boundary certificates are now qualified; they are not an all-mode completion claim.

### Fresh independent STEP matrix with low-multiplicity combined mode (2026-10-04)

Added authored frame + orientation guide + affine to the original low-multiplicity
hollow source-profile modes, including a Rush example and exporter fixture. The
exporter checks applied flags, original decomposition/cap evidence and complete
boundary budget; Rush/public B-reps must be identical. Fresh export and independent
OCCT verification pass all 34 selected cases on geometry WASM SHA256
9a3a20c60a86f63f5148687ad54e118624fe61cde304827c11fc334c56ddaef6 (10729423 bytes),
with packed/public identity verified. New combined case: 10 faces, 1 shell,
volume 62.50000000000001 vs analytic 62.5 mm³, relative error 1.1368683772161603e-16.
Evidence: external-step-native-composition/manifest.json and opencascade-sweep.json;
sweep-native-composition-step-export.log, sweep-native-composition-step-occt.log.
This supersedes the old 22-selected-case count for current qualification. Scope
remains selected fixture geometry/topology/analytic volume, not a completed
all-mode surface/seam/containment/UI matrix.

### Independent complete wall UV pcurve coverage (2026-10-04)

STEP manifest v2 records source wall coedges/pcurves. OCCT verifier now requires
imported wall pcurves to be affine lines or clamped two-pole degree-one uniform
positive-weight B-splines over the complete edge parameter interval. Exact
Fraction arithmetic over stored line origin/direction/parameters or stored poles
compares both UV endpoints to the original forward-edge parameterization,
including reversed source coedges. Linear coefficient convexity covers the full
UV interval within the stated UV tolerance; no sampled endpoint evaluation is
used for this gate. Cap pcurves and G1/G2/containment remain separate.
All 34 selected cases / 2480 wall coedge uses pass. A deliberate source pcurve
pole corruption refuses while unchanged STEP's sampled correspondence still
passes. Evidence: external-step-wall-pcurve-domain/manifest.json and
opencascade-sweep.json; sweep-step-wall-pcurve-occt.log and
sweep-step-wall-pcurve-negative.json. Legacy v1 manifests do not claim this gate.

### Independent imported material-side probes (2026-10-04)

OCCT verifier now independently classifies both sides of every matched wall
center using the imported solid and its oriented surface normal. Two offsets
(100 and 1000 times the STEP tolerance) require inward material IN and outward
void OUT; singular normals, ON/UNKNOWN and wrong material sides refuse. All
34 selected cases / 2480 classifications pass together with the previous
full-domain wall UV, topology, cap planarity, shell orientation and volume gates.
Evidence: sweep-step-material-side-probes.log and the updated
external-step-wall-pcurve-domain/opencascade-sweep.json. Python syntax and scoped
diff-check pass. These are finite material-side witnesses; arbitrary nested-shell
containment and whole-domain embedding are not established by them.

### Native whole-boundary premise for smoothness extraction (2026-10-04)

Profile/station seam extraction previously recognized only UV control endpoints,
allowing malformed/periodic pcurves to receive positive represented-strip jets.
It now additionally invokes the existing Rust sweep_boundary_coverage_audit,
which validates the actual pcurve and complete natural boundary. Periodic and
malformed-knot mutation regressions refuse extraction and G1/G2 for both scopes.
The stale closed joint-mode assertion demanding >1000000 work was replaced by
actual positive work plus refusal at consumed-work minus one, preserving every
128 seam and the upper aggregate budget (optimized current work: 61840).
All three smoothness suites / 18 tests pass: straight/unequal G2, G1-only cases,
sharp C0, closed joints, reconstruction, moving frames, Rush/viewport and Solid.
Typecheck and scoped diff-check pass. Evidence:
sweep-native-smoothness-domain-final.log and sweep-native-smoothness-domain-types.log.
The initial diagnostic run remains sweep-native-composition-smoothness.log.
No all-mode smoothness or full-boundary G2 claim is inferred from these fixtures.

### Visible group-source Solid refusals (2026-10-04)

Fixed a missing error region in the group source editor: native scalar-law
refusals were recorded but invisible there. Actual browser checks confirm
visible zero-scale refusal at wide and 720 x 900 sizes, retry with affine +
authored frame + guide + hollow quadratic profile successfully adding a B-rep
body to Solid, reopening stored source, cancellation of edits, and Undo restoring
the original one-body scene. Direct Vue typecheck and scoped diff-check pass.
Evidence and explicit remaining UI scope: qualification/sweep-coverage-2026-10-01/
sweep-group-error-ui.md and four sweep-group-*.jpg screenshots. This does not
close active worker cancellation, source-change races, narrow success or all-mode
UI qualification. The npm pretypecheck kernel rebuild was stopped during
wasm-opt; only the subsequent direct Vue typecheck is a completed check.

### Active Solid cancellation and source replacement UI (2026-10-04)

The actual group-source workflow now has wide and 720 x 900 browser evidence
for explicit cancellation while Build is disabled/busy and source replacement
while Build is disabled/busy. Both preserve the original body without publishing
the superseded result. A subsequent narrow affine + authored frame + guide
hollow quadratic build succeeds and Undo restores the original one-body scene.
All 12 exactSolidWorker tests pass, including cancellation during receiving-realm
B-rep validation. Evidence: sweep-solid-cancel-ui.md, sweep-solid-cancel-worker.log,
and the associated six screenshots under qualification/sweep-coverage-2026-10-01.
Main progressive viewport cancellation, main-source To Solid, reload recovery
and wider geometric refusal coverage remain unqualified by this group UI slice.

### Main-source cancellation and reload recovery (2026-10-04)

Actual browser qualification adds main progressive viewport Cancel build at
wide and 720 x 900 sizes, wide viewport source replacement during active work,
wide main-source To Solid cancellation, and narrow main-source To Solid source
replacement cancellation. Original 2560-triangle viewport and one-body Solid
scene were preserved. After restoring Auto/source/drawer/viewport and reloading,
the original group/body returned and textarea source exactly matched its initial
snapshot. Evidence: sweep-main-cancel-ui.md and three sweep-main-*.jpg screenshots.
Main-source positive/refusal cases at both sizes, narrow viewport replacement,
crash recovery and the full all-mode UI matrix remain outside this slice.

### Main-source Solid positive/refusal at both sizes (2026-10-04)

Actual wide/720 x 900 browser checks now show main-source To Solid refusing
scale [0,0] visibly without modifying the original body, retrying [1,1] to
successfully replace group model, and Undo restoring it. Narrow viewport source
replacement during active Build also preserves the original 2560 triangles.
After restoring Auto/source and reloading at narrow width, source equality was
confirmed and the original body was visible after resetting width. Evidence:
sweep-main-solid-positive-refusal-ui.md and five accompanying screenshots.
The selected affine + authored frame + guide + hollow quadratic workflow now
covers success, refusal, active cancellation, source change and reload/Undo
restoration across the group/main routes and both sizes. This selected fixture
does not establish an all-mode UI or arbitrary-race guarantee.

### Retained wall periodicity premise (2026-10-04; verification pending)

Rust wall regularity now requires stored periodicity to match the authored
profile, consistently with level-error and wall-geometry audits. A structurally
valid periodic-to-nonperiodic section mutation reproduced the missing rejection
before the change. Native module tests and fresh geometry WASM rebuild were
started but are still active at this entry; no completed post-change pass is
claimed. Evidence and remaining qualification: sweep-wall-periodicity.md and
sweep-wall-periodicity-{mutation-before,native-final,wasm}.log under qualification.

Native follow-up: the full progressive_miter module has now completed with
62/62 passing tests, including the valid periodic-to-nonperiodic mutation.
The WASM optimizer is still active; fresh packaged/public verification remains
pending and this entry must not be read as a completed WASM qualification.

Completed follow-up: geometry WASM optimization/packaging exits successfully;
generated/public files and identity agree on SHA256
34a4e5be7bd2b9fb9b4156adb42da4c30400e444d50331103594b12d8b27e0e6
(10731689 bytes). Five fresh public suites / 52 tests pass in 52.38 seconds:
progressive miter, boundary certificate and all three selected smoothness suites.
A new positive public regression retains periodic profile identity in all returned
sections with certified regularity and unchanged authored input. Direct Vue
typecheck and scoped diff-check pass. Evidence: sweep-wall-periodicity-public-final.log,
sweep-wall-periodicity-types.log and the updated sweep-wall-periodicity.md.
This supersedes the pending state above for this fix, not the outstanding global
embedding, arbitrary-mode regularity or complete qualification scope.

### Independent full-domain cap coedge agreement (2026-10-04)

OCCT verification now supplements sampled cap pcurve/world agreement with an
exact Fraction enclosure over every imported cap edge parameter. Shared positive
rational bases transfer affine control residuals; a convex UV hull bounds the
stored bilinear remainder of each clamped bilinear cap on its affine knot domain.
Unsupported bases refuse; outer/hole cap coedge use counts must match exactly.
Fresh export from current WASM SHA256 34a4e5be7bd2b9fb9b4156adb42da4c30400e444d50331103594b12d8b27e0e6
passes 34/34 independent cases / 488 cap coedge uses. Interior UV pole and weight
mutation checks refuse. Evidence: external-step-cap-coedge-domain, export/OCCT
logs and sweep-step-cap-coedge-domain.md under qualification. Python compilation
and scoped diff-check pass. This closes the selected fixture cap-edge sampling
gap, not global cap material/embedding, nesting or all-mode geometry guarantees.

### Closed affine cavity orientation refusal (2026-10-04)

Rust qualification now mutates a positively certified closed affine hollow miter
by reversing every inner-shell face use. Fresh boundary and nesting roles remain
positive, but the independently outward inner shell correctly refuses the volume
certificate; input is unchanged. Existing incomplete disjoint-pair evidence
continues to refuse the exact-witness nesting fallback. Native volume_validity
7/7 tests pass, plus shell_ 32/32 (overlapping subsets, not 39 distinct cases).
Evidence: sweep-affine-cavity-orientation.md and its two native logs under
qualification. This adds Rust regression coverage only; no production/ABI change.
All-mode moving-frame/guide nesting and embedding qualification remains open.

### Joint moving authored-frame/guide/affine material roles (2026-10-04)

The selected closed joint periodic hollow fixture now has public Rust/WASM
regressions for original positive material roles, reversed cavity orientation
refusal, and wrong cavity-as-material-body ownership refusal. Boundary remains
embedded in both negative cases; orientation and parent-role gates independently
prevent solid admission. All six embedding tests pass in 16.83 seconds with
unchanged timeouts. Evidence: sweep-joint-cavity-material-roles.md and final
public/typecheck logs. The body-count mutation uses supported legacy geometry
without a stale one-body topology ID table; the initial table-count rejection
is retained as a diagnostic and is not counted as a role certificate.
This extends selected joint mode coverage, not arbitrary frame/guide/nesting
or all-mode guarantees. Production Rust/WASM is unchanged by these tests.

Joint-mode budget follow-up: public volume audit with nesting limits 2/2 keeps
parents/roles/orientations null and Solid false despite positive embedding.
Orientation limits 1/1 retain positive nesting but unresolved orientation and
Solid false. Reported aggregate work stays within those limits; input unchanged.
All six embedding tests pass in 19.55 seconds, unchanged timeouts. Evidence:
sweep-joint-cavity-budget-public.log and updated sweep-joint-cavity-material-roles.md.
This verifies budget refusal composition for this joint mode, not all-mode
geometric completeness.
