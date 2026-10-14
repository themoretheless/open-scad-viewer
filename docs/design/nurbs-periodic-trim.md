# Периодический UV trim: текущая реализация

Статус NURBS-0173 — **planned**. Реализованы автоматический поиск событий, построение кандидатов, консервативный UV admission, точные стыки и достаточное тождество исходного пути. Для точного пути реализованы перенос winding и сборка локальных циклов по двум швам; добавлена приближённая сборка с достаточным сертификатом периодической изотопии и anchor winding. Соответствие 3D-поверхности проверяется отдельным и объединённым admission. Добавлено точное назначение продолжений сбалансированных многоветвевых контактов; многоветвевые corner ports поддержаны общим flux closure; неразделённые равные ведущие коэффициенты и произвольная итоговая кратность остаются незавершёнными.

API доступны в `nurbs_core::periodic_trim` и `nurbs_core::domains::parameterization::periodic_trim`. Вход — непериодическая lifted 2D UV-кривая без modulo скачков. Её knot-параметры сохраняются; нормализованный домен не предполагается.

| API | Доказанное поведение | Ограничения |
|---|---|---|
| split_candidates | Упорядоченные разрезы по явным cuts, явные tile/translation, контрольная оболочка кандидатов в chart | Само построение не доказывает исходную геометрию или правильность cuts |
| inspect_seams | Полное interval root coverage исходной кривой для всех перечисленных материализованных швов | Coincident/неразрешённые события дают Unresolved; бюджет — WorkLimit |
| verify_partition | Непрерывное UV-отклонение от исходных spans, включая translation с наружным округлением | Не world-space отклонение на поверхности и не топология |
| inspect_connectivity | Точные lifted стыки и опциональное периодическое замыкание с заданным winding | Не простота петли или наличие внутреннего региона |
| verify_identity | Достаточное точное параметрическое тождество пути через original knot insertion | Округлённые операции и отличающиеся кодировки дают «не доказано» |
| split_verified | Автоматическая цепочка поиска, точных cuts, выбора tile, chart hulls, стыков и UV admission | Partition публикуется только после этих проверок; тождество и closed winding вызываются отдельно |
| split_approximated | Кандидаты для непредставимых roots с явным geometric snapping и последующим UV admission | Не точные roots, не topology transfer; перекрывающиеся interval events отвергаются |

## Поиск и построение швов

inspect_seams перечисляет уровни по положительной рациональной контрольной оболочке исходника, добавляя внешние уровни. curve_plane работает над исходными UV, вложенными в Z=0, и сохраняет интервалы корней, касания, совпадения и доказательства. Интервалы не заменяются приблизительной точкой. Поле total_seams при раннем отказе перечисления — нижняя граница нужного числа уровней.

split_verified принимает точечные события или интервальные события, внутри которых точно подтверждён binary64 параметр. Для degree-1 используется точный homogeneous residual исходных произведений. Для более высокой степени дополнительно используется homogeneous de Boor, где каждая операция должна быть точно представима. Та же basis-проверка распознаёт точные монотонные endpoint roots в curve_plane. Середина/концы интервала — лишь предложения: до трёх попыток на событие, учитываемых parameter_attempts. resolved_events сохраняет исходный интервал и доказанный параметр. Например, t=3.5 может быть принят; t=1/3 не снапится по допуску.

Ненулевые начала и произвольные периоды chart допускаются при доказанной точности решётки: TwoSum проверяет period и origin+product, FMA — integer*period. Ненулевые концы chart и period ограничены по модулю [2^-500,2^500], индексы i32, чтобы исключить underflow остатков. Ненулевой остаток даёт LatticeRoundingUnresolved. Tile выбирается по уровням с учётом начала chart; затем вся контрольная оболочка локального кандидата должна находиться в прямоугольнике. Это достаточная, а не необходимая проверка.

## Отклонение, связность и тождество

verify_partition требует полного упорядоченного покрытия исходного домена без дыр. Исходник не заменяется повторно trimmed reference. На общей сетке исходных и кандидатных spans вычисляются наружные interval enclosures; translation добавляется интервально. WithinTolerance подтверждает всё разбиение, Mismatch — разделение на проверенной ячейке, Unresolved — недостаток бюджета или точности параметров. error_upper конечно только при полном подтверждении, иначе infinity.

inspect_connectivity требует clamped one-sided endpoints и сравнивает endpoint/translation точной суммой binary64 произведений, без округлённого предварительного сложения. Минимальный субнормальный gap не снапится. winding=None означает open chain; Some([ku,kv]) дополнительно требует точное last-first=(ku*period_u,kv*period_v). Ненулевой winding сам по себе не задаёт плоский внутренний trim-регион. Gap — доказанное нарушение связи, Unresolved — недостаток достаточного доказательства или бюджета.

verify_identity уточняет исходную кривую в cuts только точными homogeneous knot operations. Кандидаты также точно разбираются на Bezier spans; обратный translation контролей должен быть точно представим. Все соответствующие knots, controls и weights должны совпасть, все spans — покрыться. Разные кодировки могут представлять одинаковую геометрию, поэтому отказ не является доказательством различия.

## Бюджеты

- split_candidates: max_pieces 1..256, tiles=cuts+1, cuts строго упорядочены внутри active domain.
- inspect_seams: max_seams 1..4096; общий max_work 1..100000 на все seam reports.
- verify_partition: max_work 1..100000 на всё разбиение; допуск в UV-единицах.
- inspect_connectivity: max_links 1..257, до 256 кусков.
- verify_identity: max_segments 1..4096; внутренний предел точного уточнения 4096 контролей.
- split_verified: отдельные search/admission budgets, ограничивающие суммарную работу; число предложений точного параметра ограничено числом изолированных событий.

## Проверка 2026-10-14

В изолированном target, offline/locked, без default features прошли **14 periodic_trim + 36 curve_plane tests**. Процесс завершился с exit 0. Хеши periodic_trim.rs, exact_curve_segments.rs, curve_plane.rs и tests/periodic_trim.rs до и после проверки совпали.

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --manifest-path crates/Cargo.toml -p nurbs-core --no-default-features --offline --locked --jobs 1 --target-dir /tmp/nurbs-periodic-verification --test periodic_trim --test curve_plane --quiet
node scripts/map-nurbs-api.mjs --check
node scripts/audit-nurbs-catalog.mjs
```

Проверены рациональный и квадратичный точные корни, отказ от снапинга t=1/3, касание/совпадение, смещённый chart с периодом 3, неполные бюджеты, повреждённый translation, субнормальный gap, periodic winding, точное тождество и изменение endpoint на следующий binary64. Тестовые seam cuts берутся из authored knots; from_polyline может иметь ненормализованный домен.

Отдельный arithmetic unit test ранее прошёл с обычными features. Повторный native-only library harness после исправления двух негейтированных JSON-блоков в sweep tests собрался; выбранный arithmetic unit test прошёл (1 passed). Это не означает выполнение всего library suite. Полный repository suite, WASM, editor и STEP этим запуском не квалифицируются.

## Осталось для N173

Полная классификация и порядок перекрывающихся событий, неподтверждённые lattice/intermediate constructions, перенос региональной топологии для tolerance-qualified разрезов; per-chart региональные границы, seam ownership и перенос топологии петель/отверстий; соответствие на поддерживающей поверхности. Подтверждение близости или тождества пути не выдаётся за готовый trim-регион.

## Кандидаты для непредставимых корней

split_approximated сначала пробует строгую операцию. При IntervalRootNeedsConstruction она повторно использует уже полученное полное покрытие событий и доказанную решётку, не запускает новый поиск. Неперекрывающиеся интервалы задают однозначный порядок разрезов. Для каждого выбирается внутренний binary64 параметр; общая точка строится по исходной кривой и принудительно помещается на соответствующий seam. Обе one-sided endpoint controls получают эту точку. Изменённые кривые локализуются в chart, затем обязаны пройти полный UV admission и точные lifted стыки. approximated_events сохраняет исходный interval и предложенный parameter. Это геометрическая аппроксимация, а не утверждение C(parameter)=seam.

Общие интервальные U/V events не считаются aliases: EventOrderUnresolved. Точечные одновременные события могут разделять один cut. Непредставимый внутренний root не пропускается, если midpoint округлился в конец домена. В split_candidates отвергаются внутренние knot jumps с multiplicity>degree: disconnected source не трактуется как контур.

До переноса региональной топологии это расширение не закрывает N173. Новые регрессии: t=1/3 и 2/3, exact seam endpoint representatives, обязательный UV certificate, отказ от exact identity, недостаточный admission budget, overlapping corner intervals и discontinuous UV input. Проверка завершена: 17 periodic_trim + 36 curve_plane tests passed без default features, offline/locked, в изолированном target. Процесс завершился с exit 0; хеши четырёх проверяемых файлов до и после совпали.

## Точный перенос lifted и периодического winding-региона

verify_lifted_region проверяет до 64 исходных ориентированных замкнутых loops и до 256 кусков суммарно. Для каждого нужны точное авторское замыкание, exact lifted closure с winding=[0,0], целочисленные периодические translations, положительная рациональная контрольная оболочка в chart и verify_identity всего пути. При этих условиях для любой точки вне общей границы совпадают winding-функции каждого исходного/восстановленного контура, их сумма, а затем конечные периодические суммы по UV-образам. Hole roles не угадываются: ориентация и порядок исходных loops сохраняются.

Отчёт выдаёт lifted_winding_preserved и periodic_winding_preserved только после полного прохода. Общие max_segments 1..4096 и max_links 1..256 действуют на всю коллекцию; частичный проход не публикует общий положительный результат. Непредставимые уточнения, неверные tiles, открытые source loops, незамкнутые кандидаты и недостаточный бюджет остаются неподтверждёнными. Это не доказывает простоту или nested-loop расположение исходного региона и не создаёт per-chart B-rep face boundaries.

Проверка 2026-10-14: 19 periodic_trim + 36 curve_plane tests passed без default features, offline/locked, в изолированном target. Хеши четырёх исходных файлов до/после совпали. Включены outer loop и clockwise hole через seam, неверный tile, повреждённая hole geometry, общие бюджеты и open loop. Отдельно прошёл 1 native arithmetic unit test после gating fixes. Полная готовность N173 пока не заявляется.

## Полные knot boundaries без перестройки рациональных данных

В split_candidates добавлен точный путь для границ существующих spans с полной кратностью degree (и clamped концов active domain). Контроли, веса и внутренние узлы копируются из исходной B-spline definition; преобразования Euclidean→homogeneous→Euclidean не выполняются. Это сохраняет любые исходные binary64 веса и координаты, даже когда их произведение не представимо точно. Неподдержанные границы используют прежний trim с последующим admission.

Точный decomposition helper аналогично копирует уже авторские Bezier spans. При дополнительных неавторских cuts сохраняется прежняя проверенная knot insertion, которая может отказать из-за округления. Проверка exact identity остаётся достаточной и не превращает несовпадение кодировок в отрицательный геометрический сертификат.

Проверка 2026-10-14: **67 tests passed** без default features, offline/locked в изолированном target: 20 periodic_trim, 36 curve_plane, 4 curve_self_intersection, 4 uv_closure, 3 uv_simplicity. Хеши четырёх проверяемых файлов до/после совпали. Новый end-to-end test берёт рациональную окружность с ненулевым центром и исходными nonunit weights, пересекающую UV seam, проверяет сохранение controls/weights, exact path identity, periodic region winding и неизменность исходника. Полный repository suite и готовность N173 не подразумеваются.

## Сборка локальных boundary cycles по одному шву

`assemble_single_seam(sources, partitions, chart, wrap_axis, max_segments, max_links, max_edges)` экспортируется из periodic_trim; реализация находится в periodic_trim/chart.rs. Возвращает ChartBoundaryGraph с исходными NURBS-участками, добавленными degree-1 boundary connectors, замкнутыми последовательностями edge indices и парами противоположных connectors.

Перед сборкой требуется точный RegionTransfer. wrap_axis=0 или 1; другой axis не должен иметь tile shifts. Исходные и кандидатные контрольные оболочки на поперечной оси должны лежать в chart с непустой внешней полосой. В ней winding всех периодических образов равен нулю. Неоднозначные endpoint contacts, цельные seam-coincident edges, неальтернирующие endpoints, несовпадающие противоположные endpoints или отсутствие этой полосы дают отказ без graph.

Непарные endpoints сортируются по поперечной координате. Соединяется incoming→outgoing; противоположный connector имеет точно обратные endpoints и отличается на authored period. Каждая исходная piece используется ровно один раз. После добавления connectors каждая локальная вершина должна иметь по одному входящему и выходящему edge; все edges разбираются на полные ориентированные циклы. max_edges 1..512 ограничивает source edges и connectors вместе; частичная сборка не публикуется.

Ориентированные connector pairs точно сокращаются по периодической идентификации, поэтому периодические boundary chains исходного региона и собранных циклов совпадают. Их winding-функции могут отличаться только константой; общая пустая поперечная полоса фиксирует её как ноль. Только после всех этих условий chart_winding_preserved=true и graph=Some. Случайная точечная проверка не используется для выдачи сертификата.

Это oriented boundary representation, а не сертификат регулярных B-rep faces: циклы могут разделять или пересекать исходные внутренние сегменты, если исходный регион так устроен. Для физических face/body admission нужны отдельные проверки простоты, согласования внутренних границ, sewing и поддерживающей поверхности. Одновременный wrap обеих осей, отсутствие доказанной anchor strip и tolerance-only path identity пока не поддержаны этой сборкой.

Проверка 2026-10-14: **69 tests passed** без default features, offline/locked в изолированном target: 22 periodic_trim, 36 curve_plane, 4 curve_self_intersection, 4 uv_closure, 3 uv_simplicity. Хеши пяти проверяемых файлов до/после совпали. Проверены U/V seam, outer loop с clockwise hole, рациональная окружность с исходными weights, полное edge coverage и exact endpoint closure, пары connectors, общий edge budget, contact refusal и отсутствие anchor strip. Независимый polygon winding oracle проверил материал, отверстие и внешние точки; он используется только как регрессионная проверка, не как сертификат алгоритма.

## Сертифицированный winding по двум осям

`inspect_periodic_winding(loops, chart, point, tolerance_uv, max_images, max_cells)` из periodic_trim/winding.rs принимает коллекцию точно замкнутых multi-curve loops. Возвращает целую сумму winding(point+k*period) по всем потенциально влияющим UV-образам. Положительные рациональные контрольные оболочки ограничивают область ненулевого winding; interval arithmetic даёт консервативный полный диапазон целых индексов. Вне этого диапазона вклады нулевые.

Каждый query rectangle содержит истинную вещественную точку, заданную исходными binary64 point, chart endpoints и целыми индексами. Period вычисляется интервальным вычитанием, включая непредставимый остаток; сдвиг не заменяется округлённой точкой. TrimDomain::classify доказывает постоянный winding на всём rectangle. Только после всех образов winding=Some(sum), reason=Complete. Частичная сумма не публикуется. Boundary/Join bands и precision limits остаются ClassificationUnresolved; исходный WorkLimit классификатора явно сохраняется как WorkLimit.

Ограничения: до 256 curves и 4096 spans суммарно; max_images 1..4096, max_cells 1..100000 общий на все классификации; индексы i32. required_images — число образов консервативного перечисления, None при переполнении представимого размера или до построения диапазона. tolerance_uv задаёт только неподтверждённую полосу около границы, не снапинг и не разрешение менять winding. Точка может лежать вне chart, если её image indices поддержаны.

Это основа для калибровки winding-константы при двухшовной сборке. Само наличие запроса не означает готовую сборку двухшовных границ или их соответствие 3D-поверхности.

Проверка 2026-10-14: **73 tests passed** без default features, offline/locked в изолированном target: 26 periodic_trim, 36 curve_plane, 4 curve_self_intersection, 4 uv_closure, 3 uv_simplicity. Хеши шести проверяемых файлов до/после совпали. Новые случаи: четыре угловых образа окружности, периодически сдвинутая точка, внешняя точка, точка на границе, открытый контур, бюджеты images/cells и clockwise hole. Для chart [0.1,1.1] независимо проверено рациональной арифметикой, что точный период authored binary64 равен 1+3/2^55; query enclosure сохраняет этот остаток, integer winding подтверждён.

## Двухшовная сборка с поправкой полного chart

`assemble_periodic_chart` из periodic_trim/double_chart.rs использует ChartAssemblyLimits: max_segments/max_links для точного RegionTransfer, max_edges 1..512 для source edges и boundary segments, max_images 1..4096 на каждый winding query, max_cells 1..100000 общий на source/candidate queries и положительный tolerance_uv для классификационной boundary band. anchor должен лежать строго внутри chart и иметь подтверждённый winding в обеих моделях.

Сначала собираются source pieces и точно противоположные connectors по обеим осям. Их периодические ориентированные цепочки совпадают с исходными, но заполнение может отличаться на константу. ChartCalibration сохраняет этот raw_graph, source/candidate PeriodicWindingReport и точную integer-поправку offset. Raw graph — диагностический промежуточный результат, не итоговое заполнение; его winding может быть дополнением нужного региона.

Периметр subdivides по общим уровням endpoints обеих противоположных сторон и corners. На каждом интервале к ориентированному коэффициенту raw connector прибавляется offset. Нулевые coefficients удаляются, ±1 дают ориентированные boundary edges, другая multiplicity остаётся неподтверждённой. Противоположные intervals имеют одинаковый коэффициент относительно CCW orientation, поэтому net boundary edges снова образуют точные периодические пары. После сокращения должны получиться полные замкнутые cycles с однозначными продолжениями incoming/outgoing edges и полной source-piece coverage. Для многоветвевых вершин используется точный angular ownership, описанный ниже. Только тогда chart_winding_preserved=true и graph=Some.

Это устраняет ошибку с дополнением региона около угла: CCW окружность вокруг двух швов имеет raw winding −1 в центре chart при исходном 0; offset=+1 даёт четыре заполненных угловых региона и пустой центр. Обратная ориентация требует offset=−1. Отверстия сохраняются суммой ориентированных winding contributions, без угадывания ролей loops.

Ограничения: unmatched corner endpoints, неоднозначные contacts, seam coincidences, неальтернирующие endpoint orders и boundary multiplicity остаются unresolved; raw calibration graph ограничен 256 curves и 4096 spans существующим winding classifier. Tolerance-qualified path modifications ещё не дают exact RegionTransfer. Результат — ориентированные локальные boundary cycles с winding certificate, не регулярные B-rep faces, не boundary ownership на всех швах и не сертификат соответствия 3D supporting surface.

Проверка 2026-10-14: **76 tests passed** без default features, offline/locked в изолированном target: 29 periodic_trim, 36 curve_plane, 4 curve_self_intersection, 4 uv_closure, 3 uv_simplicity. Хеши семи проверяемых файлов до/после совпали. Проверены положительная/отрицательная full-chart поправка, рациональные круговые regions через обе оси, oriented hole, одинаковый итог при anchor внутри материала и вне его, непубликация частичной сборки при edge/image budgets и явный отказ на неоднозначном corner endpoint. Итоговый winding дополнительно проверен в угловых regions, отверстии и центральной внешней области.

## Однозначные угловые переходы

Добавлен corner-aware raw closure в periodic_trim/double_chart/corner.rs. Уникальные incoming/outgoing ports упорядочиваются по четырём сторонам периметра, включая corners, без вычисления округлённой длины периметра. Рассматриваются два циклических соседних pairing offsets; выбранные gaps не перекрываются. Их signed coefficients переносятся на общие intervals противоположных сторон. Только при точном совпадении коэффициентов, допустимой кратности и бюджете создаются периодические connector pairs.

Для промежуточной калибровки raw graph может быть сбалансированной ориентированной цепочкой с несколькими edges в вершине: она разбирается на замкнутые диагностические cycles без изменения суммы winding. Это не назначение владельца итоговой грани. После winding correction и perimeter cancellation итоговый graph требует однозначного продолжения каждого edge. Сбалансированные многоветвевые source contacts теперь проходят отдельный angular ownership; неальтернирующие ports и неподдержанные multiplicities остаются unproven. Геометрически одинаковые +0 и −0 имеют одинаковый порядок ports.

Проверка 2026-10-14: **78 tests passed** без default features, offline/locked в изолированном target: 31 periodic_trim, 36 curve_plane, 4 curve_self_intersection, 4 uv_closure, 3 uv_simplicity. Хеши восьми проверяемых файлов до/после совпали. Новый положительный случай пересекает U/V одновременно в углу и собирается в три локальных цикла; проверены заполнение и пустые области, обратная ориентация и signed zero. Дополнительно проверен угол с crossing по одной оси и corner touch по другой (четыре цикла). Бывший corner-refusal fixture теперь доходит до проверки perimeter multiplicity и корректно отвергается там: неподдержанный winding не публикуется как graph.

## Проверка изотопии приближённого разбиения

В `periodic_trim/isotopy.rs` добавлен `verify_isotopy` для достаточной проверки всего линейного перехода H(t,s) между исходным и кандидатным lifted UV-контуром. Предусмотрены отдельные результаты для lifted и периодического пространства, общий бюджет ячеек и пар, проверки замыкания и непрерывного отклонения. Положительный периодический результат требует отделения оболочки всего перехода от периодических образов.

Проверка 2026-10-14: завершён исходный прогон **81 теста** (34 periodic_trim, 36 curve_plane, 4 curve_self_intersection, 4 uv_closure, 3 uv_simplicity). Хеши восьми файлов реализации совпали с предтестовым снимком. Затем добавлены две регрессии и повторно успешно выполнены **36 periodic_trim tests**. Проверены приближённый разрез с непредставимым корнем, сохранение отверстия, столкновение отверстия с внешним контуром во время перехода и коллапс двух простых конечных контуров в середине перехода. Дополнительные регрессии подтверждают явный отказ при jet/pair budgets и различие lifted/periodic результатов для сталкивающихся периодических образов.

Для каждой ячейки вычисляются interval hull всего H(t,s) и набор направлений со строго положительной проекцией производной обоих конечных путей. Поэтому эти направления монотонны для всего перехода. Каждая пара ячеек должна иметь разделённые hulls либо принадлежать одной петле с общей монотонной дугой между ними. Между разными петлями принимается только разделение. Неразрешённые пары адаптивно уточняются; общий бюджет учитывает также заменённые родительские ячейки и устаревшие запросы пар. Частичная proof partition при отказе не публикуется.

Ограничения: до 64 loops/256 pieces, max_deviation_cells 1..100000, max_jet_cells 1..4096, max_pairs 1..100000. Периодический сертификат пока использует достаточное условие: общая оболочка всего семейства уже каждого периода. Широкие безопасные регионы могут остаться неподтверждёнными. Изотопия сохраняет вложенность и ориентацию, но не равенство winding в любой неподвижной точке: граница может пройти через эту точку. Для калибровки приближённой сборки нужен дополнительный контроль anchor на всём переходе.

Модуль не повышает статус NURBS-0173: многоветвевые контакты ещё не завершены; supporting-surface correspondence и объединённый admission описаны ниже. Результат не подтверждает соответствие 3D-поверхности или готовность регулярных B-rep граней.

## Перенос winding в точке калибровки

`verify_anchor_winding` из `periodic_trim/anchor.rs` заново вычисляет изотопию по фактическим исходным кривым и разбиению. При подтверждённой периодической изотопии проверяет отделение каждого hull всего H(t,s) от всех потенциально пересекающих его периодических образов anchor. Диапазоны образов вычисляются интервально по точному authored периоду; общий max_checks 1..100000 ограничивает число сравнений. Касание hull или исчерпание бюджета дают явный неподтверждённый результат, а не winding. Anchor допускается вне chart с поддержанными i32 индексами.

Только после полного отделения вычисляется исходный `inspect_periodic_winding`. Тогда опубликованный integer winding относится и к исходному контуру, и к приближённому кандидату в этой точке. Исходный winding query имеет отдельные max_images/max_cells и tolerance_uv; эти допуски не разрешают перемещение границы через anchor. Результат содержит исходный query, изотопию и число проверок. Публичный caller-authored IsotopyReport на вход не принимается.

Это сертификат значения в точке калибровки; его подключение к сборке приближённых chart cycles описано ниже. Проверка 2026-10-14: все **38 periodic_trim tests** прошли без default features, offline/locked в изолированном target. Хеши нового модуля, isotopy, корневого periodic_trim и тестового файла до/после совпали. Проверены внутренний/внешний anchor, периодически сдвинутый anchor, общий бюджет сравнений и отказ для точки, которую граница пересекает во время движения.

## Сборка приближённого chart

`assemble_approximated_chart` в `periodic_trim/approximate_chart.rs` связывает UV admission, периодическую изотопию и перенос anchor winding с общим алгоритмом сборки швов. Результат `ApproximateChartAssembly` отделён от точного `ChartAssembly`: `candidate_winding_preserved` означает равенство winding итогового graph и приближённого кандидата вне его границы; равенство исходной и сдвинутой границы в каждой точке не утверждается. `transfer` сохраняет сертификаты изотопии и значения в anchor.

Помимо пересчёта сертификатов, проверяются точные равенства translation = tile * period и containment всех контрольных точек каждой положительно-взвешенной кривой. Публичный флаг candidate_hulls_contained и authored tile labels не считаются доказательством. Общий backend строит ориентированные source edges, замыкает их точными противоположными connectors, определяет integer winding correction и публикует graph только после полного сокращения периметра и замыкания циклов. Точный API использует тот же backend после собственного exact RegionTransfer.

`ApproximateChartLimits` задаёт отдельные budgets изотопии, max_checks для отделения anchor от всего перехода, max_edges для graph, max_images на каждый winding query и общий max_cells для исходного и raw-chain winding. Anchor строго внутри chart. Exact identity/link budgets не нужны приближённому admission. Ограничения общего backend на contacts, multiplicity и seam coincidences сохраняются; регулярные B-rep грани и соответствие 3D supporting surface не подтверждены.

Проверка сборки 2026-10-14: **88 тестов прошли** без default features, offline/locked: 41 periodic_trim, 36 curve_plane, 4 curve_self_intersection, 4 uv_closure, 3 uv_simplicity. Хеши файлов periodic_trim и тестового файла совпали со снимком на время компиляции. Новые проверки: приближённая сборка разреза с непредставимым точным параметром и ориентированным отверстием; правильное заполнение нескольких внутренних/внешних точек при трёх anchors; отказ при недостатке edges, поддельном tile и выходе контрольной оболочки за chart несмотря на сохранённый публичный флаг containment. Прежние exact assembly tests проходят после переноса в общий backend.

## Соответствие поддерживающей поверхности

`verify_surface_correspondence` в `periodic_trim/support.rs` проверяет фактическую Surface, исходные UV-кривые и разбиения. Chart обязан точно совпадать с активным доменом поверхности. Для каждой wrapped оси требуется достаточное точное C0-замыкание: одинаковые контрольные границы/веса при clamped storage либо точные повторения knot periods, controls и weights при periodic storage с непрерывностью у шва. Допуск в Surface::validate для внешних периодических узлов не заменяет этот сертификат. Неразрешённый шов, поддельные tile/translation и несовпадение chart дают явный отказ.

Для wrapped осей проверяется точное равенство translation = tile * (chart_end - chart_start) без предварительного округления периода. Для nonwrapped осей translations/tiles должны быть нулевыми, исходная положительно-взвешенная контрольная оболочка остаётся внутри chart. Контрольная оболочка каждого локального кандидата проверяется заново. UV admission вычисляется по исходным spans с общим max_deviation_cells; затем глобальные нормы производных поверхности в authored UV дают непрерывную границу world_error <= (Lu + Lv) * UV_error с наружным округлением. Полное C0-замыкание позволяет применить ту же границу через конечное число периодических швов. Превышение верхней границей tolerance_world означает «не подтверждено», а не доказанное нарушение допуска.

Исправлена `surface_parameter_bounds::inspect`: stable jets относятся к единичным координатам исходного knot span; нормы теперь делятся интервально на authored span width. Без пересчёта короткие spans могли занижать Lipschitz bound. Добавлена независимая affine-регрессия с точными нормами 16 и 20 на интервалах длиной 1/8 и 1/4.

SurfaceCorrespondenceLimits содержит отдельные tolerance_uv/tolerance_world, max_rectangles 1..10000 и общий max_deviation_cells 1..100000. Проверка не утверждает регулярность, инъективность поверхности, корректность B-rep граней или простоту пространственного образа. Проверка 2026-10-14: **8 supporting-surface tests**, **41 periodic_trim tests** и **3 surface_parameter_bounds tests** прошли без default features, offline/locked. Подтверждены переход через C0-шов квадратного цилиндра с независимой аналитической 3D-картой, два шва рационального тора, exact periodic storage, отказ при возмущении внешнего узла внутри допуска обычной валидации, несовпадение chart, поддельные tiles, nonwrapped translation, NaN, budgets и UV-допуск без world-допуска. Регрессия nonwrapped chart [0.1,1.1] не требует предварительно округлять период. Хеши критических исходников и тестов после повторного прогона совпали со снимком. Расширенный набор **11 thread tests** также завершился успешно, включая полный замкнутый контур крышки и mapping pcurve после исправления единиц производных. Всего подтверждены 63 различных теста в этих четырёх наборах; повторно выполнен окончательный supporting-surface набор из 8 тестов.

## Объединённый admission на Surface

`assemble_on_surface` в `periodic_trim/surface_chart.rs` принимает фактическую Surface, source/partitions, chart, wrapped axes, anchor и `SurfaceChartLimits`. Сначала пересчитывает supporting-surface correspondence. Только при WithinWorldTolerance запускает выбранный `SurfaceUvLimits::Exact` или `::Approximate` путь сборки. Затем проверяет, что ни один добавленный periodic connector не лежит на объявленной nonwrapped оси.

`SurfaceChartAssembly::graph()` возвращает итоговый граф только при `SurfaceChartStatus::Admitted`. Отказы SurfaceUnproven/UvUnproven/NonwrappedConnector сохраняют объяснение и доступную диагностику. `SurfaceUvAssembly::Exact` сохраняет точное исходное winding equality, `::Approximate` — candidate winding и source-to-candidate isotopy. Вложенный UV report диагностический и не заменяет итоговый surface admission. Общий граф не копируется и не публикуется повторно отдельным полем.

Бюджеты supporting-surface correspondence и UV-сборки независимые и явно представлены в limits; общий бюджет между этими стадиями не утверждается. Admitted не доказывает регулярность B-rep граней, инъективность Surface или отсутствие пространственных самоналожений её образа. Проверка 2026-10-14: **55 тестов прошли** без default features, offline/locked: 41 periodic_trim, 11 periodic_trim_support и 3 surface_parameter_bounds. Три новых native сценария проверяют точный/приближённый цилиндрический trim, заполнение обеих локальных частей и внешней области, сохранение различий Exact/Approximate, отсутствие итогового графа при world/seam отказе и недостатке UV edge budget. Хеши всех верхнеуровневых файлов periodic_trim, исходника bounds и двух trim test files до/после совпали. Это нативная проверка цепочки, без WASM/editor/B-rep qualification.

## Сбалансированные многоветвевые контакты

`inspect_contact_ownership` в `periodic_trim/contact.rs` проверяет balanced endpoint vertices ориентированной цепочки (до 512 clamped UV curves, max_ports 1..1024). Для многоветвевой вершины строит точный циклический порядок one-sided outward rays. Ray определяется первым отличающимся контролем в clamped endpoint span положительно-взвешенной рациональной кривой; сохраняется vanishing_order, поэтому нулевая первая производная сама по себе не вызывает отказ. Полностью stationary endpoint span остаётся unproven.

Направления не нормализуются и не вычитаются с предварительным binary64 округлением. Half-plane определяется точной суммой исходных координат; cross((a-o),(b-o)) вычисляется точной суммой восьми binary64 произведений. Это сохраняет знак при cancellation и underflow обычного determinant. При различных, строго чередующихся incoming/outgoing rays outgoing связывается со следующим CCW incoming: пары ограничивают ориентированный левый сектор. Совпадающие rays дополнительно разделяются по exact bend sides, описанным ниже; одинаковые bend sides и неальтернирующие ветви остаются неподтверждёнными.

Report содержит упорядоченные ContactPorts, leading control/vanishing_order, пары incoming→outgoing и полную successor permutation только после проверки всех вершин. Это точное назначение продолжений ориентированных рёбер, сохраняющее всю цепочку и сумму winding; оно не утверждает регулярные faces, простые cycles или назначение владельца nonzero-fill области при произвольной кратности.

Двухшовный общий backend теперь допускает сбалансированные source contacts, включая касание seam, и использует этот сертификат для итоговых cycles. Raw calibration chains по-прежнему допускают произвольное сбалансированное разложение, поскольку оно сохраняет summed winding. Unbalanced multibranch side ports теперь проходят flux closure, описанный ниже; corner ports, coincident rays и неподдержанная perimeter multiplicity остаются unresolved. Проверка 2026-10-14: **58 тестов прошли** без default features, offline/locked: 41 periodic_trim, 6 periodic_trim_contacts и 11 periodic_trim_support. Хеши файлов periodic_trim и trim test files до/после совпали. Подтверждены touching triangles в interior и на periodic seam, заполнение обоих регионов и пустых областей, точный порядок при underflow determinant, инвариантность при перестановке edges и signed zero, higher-order rational endpoint ray, отказы на coincident/nonalternating/stationary ветвях и port budget. Прежние exact/approximate и supporting-surface проверки проходят после изменения общего backend.

## Несбалансированные ветви на стороне шва

`periodic_trim/double_chart/flux.rs` строит raw connectors для нескольких endpoint ports на стороне шва без предварительного назначения пар исходным ветвям. Для каждой общей transverse level вычисляется целый net flux ends−starts на обеих противоположных сторонах. Fluxes должны быть противоположными. Накопленный CCW coefficient задаёт ориентированную кратность connector на следующем interval; orientation верхней V-стороны противоположна направлению роста параметра. После последнего level накопленный flux должен точно обнулиться.

Уровни включают все source boundary vertices, в том числе balanced contacts, чтобы последующие connectors делились в этих вершинах. Каждая копия connector имеет точно противоположную копию на другой стороне; integer multiplicity не скрывается снапингом или выбором произвольной пары. Все копии расходуют max_edges; raw calibration curve cap 256 сохраняется. Общий backend затем выполняет winding correction и perimeter cancellation и требует подтверждённого angular ownership итоговой цепочки.

Новый путь используется при multibranch seam ports. Исторический single-port/corner path сохранён. Несбалансированные corner ports теперь обрабатываются полным perimeter flux, описанным ниже; coincident/nonalternating итоговые rays и неподдержанная итоговая perimeter multiplicity остаются unproven. Проверка 2026-10-14: **60 тестов прошли** без default features, offline/locked: 41 periodic_trim, 8 periodic_trim_contacts и 11 periodic_trim_support. Проверены crossing region с отдельным touching triangle в общей seam vertex (source starts=1, ends=2), три положительных локальных цикла, заполнение обеих областей и пустых промежутков, V-axis coordinate exchange с отрицательным winding и отказ при max_edges. Отдельный V-тест выявляет неправильный знак flux при верхней CCW-стороне; окончательный повторный прогон после учёта direction(axis,1) прошёл. Хеши всех файлов periodic_trim, обоих corner/flux children и trim test files совпали со снимком.

## Многоветвевые контакты в углах

Flux closure обобщён на один CCW-обход всего периметра: bottom → right → top → left. В каждой вершине cumulative coefficient увеличивается на точный integer ends−starts. Угол посещается один раз, поэтому его net flux распределяется между двумя incident sides без предварительного выбора оси или пары. Последний coefficient должен замкнуться на исходный нулевой; все intervals противоположных сторон должны получить одинаковые CCW coefficients. Это необходимые проверки raw periodic cancellation.

Общая сетка содержит уровни всех source boundary vertices и зеркальные уровни противоположных сторон. Начальное значение coefficient задаёт только свободную whole-chart constant; последующий source/raw winding query по-прежнему определяет её точную integer correction. Материализуются все копии ненулевых connectors с общим max_edges, затем действуют прежние raw calibration cap, perimeter cancellation и angular ownership. Ошибка противоположных коэффициентов или бюджета не публикует graph.

Новый путь поддерживает unbalanced multibranch corner ports при подтверждённой последующей цепочке. Coincident/nonalternating конечные rays и неподдержанная итоговая multiplicity остаются unproven. Проверены corner crossing с отдельным touching triangle, положительный/отрицательный winding и ±1 full-chart correction. Проверка 2026-10-14: подтверждены **62 различных теста** без default features, offline/locked: 41 periodic_trim, 10 periodic_trim_contacts и 11 periodic_trim_support. Первый общий прогон включал 9 contact tests; после добавления origin/budget regression повторно прошёл окончательный набор из 10 contact tests. Подтверждены четыре положительных локальных цикла, сохранение отрицательного winding при обратной ориентации, заполнение crossing/touching regions и пустых промежутков, chart [[2,3],[-3,-2]] и явный edge-budget отказ. Хеши periodic_trim sources, corner/flux children и trim test files совпали со снимком.

## Совпадающие касательные с разными сторонами изгиба

`ContactPort` теперь сохраняет `bend_side` (−1 clockwise, 0 straight, +1 CCW) и `normal_vanishing_order`. Для каждого endpoint span проверяются точные signs cross(leading_ray, control−endpoint); первый ненулевой normal control определяет сторону изгиба. Положительные рациональные weights и положительные leading basis coefficients сохраняют этот знак в достаточно малой one-sided окрестности endpoint. Все предыдущие normal controls имеют точный нулевой знак; сравнение не заменяется sampled curvature.

Если leading rays совпадают, различные bend sides задают строгий локальный порядок clockwise → straight → CCW. Поэтому внешнее касание двух рациональных замкнутых контуров и касание прямой/изгибающейся ветви получают подтверждённые продолжения. Если совпадают leading ray и bend side, сравниваются radial bend exponents, описанные ниже; при равных показателях порядок остаётся unproven: различные кривые не упорядочиваются по номерам edges, величине контрольных координат или произвольному допуску. Это достаточный локальный сертификат, не глобальная проверка пересечений.

Проверка 2026-10-14: подтверждены **65 различных профильных тестов** (41 periodic_trim, 13 periodic_trim_contacts, 11 periodic_trim_support), без default features, offline/locked. После общего прогона из 12 contact tests повторно прошёл окончательный набор из 13. Проверены внешне касающиеся рациональные circles, точное сохранение исходных controls/weights в собранном graph, заполнение обоих регионов и внешних областей, straight/curved tangent contact и явный отказ для разных internally tangent кривых с одинаковой bend side. Хеши periodic_trim sources, corner/flux children и trim test files совпали со снимком.

## Одинаковая сторона изгиба: геометрический порядок касания

При одинаковых leading ray и bend side сравнивается точный radial bend exponent `(m-k)/k`, где k — порядок первого ненулевого касательного члена, m — первого нормального члена. `ContactPort::radial_bend_exponent()` возвращает integer numerator/denominator без вычисления дроби в binary64. Для positive-weight clamped NURBS локально radial distance ~ A*t^k, normal deviation ~ B*t^m, поэтому angular bend ~ signed_C*radius^((m-k)/k). Меньший показатель означает больший отход от луча на достаточно малом равном расстоянии от вершины; порядок меняет знак для clockwise bend. Сравнение выполняется integer cross-products orders <=25.

Это геометрический порядок ветвей, а не сравнение raw Taylor orders разных параметризаций. Он корректно различает regular branch с k=1,m=2 и cusp с k=2,m=3. Если и показатели совпали, regular k=1,m=2 ветви проверяются по interval coefficient, описанному ниже; остальные либо неразделённые коэффициенты остаются unproven; никакого fallback на edge index или sampled angle нет.

Проверка 2026-10-14: **68 различных профильных тестов** подтверждены (41 periodic_trim, 16 periodic_trim_contacts, 11 periodic_trim_support), без default features, offline/locked. После общего прогона повторно прошёл окончательный contact-набор с добавленной проверкой положительных rational weights. Новые случаи: четыре same-side tangent branches, показатели 3, 2, 1 и 1/2, clockwise reflection, независимые knot scales/weights и два точных degree-4 lens regions с аналитически известными границами и пустым промежутком. Полный общий backend сохраняет winding обеих областей. Хеши periodic_trim sources и trim test files совпали со снимком.

## Равный показатель: интервальный коэффициент регулярного изгиба

Для regular endpoint branches k=1,m=2 `ContactPort::regular_bend_coefficient` содержит наружный interval signed curvature/2 — коэффициент angular bend при равном radial distance. Он вычисляется по исходным clamped controls, weights и endpoint knot distances: `(p−1)/(2p) * h1/h2 * w0*w2/w1² * cross(r,q)/|r|³`. Входящая ветвь использует отражённый endpoint span. Все операции, norm и division округляются наружу; недостаточная numeric separation даёт отсутствующий coefficient, а не приближённое доказательство.

При равных leading ray, bend side и radial exponent порядок подтверждается только строгими gaps между coefficient intervals. Для сортировки используется transitive lower-bound key; overlapping intervals никогда не считаются транзитивным равенством в comparator. После сортировки проверяются все соседние gaps, включая корректное cyclic wrap, если все ветви лежат в одной asymptotic группе. Higher-order равные показатели теперь проверяются общими powered coefficients, описанными ниже; совпавшие/перекрывающиеся интервалы остаются unproven.

Проверка 2026-10-14: **71 различный профильный тест** подтверждён без default features, offline/locked (41 periodic_trim, 19 periodic_trim_contacts, 11 periodic_trim_support). После общего прогона повторно прошли все 19 contacts с дополнительной проверкой через assemble_on_surface. Новые случаи: четыре одинаковых exponent branches с положительными/отрицательными коэффициентами и cyclic wrap; отказ для коэффициентов с разницей в несколько ULP; неравномерные knots и rational weights с независимо известными коэффициентами 1/20 и 1/18, оба endpoint sides; внутренне касающееся oriented hole, сохранение материала/отверстия/внешней области и combined native surface admission на affine plane. Хеши periodic_trim sources и trim test files совпали со снимком.

## Общие ведущие коэффициенты для higher-order касаний

`periodic_trim/contact/coefficient.rs` расширяет coefficient admission на все доступные endpoint orders до степени 25. `PoweredBendCoefficient` сохраняет reduced denominator K и наружный interval `|C|^K`. Для m/k=M/K, radius ~ A*t^k и normal ~ B*t^m, angular coefficient C удовлетворяет `|C|^K=|B|^K/A^M`. Дробные степени и приближённые корни не вычисляются. Знак сохраняется отдельно в bend_side; ветви с одинаковым показателем сравниваются в одной общей степени K.

Leading basis coefficient вычисляется по исходным clamped knots и weights: `beta_i=(wi/w0)*binomial(p,i)*product(h1/hj,j=1..i)`, где t нормализован на первый оригинальный span. Это устраняет зависимость от абсолютного масштаба параметра. Все beta, tangent/normal norms и integer powers вычисляются interval arithmetic; numeric overflow/underflow или неотделённый denominator дают None. Strict gaps между signed powered intervals остаются обязательными; равные либо пересекающиеся интервалы не получают произвольного порядка. Существующая tighter regular curvature/2 формула сохранена как специальный случай.

Проверка 2026-10-14: **74 различных профильных теста прошли** без default features, offline/locked (41 periodic_trim, 22 periodic_trim_contacts, 11 periodic_trim_support). Три новых случая подтверждают four k=2,m=5 branches с coefficients power=2 и независимо известными `c²/100000`, positive/negative orientation и cyclic wrap; nonuniform cubic knot gaps/rational weights с coefficients 1/486 и 10/6561 на обоих endpoint sides; сборку двух degree-5 cusp lens regions с аналитически известным материалом и пустым промежутком. Хеши periodic_trim sources, нового coefficient child и trim test files до/после совпали.

## Взвешенные периодические цепочки и точное сокращение

`assemble_weighted_periodic_chart` в `periodic_trim/weighted.rs` использует exact RegionTransfer и тот же raw seam/perimeter/winding-calibration backend, но сохраняет произвольную целую кратность в пределах max_edges. Это отдельный `WeightedChartAssembly`: он подтверждает summed winding исходного региона, а не angular ownership или регулярные B-rep faces. Существующие owned APIs сохраняют свои проверки и отказы. Approximate weighted topology этим API не заявляется.

После калибровки материализуются все допустимые копии perimeter connectors, проверяется balanced closed chain и строится компактный словарь `WeightedChartGraph`. Одинаковые Curve encodings объединяются с integer coefficient. Противоположные encodings сокращаются только при точном совпадении reversed controls/weights, общем domain и точной knot-reflection identity через exact sums исходных binary64 чисел. Новая кривая отражением knots не вычисляется. При отрицательной net кратности берётся реально существующий исходный reverse encoding; нулевые группы удаляются. Пересчёт циклов выполняется по оставшемуся balanced flow.

`WeightedSource` сохраняет expanded edge ID, original loop/piece и orientation. Сохраняются provenance как оставшихся, так и отменённых групп и исходные connector-pair IDs. Их сумма покрывает каждое materialized source/connector edge ровно один раз. Loops используют dictionary edge с его integer coefficient; контрольные точки/веса final dictionary не копируются для каждого повторного обхода. Диагностический raw calibration graph остаётся expanded и ограниченным прежним cap. Structural compaction — достаточная проверка: геометрически равные кривые с другими неподтверждёнными parameter encodings не объединяются.

`inspect_weighted_winding` проверяет dictionary indices, коэффициенты и полноту loop uses, затем вызывает continuous periodic winding classifier. Query допускает до 256 expanded curves; assembly сохраняет общий max_edges 1..512. Пустая точно сокращённая цепочка имеет winding 0 и не требует image queries. Неполный бюджет не публикует graph. Общая регуляризация произвольно пересекающихся контуров в owned faces не утверждается.

Проверка 2026-10-14: **80 различных профильных тестов прошли** без default features, offline/locked (41 periodic_trim, 22 periodic_trim_contacts, 11 periodic_trim_support, 6 periodic_trim_weighted). Новые случаи: три совпадающих rational boundaries в одной dictionary curve с winding 3; exact opposite cancellation в пустую цепочку; отрицательная net кратность с исходным reverse encoding; perimeter multiplicity 2 с независимым polygon/image oracle; forged loop counts/indices и budget refusal; точное сокращение при domain [0.1,1.1], где предварительно округлённый a+b нарушает knot reflection. Хеши periodic_trim sources и четырёх trim test files совпали со снимком.
