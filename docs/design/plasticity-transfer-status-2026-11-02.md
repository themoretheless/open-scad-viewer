# Plasticity → собственные Rust-библиотеки: результаты, происхождение, остаток

> **Актуальный остаток на 2 ноября 2026:** [консолидированный план](plasticity-remaining-2026-11-02.md). Он имеет приоритет над устаревшими статусами в исторических записях ниже. Полный библиотечный паритет не завершён.

Исходный снимок сохранён 2 ноября 2026 года, Asia/Tbilisi, во время паузы.
После последующего явного возобновления пользователем цель «все реализовано
у меня» **активна и не завершена**. Полный паритет библиотек не доказан.

## Что означает «перенесено»

Воссоздаются функции и отдельные архитектурные подходы. Исходники,
декомпиляция, ABI, SDK-константы и проприетарные runtime в реализацию не
переносятся. Наши геометрические алгоритмы и доказательства написаны независимо
на Rust. Наличие одноимённого API или проходящих собственных тестов не означает
совпадения с поведением Plasticity.

Референс: установленная `/Applications/plasticity-beta.app`,
Plasticity Beta **26.2.0-beta.39**, ARM64 `pk.node`:
`Contents/Resources/app.asar.unpacked/.webpack/renderer/pk.node`.
SHA-256 повторно проверен при сохранении этого отчёта:
`5f00e0c6bcf0497e2e27674629492aeb37228c06dfad1ddda455e221c1ca7f1d`.

Различаем четыре уровня сведений: библиотека присутствует; найден импорт или
строка; подтверждена статическая цепочка вызовов; наблюдалось выполнение на
геометрическом примере. Последнего уровня сравнений сейчас нет.

## Откуда взята опора и что реализовано самостоятельно

| Источник в Plasticity | Что действительно наблюдалось | Наша реализация | Граница вывода |
| --- | --- | --- | --- |
| Native operation wrappers | Разделение нативных операций, async-обвязки, partitions/marks, транзакций и отображения | `independent-modeling/informed`: `Operation`, неизменяемые снимки, jobs, проверка ревизии, свои транзакции/undo, производный display cache | Архитектурная опора; не воспроизведение чужих внутренних классов или журнала |
| Parasolid / extrusion | Исторически подтверждён вызов `PK_BODY_extrude`, затем запросы геометрии/топологии и tracking | `independent-modeling/kernel-informed/src/brep.rs`: собственные retained BRep, поверхности, coedges, tracking и экструзия | Не восстановлен алгоритм Parasolid; нынешний `brep-core` — отдельный развиваемый Rust-владелец |
| Parasolid / loft | Исторически подтверждён `PK_BODY_make_lofted_body` | `kernel-informed/src/loft.rs`: свой common refinement, degree elevation и compatibility report | Математическое решение выбрано самостоятельно; алгоритм Parasolid неизвестен |
| Parasolid / Boolean | Исторический callsite `PK_BODY_boolean_2` | `kernel-informed/src/planar.rs`: собственное convex planar clipping; далее отдельные native Boolean-владельцы в основном ядре | Общий intersecting-NURBS Boolean не завершён; прототип convex clipping не равен Parasolid |
| Parasolid / faceting | Исторический callsite `PK_TOPOL_facet_2` | Производная сетка и связи с исходными гранями, отдельно от точной геометрии | Не перенос чужого тесселятора |
| Parasolid / Full Round | Свежая статическая цепочка `0x11218d8 → 0x1100f60 → PK_FACE_make_3_face_blend` | `brep-core::feature_ops`: cuboid/profile/grouped Full Round; новый `full_round_curved::build`; свои контакты, построение, G1/radius/envelope/closure/embedding/volume-проверки | Численные методы, правило положения crown и выбор локальной ветви — наши решения; одинаковое поведение Plasticity ещё не проверено |
| Parasolid / fillet, chamfer, offset, B-surface | Наблюдались соответствующие импорты `PK_EDGE_set_blend_*`, `PK_FACE_make_blend`, `PK_VERTEX_make_blend`, `PK_BSURF_*` и другие | Самостоятельные `profile_blend`, `edge_transition`, freeform offset/replace и constrained-surface семейства | Импорт не доказывает конкретный сценарий, алгоритм, домен или паритет |
| XNURBS, `libxnkernel.dylib` | Импорты семейства `VK_*`, включая constraints и `VK_generate_solution`; строки интерфейса `XNurbsSystem` | Собственные `constrained_surface::{network,rational,adaptive,unparameterized,variational}` и G0/G1/G2-проверки | Не восстановлен решатель XNURBS; соответствие оптимизации и охвата неизвестно |
| `libPolyhedralSplinesLib.dylib` | Импорты семейств `PnSpline_*`, `PnSPatch_*` для control mesh и патчей | Собственные quad topology/editing и реконструкция патчей | Не перенесён алгоритм PolyhedralSplines; extraordinary/nonprojectable случаи остаются открыты |
| Bodyshop, `libpsbodyshop.dylib` | Подтверждено присутствие нативной зависимости | В основном ядре есть свои healing/repair-владельцы | Для них сейчас нет подтверждённой таблицы «конкретная Bodyshop-функция → наш алгоритм»; Bodyshop не считать перенесённым |
| Конвертеры, `libA3DLIBS.dylib`, `libhepb.dylib` | Наблюдались зависимости пакета | Самостоятельные STEP/IGES и другие native exchange-модули | Наличие зависимостей не устанавливает их точное назначение и поведение каждой операции; whole-library parity не доказан |

Исторические extrusion/loft/Boolean/faceting-пойнтеры и ограничения:
[kernel-informed/PROVENANCE.md](../../crates/independent-modeling/kernel-informed/PROVENANCE.md).
Свежие Evidence ID и полный Full Round trace:
[plasticity-full-round-reference.md](plasticity-full-round-reference.md).
Для Full Round dossier: `ev_1f5d91000627dbc8294f2eb5afea31e87a8e4a1b4b8b867c1157e81759cd2a75`
и `ev_a3cb29966d5b8470b27664a01ebf5ecbc014d09545784b072626c577bba8d4a0`.

Полные наблюдения хранятся вне исходников:
`/Users/themoretheless/.codex/research/plasticity-beta-26.2.0-beta.39/`:
`desktop-and-selected-js.json`, `pk-mach-o.json`, `fullround-static-trace.json`.
Их наличие проверено. Они не являются входом Cargo-сборки.

## Сохранённые версии

| Версия | Роль и состояние |
| --- | --- |
| `independent-modeling/clean` | Замороженная база для сравнения. `verify_clean.py`: **11 файлов неизменны**. Это локальный контроль идентичности, не внешний аудит формальной clean room |
| `independent-modeling/informed` | Отдельный архитектурный прототип с собственным document/job/transaction API |
| `independent-modeling/kernel-informed` | Отдельный геометрический прототип retained BRep/loft/convex clipping |
| `nurbs-core` + `brep-core` | Основное развиваемое native-ядро; поздние реализации добавлялись сюда. Оно не объявляется полным клоном перечисленных бинарных библиотек |

## Что уже есть в основном ядре

Это инвентаризация по сохранённому плану и текущим исходникам. Все старые
семейства заново целиком не прогонялись при сохранении отчёта.

| Семейство | Реализованная часть |
| --- | --- |
| Пересечения | Original-source fibers, mixed-coordinate transitions/components, закрытые физические ветви с C1/rank/interior/injectivity gates; actual-cap cuts, замкнутые контуры, разделение областей, imprint/conform/sewing |
| Fillet/chamfer/reblend | Прямые и круговые spines в допущенных семействах, рациональные профильные опоры, authored radius patches, целые G1/radius/envelope-проверки; распознаваемые constant-radius reblend chains |
| Offset/replace/thicken | Исходные normal-offset expressions, непрерывные displacement/regularity bounds, constrained replacement и closed cap/wall networks, planar/miter/mapped/quadrilateral варианты |
| Quad | Согласование граничных циклов, удаление quad strips/rings и disk-star регионов, ограниченная реконструкция с картами исходных элементов и manifold-проверками |
| Constraints/G2 | Hard point/jet constraints, mapped-boundary и рациональные G0/G1/G2 сети; положительные веса, адаптация degree/knots, inferred UV, variational/load направления с независимым replay |
| Pattern/history | Named feature occurrences, affine patterns и lineage; mixed edge split/seam/compact/reclock history, replay, rebasing, prefix branches и миграция `/1 → /2` |
| Надёжность | Раздельные владельцы edge/carrier agreement, injectivity, pair contacts, boundary embedding, shell nesting/roles и physical orientation; незакрытые доказательства дают отказ |

Полный подробный ledger:
[plasticity-library-gap-plan.md](plasticity-library-gap-plan.md).
Этапы до r80 и их исторические проверки:
[plasticity-parity-checkpoint-2026-10-31.md](plasticity-parity-checkpoint-2026-10-31.md).
Например, r80 сохраняет 76 выбранных тестов прежнего снимка; их нельзя выдавать
за новый полный прогон текущего worktree.

## Исторический этап: Full Round на криволинейных опорах

Добавлены:

- `nurbs-core::offset::profile_corner::full_round::propose`: совместный поиск
  двух параметров исходных кривых и радиуса. Аналитический Jacobian,
  ограниченный Newton/descent, сохранение ограничений исходных кривых и
  непрерывная оценка погрешности их фактического trim.
- `brep-core::feature_ops::full_round_curved::build`: явный отдельный API
  построения тела на выбранных исходных гранях. Владение устанавливается по
  исходным рёбрам, а не по близости поверхностей. Сохраняются исходная модель,
  предложение контактов и фактически построенный результат.
- Обязательные проверки трёх продольных G1-стыков, радиуса и радиальной
  нормали обеих дуг, непрерывного boundary agreement и solid incidence audit.
- Дополнительные явные `qualify_embedding` и `qualify_volume` дают неизменяемые
  типизированные результаты только после полной проверки соответствующего
  уровня. Развёрнутая внутрь оболочка не проходит подтверждение материала,
  даже если её граница остаётся внедрённой.

Поддержаны проверенные полиномиальные и рациональные профильные опоры.
Текущий домен нового построения: одна XY-призма, один body/shell, три различные
последовательные выбранные грани, прямой middle и две исходные кривые опор,
положительные веса и локальная ветвь; две составные круговые дуги, все активные спаны которых проверяются отдельно.
Это не произвольный пространственный Full Round. `propose` сам по себе не
является root certificate, полным census ветвей или допуском готового тела.
Полное отношение удаления материала относительно всех удаляемых исходных
участков также не следует только из G1/radius/volume-проверки результата.

## Проверка сохранённого снимка до многоспанового расширения

| Наборы | Прошло |
| --- | ---: |
| NURBS: `profile_full_round`, `freeform_geometry` | 10 |
| BRep: `feature_ops`, `full_round_curved`, `full_round_embedding`, `full_round_embedding_prism`, `full_round_frame`, `full_round_groups`, `full_round_prism` | 16 |
| BRep: `full_round_curved_volume` | 1 |
| **Итого без повторного подсчёта перезапусков** | **27** |

Прогон, который оставался запущенным при паузе (session 6061), теперь проверен:
**terminal exit 0**, 7 + 3 теста прошли. Новая реализация на криволинейных
опорах проверена до положительного embedding/material-volume результата для
обоих тестовых вариантов весов. Неправильный выбор исходных граней и
непомещающиеся контакты проверены отрицательными случаями.

Сохранены журналы, хеши исходников и receipt:
[full-round-curved-2026-11-02/results.json](../qualification/full-round-curved-2026-11-02/results.json).
Это focused qualification, не полный прогон проекта и не сравнение с Plasticity.
Worktree содержит параллельные незавершённые изменения; они не присваиваются
этому этапу. Последний architecture gate имеет существующую ошибку:
`src/services/mainSolidProtocol/mainSolidResult.ts`: 595 строк при pin 594.
Лимит не увеличивался. Чистые 11 файлов сохранены.

## Что осталось сделать после возобновления

1. **Сопоставление с референсом.** Для каждого native семейства закрепить
   конкретный вход Plasticity, свой API и одинаковый геометрический пример.
   Проверить успехи, отказы, topology/geometry и допуски. Статические импорты
   XNURBS/Bodyshop/конвертеров не заменяют эти проверки.
2. **Пересечения и Boolean.** Полный census скрытых ветвей/касаний,
   периодические области и полюса, mixed-axis world/UV endpoints, автоматические
   closed BRep trims, общий intersecting-NURBS Boolean и material admission.
3. **Общие fillet/chamfer/Full Round.** Пространственные некруговые spines,
   общие криволинейные опоры и группы, variable-radius сети, несколько
   пересекающихся blends, higher-valence corners, support extension/retrim,
   общие ветви с overhang/самоконтактами, глобальный выбор ветви и отношение
   удаления/сохранения исходного материала. Новый XY-случай их не закрывает.
4. **Offset/replace.** Автоматическое открытие общих ветвей/окон и расширение
   опор за charts, произвольные trims/periodic/poles, self-trimming и изменение
   топологии при collapse, глобально корректные closed-solid оболочки.
5. **Quad/constraints.** Nonprojectable/non-disk реконструкция, дополнительные
   controls и extraordinary neighborhoods, перенос авторских ограничений при
   edits/basis changes, совместный глобальный поиск UV/shape/weights/basis и
   nonlinear equilibrium, сложные gauges и material/follower forces.
6. **Pattern/history/exchange.** Перенос внутренних полостей, seams между
   разными опорами, общие Boolean-пары, остальные семейства истории,
   dependency/branch merges, ModelGraph integration, расширенный STEP/IGES
   roundtrip с сохранением исходных связей и квалификация конвертеров.
7. **Общая поставка.** Актуальные полные regression/architecture gates,
   интеграция библиотечного результата в основной pipeline и итоговая внешняя
   проверка. Native slice, отдельный crate и локальный receipt не означают
   завершённую интеграцию или весь паритет.

Сравнение на запущенной beta ранее остановилось на окне лицензии с ошибкой
`fingerprints/0 type mismatch (received integer expected string)`.
Моделирующие операции в Plasticity тогда не выполнились. Лицензирование не
изменялось; runtime equivalence остаётся неизвестной.

При следующем возобновлении сначала прочитать этот отчёт, detailed reference
и ledger, проверить актуальные исходники/процессы, затем продолжать открытый
scope. Историческая пауза после этого снимка отменена явным возобновлением
пользователем; последующие результаты добавляются отдельными receipts.

## После явного возобновления: многоспановые дуги

Снято ограничение на два односпановых круговых участка. Построение перечисляет
все активные спаны обеих дуг, а квалификатор проверяет каждую фактическую грань
и каждый продольный стык. `all_edges()` даёт полную цепочку; `edges()` сохраняет
контакт/crown/контакт для совместимости. Общие лимиты не увеличивались.

Новый прогон: **17 BRep-тестов прошли**. Внутри volume-регрессии проверены шесть
вариантов полиномиальных/рациональных опор, симметрия и асимметрия, разбиения
дуг 1+1, 2+2, 2+1 и 1+2 до полного подтверждения объёма. Это не 17 дополнительных
к прежним 27: наборы частично перекрываются. Receipt и текущие хеши:
`docs/qualification/full-round-multispan-2026-11-02/results.json`.

Пространственные опоры, полный поиск ветвей, отношение удаления исходного
материала и runtime parity остаются открыты. Цель активна; полный scope сохранён.

## Непрерывная оценка новых дуг относительно исходного профиля

Добавлены `trim_domain::curve_cover::inspect` и
`CurvedFullRound::qualify_source_arc_cover`. Для каждого параметра фактического
нового ребра подтверждается положение внутри исходной winding-области либо
расстояние не более `source.tolerance_mm` до точки исходной границы. Принимаются
целые интервалы, а не выборка точек; непокрытые интервалы и исчерпанные лимиты
не дают сертификата. Исходные BRep-рёбра читаются с реальными часами, без
нормализующего деления; разворот сохраняется точным отражением знака параметра.

Новый прогон: **30 тестов прошли** (12 NURBS + 17 BRep + 1 проверка исходных
часов и развёрнутого ребра). Это не добавка к прежним 27/17: наборы перекрываются.
Сохранены source/result snapshots, interval certificates, логи и хеши:
`docs/qualification/source-arc-cover-2026-11-02/results.json`.

Это граница отклонения новых боковых дуг, **не доказательство точного включения
всего результата в исходный материал**. Полное signed material partition /
subtraction, общие пространственные опоры и runtime equivalence остаются открыты.


## Дополнение: точное тело и исходные BRep-рёбра

Новая рациональная ветка Full Round строит G0-контур с неизменёнными исходными
участками, проверяет регулярность/G1 с объявленным допуском, Jordan-пары,
ориентацию и вложенность; затем допускает выдавленную оболочку как Solid.
Теперь исходные участки привязаны к реальным coedge/edge ID и исходным knot-
шкалам с точным переносом параметров. Последний этап: 11 тестов прошли,
включая допуск новой геометрии в шести объёмных вариантах.
См. `../qualification/full-round-raw-binding-2026-11-02/results.json`.

Открыто: связь со всей исходной опорной поверхностью, отношение исходного и
результирующего материала, радиус на новой геометрии и интеграция. Старый
округлённый Model не заменён этой веткой и сохраняет известный отрицательный
результат строгого включения. Полный первоначальный scope активен.


## Точные UV-границы на исходных крышках

Добавлены точные обратные отображения исходных мировых рёбер на неизменённые
аффинные и билинейные поверхности крышек. Для аффинного случая сохраняются
рациональные UV-коэффициенты, для общего билинейного — неявное выражение с
гарантированными рациональными интервалами. Исходная модель, рёбра, поверхности
и старые UV-данные не изменяются; прежняя погрешность сохранена отдельно.
Замкнутость, простота и вложенность новых границ проверяются на исходных
мировых контурах. Десять тестов прошли, включая разные исходные параметрические
шкалы, развёрнутое ребро и перестановку/разворот UV крышки. Все 11 файлов
чистой версии неизменны.
Receipt: `../qualification/full-round-cap-pullbacks-2026-11-02/results.json`.

Далее: совместимость нижних рёбер с боковыми поверхностями, полная исходная
граница тела, отношение исходного/нового материала и точность радиуса. Этот
этап не закрывает общий пространственный Full Round или полный паритет.


## Нижние рёбра и исходные боковые поверхности

Добавлен `SourceSupports::qualify_lower_rims`: все нижние рёбра привязаны к
единственным исходным боковым граням; точное совпадение образов проверяется по
всей кривой, на реальных параметрических шкалах. Полный перечень рёбер крышки
должен покрывать нижние границы всех боковых поверхностей. Исходные данные
не изменяются. Восемь тестов прошли, включая отказ при малом отклонении внутри
нижнего ребра, которое ещё допускается обычной проверкой модели.
Receipt: `../qualification/full-round-lower-rims-2026-11-02/results.json`.

Далее: вертикальные рёбра/стыки с исходными боковыми поверхностями и полная
исходная граница тела; затем отношение материалов и точность радиуса новой
геометрии. Совпадение образов нижних рёбер не означает равенство параметризаций
или завершённую сертификацию исходного Solid. Полный scope остаётся открытым.


## Вертикальные стыки и полный перечень исходных рёбер

Добавлена точная проверка вертикальных рёбер относительно обеих исходных
боковых поверхностей: XY, проективная высота, концы, исходные вершины и
противоположность обхода. Полный поиск корней производной исключает остановки
и обратный ход, включая чётные стационарные корни. Верхние, нижние и вертикальные
привязки должны покрывать все исходные рёбра без пересечений перечней.
Двенадцать тестов прошли (9 support + 3 vertical); 11 чистых файлов неизменны.
Receipt: `../qualification/full-round-vertical-seams-2026-11-02/results.json`.

Далее: регулярность и инъективность исходных боковых профилей на фактических
trim-окнах, согласованная ориентация носителей и полная сборка исходной границы.
Полный перечень рёбер ещё не является сертификатом исходного Solid. Отношение
материалов, радиус новой геометрии и весь первоначальный scope остаются открыты.


## Регулярность и ориентация исходных боковых носителей

Добавлен `qualify_native_profiles`: проверяются фактические профили исходных
поверхностей на индексированных trim-окнах, направленные концы верхних рёбер,
регулярность спанов, точная касательность во внутренних узлах, простота контуров,
вложенность отверстий и наружная ориентация носителей. Авторские углы между
отдельными исходными рёбрами допускаются. Коэффициенты/степень/узлы исходной
поверхности сохраняются; повышенная степень проверена отдельным утверждением.
Тринадцать тестов прошли; 11 файлов чистой версии неизменны.
Receipt: `../qualification/full-round-native-profiles-2026-11-02/results.json`.

Далее: направленные концы нижних рёбер/исходные вершины, регулярность мировых
рёбер крышек и полная сборка исходной границы. Совпадение образов само по себе
не доказывает направленное соответствие концов. Затем остаются отношение
материалов, радиус и весь первоначальный открытый scope.


## Полная исходная граница и отдельный эталон материала

Добавлен `qualify_source_boundary`: объединяет проверки исходных крышек,
боковых профилей и вертикальных стыков на одной неизменённой модели. Проверены
направленные концы нижних рёбер, точные исходные вершины, регулярность мировых
рёбер крышек, полный перечень граней и два противоположных использования каждого
ребра разными гранями.

`canonical_material` строит отдельное точное рациональное тело-эталон из
проверенного исходного профильного региона и высоты. Это явная канонизация
вспомогательных UV-границ на исходных носителях; старые pcurve и исходная модель
сохранены. Равенство старых ненулевых UV-стыков не утверждается. Сквозное
отверстие проверено как отверстие региона одного тела, без отдельной полости.
Семнадцать focused-тестов прошли; 11 чистых файлов неизменны. Начальные ошибки
тестовых предположений/масштаба сохранены в логах, проверки ядра не ослаблены.
Receipt: `../qualification/full-round-source-boundary-2026-11-02/results.json`.

Далее: сравнение исходного и нового материалов на НОВЫХ точных рациональных
Wire, с этим явным исходным эталоном; старый округлённый отрицательный пример
сохранить отдельно. Затем точность радиуса/касательности новых мостиков.
Общие пространственные семейства, pipeline/WASM, полные регрессии и внешний
паритет остаются открытыми в первоначальном scope.


## Точное сравнение нового материала: обнаружено добавление

Добавлен движок сравнения точных рациональных Wire без округления коэффициентов
в f64 и API `inspect_exact_source_material`. Полный учёт пересечений/общих
участков, точные поперечные свидетельства и winding проверены шестью тестами,
включая обе реальные модели Full Round, повторную проверку сертификатов,
касание, скрытые пересечения, заполнение отверстия и полосу шириной 2^-45.

Новая точная ветка тоже пока добавляет материал: **7 свидетельств** при весе 1
и **4** при весе 0.7; это не число отдельных объёмов. Сохранены точные координаты
свидетелей с winding исходного/результата [0,1]. Сертификат отсутствия добавления
правильно отвергается. Старый отрицательный пример округлённого Model не заменён.
Receipt: `../qualification/exact-wire-material-2026-11-02/results.json`.

Далее: исправить геометрию/касательные рациональных мостиков при неизменённых
исходных участках, затем повторно доказать включение материала и точность
радиуса. Одной корректировки концов и G1 с допуском недостаточно. Полный
первоначальный scope, общие пространственные семейства, pipeline/WASM и внешнее
сравнение остаются открытыми. Чистые 11 файлов сохранены.


## Исправлены касательные и вершина новых мостиков

Новая рациональная ветка сохраняет исходные участки и веса, пересчитывая
внутренние контрольные точки по точным пересечениям касательных. Вершина
мостиков точно привязана к фактическому конечному исходному среднему сегменту.
Допуск перемещения проверяется по всем контрольным точкам и ограничивает всю
кривую. G1 на изменённых стыках теперь проверяется с нулевым угловым допуском.
Добавлен непрерывный Bernstein-сертификат отклонения радиуса для каждого нового
спана, в пределах исходного tolerance/32; ненулевая оценка не объявляется
тождеством аналитической окружности.

Строгое включение материала прошло для обеих симметричных моделей: добавленных
свидетельств 0 при весах 1 и 0.7. Старый вариант с одной коррекцией концов в той
же регрессии по-прежнему даёт 7 добавленных свидетельств и отвергается.
Десять тестов прошли, включая шесть объёмных вариантов, разные исходные часы и
развороты. Чистые 11 файлов неизменны. Исходники, логи и этапы исправления:
`../qualification/full-round-exact-tangent-radius-2026-11-02/results.json`.

Далее: строгая material-проверка остальных асимметричных/многоспановых вариантов
(объёмная регрессия сама её не заменяет), затем подключение новой допущенной
рациональной геометрии к основному pipeline. Старый округлённый Model пока
отдельный результат и не авторизуется новыми сертификатами. Общие пространственные
семейства, сети скруглений, WASM/полные регрессии и весь первоначальный паритет
остаются открытыми.


## Строгая проверка всего текущего семейства и точный библиотечный API

Все шесть симметричных/асимметричных полиномиальных/рациональных случаев с
разбиениями дуг 1+1, 2+2, 2+1 и 1+2 прошли строгую проверку: добавленных
свидетельств 0, сертификаты повторно проверены. Радиус на всей кривой и точная
G1 также проверены для каждого результата.

Добавлен `feature_ops::full_round_exact(&source, faces, Default::default())`.
Он возвращает именно сертифицированную рациональную топологию с материалом,
касательностью и радиусом; исходная модель сохраняется. Недостаточный бюджет
или неверные грани дают отказ, а не старый округлённый результат. Сквозное
отверстие отдельно прошло строгий допуск и сохраняется в одном теле.
Девять focused-тестов прошли (6 family + 3 API); 11 чистых файлов неизменны.
Receipt: `../qualification/full-round-admitted-family-2026-11-02/results.json`.

Далее: перенос рациональной топологии с привязкой доказательств в
ModelGraph/WASM/основной pipeline, исходные topology ID/lineage и обмен с
roundtrip. Новый API пока отдельный native-вход; старые `full_round` и
`CurvedFullRound::model()` автоматически не заменены. Общие пространственные
семейства, сети скруглений и полный первоначальный паритет остаются открытыми.


## Точный перенос native ↔ WASM без повторного численного решения

Добавлен замороженный рецепт: исходные natural-clock интервалы, исходные
мостики, точные целевые центр и радиус. При восстановлении Rust заново строит
рациональную геометрию и доказывает материал/G1/радиус, не повторяя численный
Newton/тригонометрическое построение. Все шесть семейств и разные исходные
шкалы/развороты воспроизводят идентичную точную топологию.

Lossless-форматы сохраняют рациональные коэффициенты и узлы парами целых строк.
Публичный общий router получил `cad_full_round_exact` и
`cad_full_round_exact_restore`. Присланные коэффициенты/флаги не авторизуют
результат: входной рецепт перепроверяется, а результат сравнивается целиком.
Подмена данных, исходных интервалов и недостаточные бюджеты отвергаются.

**12 Rust-тестов + 6 случаев реального WASM ABI прошли.** Свежий import-free
WASM восстановил native-артефакт с идентичной рациональной геометрией, выполнил
собственное построение и JSON roundtrip, отверг три вида подмены. Хеш модуля:
`69eb251798727f99df9c2a5b88691a56b5db3ce4ac2122edadc565a06af2adf6`.
Receipt, native-артефакт, исходники и логи:
`../qualification/full-round-frozen-recipe-transport-2026-11-02/results.json`.
Чистые 11 файлов неизменны. Опубликованный kernel приложения не заменён;
проверялся отдельно собранный модуль с актуальными операциями.

Далее: типы/узлы и результаты ModelGraph для рациональной топологии,
исходные topology ID/lineage, потребители точных данных (display/edit/exchange),
поставка source-bound kernel и проверка worker pipeline. Общие пространственные
семейства и полный первоначальный scope остаются открытыми.


## IDs точного результата и происхождение граней

Добавлены IDs всех сущностей и `lineage` граней по фактическим индексированным
связям исходной модели. Полностью сохранённая грань с одним потомком удерживает
исходный face ID; обрезанные/разделённые грани и крышки получают новые ID и
валидный ChangeSet. Для исходных участков сохраняются rim/loop ID, coedge и
точные диапазоны в ориентированной и исходной stored-шкале, включая развороты.
Для мостиков записаны исходные выбранные грани и номера bridge/span.

14 Rust-тестов и 8 случаев реального WASM ABI прошли. IDs и родители совпадают
между native и WASM; подмена lineage отвергается. Старые v1-артефакты без
необязательных метаданных обновляются только после полной перепроверки геометрии.
Источник, snapshot, native-артефакт, хеш WASM и логи:
`../qualification/full-round-source-lineage-2026-11-02/results.json`.
Чистые 11 файлов неизменны; опубликованный kernel не заменён.

Открыто: IDs рёбер/вершин/петель пока генерируются заново, полная история их
происхождения и устойчивость при дальнейших edits/occurrences не заявлены.
Далее — явный exact-rational carrier и операции ModelGraph, сохранение Rust
body/recipe при передаче результатов, worker budgets и source-bound поставка.
Существующий f64 Model не принимает точную рациональную топологию автоматически.
Общие пространственные семейства и весь первоначальный scope остаются открыты.


## Native DAG: точный Full Round как результат графа

Добавлен unary-узел `full-round-exact` в native semantic DAG и существующий
carrier `rational-brep`/`representation-preserving`. Результат — точная топология
с полным frozen artifact/IDs. Перед публикацией lease выполняется свежая native
проверка рецепта, материала, G1 и радиуса. Бюджеты считают весь артефакт; ложная
смена типа, недостаточный material-бюджет и старые операции на точных данных
дают отказ без преобразования в f64 Model.

15 Rust-тестов (4 integration + 11 Session regression) и 6 реальных WASM ABI
случаев прошли. Уточнены тестовые ожидания: неуспешная прямая сессия удерживает
прежние записи до закрытия/abort, но не может опубликовать/закоммитить новый
результат; публичный graph runner откатывает всё. Контракт транзакций не ослаблен.
Receipt: `../qualification/full-round-native-graph-2026-11-02/results.json`.
WASM: `20c3db1d2ae0a1226a182b1bbef51afadc2d7510e4bbc41ed3c3fb512a427f21`.
Чистые 11 файлов неизменны; опубликованный kernel не заменён.

Далее: высокоуровневые union/schema/capability closure и lowering ModelGraph /
SemanticProgram (новый operation kind ещё не объявлен), отдельный NURBS graph
resolver, точные result/display/edit/exchange-потребители. Строгую envelope-
проверку не обходить. Затем история edge/vertex/loop IDs, source-bound поставка
и worker/full regression. Общие пространственные семейства и первоначальный
scope паритета остаются открытыми.

Native envelope qualification: 5 graph integration + 8 envelope regression tests passed. Receipt: `../qualification/full-round-native-envelope-2026-11-02/results.json`. TS schemas/lowering, native occurrence-production and packaged WASM remain open.


### Native Full Round operation production — 2026-11-02

Added authored `geometry:full_round_exact` → `full-round-exact` map production.
Every output consumes its matching ordered child frontier, all outputs of one
occurrence share face/limits/maxBytes parameters, and the feature result owns
its own scene identity. Face selection must contain three unsigned integers.
Native fixture IDs are re-derived from actual structural paths.

16 tests passed (3 new + 13 existing production regressions). New coverage
includes wrong input/identity/kind and malformed faces, plus an actual graph
construction with complete operation/occurrence identity rows and capability
closure, returning rational geometry and the feature-owned result identity.
Receipt: `../qualification/full-round-native-production-2026-11-02/results.json`.
The 11 clean files remain unchanged.

The new fixture is authored natively, not lowered by the TS frontend. Host node
union/schema/producer names/production replay and language lowering remain
open, followed by exact consumers, new WASM/package/worker qualification and
the complete original parity scope. Source-text attestation is not claimed by
this fixture.


### Host Full Round contract and real source-to-native execution — 2026-11-02

Added `SemanticFullRoundExactNode`, unary dependency/remapping, producer name
and capability closure; strict brep-1 validation requires an original analytic
solid and rational solid output, distinct indexed faces and bounded optional
material budgets. Normal and interrupted host map replay recognize the new
operation with canonical shared parameters and new result identity. Added
`full_round_exact(faces=[5,4,3]) cube([4,8,6]);` source lowering, including user
modules and binary codec roundtrip.

The public trusted lowerer generates the saved full envelope and native request.
Rust admits the actual source text/hash, operation/occurrence IDs, capability
closure and execution plan, constructs exact geometry, and refuses altered
source text or omitted capability. 279 host tests, both TS typechecks and
22 Rust tests (16 production + 6 graph/source) pass. Receipt:
`../qualification/full-round-host-contract-2026-11-02/results.json`.
11 clean files unchanged. Common map parameters and shared DAG remapping are
extracted to dedicated owners; all changed oversized files respect existing
pins. Global architecture still fails on five other recorded worktree files;
no pins were raised.

Exact transport now has a separate rational type. Existing f64 scene display
explicitly refuses it; this is an open consumer, not completed display support.
New WASM/package/worker delivery, exact display/edit/exchange, downstream exact
operations, separate NURBS graph integration, broader interrupted/multi-output
qualification and the original complete parity scope remain open.


### Source-owned rational Full Round display sampling — 2026-11-02

Added `prism::display::tessellate` and `cad_full_round_exact_display`. The public
boundary reconstructs/proves the frozen artifact, evaluates the actual rational
controls by exact De Casteljau and rounds only final display points. It never
creates or substitutes a legacy f64 BRep. Shared cap/wall samples preserve
through-holes and topology face ownership. Bounded sampling/triangulation,
binary64 collision, closed/oriented mesh and existing Float32 collapse/reversal
checks can refuse the display result. Source/body coefficients remain exact.

Four native integration tests passed: polynomial/rational profiles at 2/8/16
samples, through-hole cap preservation, resource/forgery refusals and independent
analytic rounded-box volume convergence. Receipt:
`../qualification/full-round-exact-display-2026-11-02/results.json`.
11 clean files unchanged; API map 242/200 passes, not parity. No architecture
pins changed; the same five other recorded worktree files keep the global gate
red. A TS transport helper is separate from the application's existing scene.

Uniform display sampling has no Hausdorff error certificate, self-intersection
proof or exact mass certificate. It is qualified only for this admitted exact
extrusion family. The scene builder still refuses exact output pending exact
artifact snapshot/edit ownership. Fresh WASM/package/worker delivery, broader
rational display, downstream exact operations/exchange and the entire original
parity scope remain open.


### Actual WASM source and exact display qualification — 2026-11-02

A fresh import-free WASM module passed 12 actual ABI cases: source-attested
Full Round graph construction; display at 2/8/16 samples; independent rounded-box
volume convergence; restoration and display of the native rational frozen
artifact with identical exact geometry/IDs; invalid sampling/triangle budgets,
forged geometry, changed source text and missing capability refusals.

WASM SHA-256: `da45c8d325c8295264649e09f075920ba5f4950cc1fd043a67355c726fecb5e9`. Retained qualified bytes:
`/tmp/rea-full-round-qualified/da45c8d325c8295264649e09f075920ba5f4950cc1fd043a67355c726fecb5e9.wasm`. Source program, original native artifact, WASM-generated artifact,
display data, build/runtime logs and selected source snapshots are saved in
`../qualification/full-round-display-wasm-2026-11-02/`. Receipt:
`../qualification/full-round-display-wasm-2026-11-02/results.json`.
The sampled rounded-box volume errors at 2/8/16 are
`[3.7579863461232037, 0.24324684347024572, 0.060921291870073446]` mm³. These are display convergence observations,
not exact mass or certified mesh-distance bounds.

The application package/kernel and scene builder were not changed. Exact
snapshot/edit ownership, actual packaged worker delivery, general rational
consumers, downstream operations/interchange and the full original parity
scope remain open. Native through-hole display coverage is in the previous
receipt; this WASM slice does not separately qualify through-hole rendering.


### Exact Full Round snapshot ownership — 2026-11-02

`NativeGeometryArtifact` now has a separate `rational-brep` kind.
`createExactFullRoundSnapshot` saves the entire native-verified frozen artifact;
`restoreExactFullRoundSnapshot` verifies storage metadata and then reconstructs
fresh native geometry/material/G1/radius proof, returning deeply frozen exact
geometry. No f64 BRep conversion occurs. Valid hashes do not authorize forged
geometry; wrong carriers/unknown payload fields/accessor metadata refuse.

Five opt-in snapshot tests passed with the SHA-pinned qualified real WASM
(only test module delivery is substituted; geometry uses the production host
kernel and Rust ABI). Thirty existing scene/contract regressions and
`vue-tsc --noEmit` pass. Tests verify lossless rational strings/IDs/recipe,
sampling-independent geometry revision and stale face selection after an actual
height edit. Receipt:
`../qualification/full-round-exact-snapshot-2026-11-02/results.json`.
11 clean files unchanged. Global architecture currently reports six other
worktree files; this change raises no pins and all changed files fit limits.

The saved document is metadata, not proof of parametric-history regeneration.
Application scene publication, exact edit/lineage transfer and released
kernel/worker delivery remain open, followed by general consumers/exchange and
the complete initial parity scope. The opt-in test does not qualify released
module delivery or worker bootstrap; without its external qualified WASM it is
explicitly skipped.


### Internal exact scene publication and root-map identity repair — 2026-11-02

The internal BRep scene adapter now publishes rational Full Round display with
its complete verified exact snapshot and actual topology face IDs. Sampling
changes retain source geometry, semantic entity and snapshot revision; native
face selection re-anchors onto the new mesh. Source geometry edits make prior
selection stale. Metrics remain display estimates, not certified exact mass.

The two-output test found a real bug: duplicated output rows of one root map
invocation were incorrectly compared as separate branches. TS and native
result admission now compare canonical invocation roots while retaining exact
node/owner/branch validation. Host/native owner-forgery regressions remain green.
Two Full Round outputs publish distinct semantic entities and snapshots.

Evidence: 7 actual-WASM scene tests, 199 host regressions, 3 native result tests,
12 fresh ABI cases and both TS typechecks passed. Cancellation during committed
snapshot disposal prevents publication; evaluation recovers after aborting a
computed exact node. WASM SHA: `a0d2c6d24432429cd88e14683f5f4256c6e0d914bf84d41752fb1951327f441c`.
Receipt and complete logs/source snapshots/ABI artifacts:
`../qualification/full-round-exact-scene-2026-11-02/results.json`.
11 clean files unchanged. No architecture pins changed; the same six other
recorded worktree files keep the global architecture gate red.

This qualifies the internal executable adapter with test-only SHA-pinned module
delivery, not production engine registration, the released package or actual
application Worker/bootstrap. Released source-bound delivery and full app
qualification remain open, as do general edit/history/lineage transfer,
downstream rational operations/exchange and the complete initial parity scope.


### Installed bounded release kernel and real browser worker — 2026-11-02

Built and installed the release package through the standard bounded packing
format. Kernel SHA: `c0701cb91cf7b719fad299c8ec1e713a820fea0521dd3150c8f778328c4f49c4`, 16225953 bytes; the 16 MiB decompression
limit and all other transport/deadline bounds remain unchanged. Previous local
package is preserved in `/tmp/rea-full-round-delivery-before`. Current qualified
bytes are retained at `/tmp/rea-full-round-qualified/c0701cb91cf7b719fad299c8ec1e713a820fea0521dd3150c8f778328c4f49c4.wasm`.

The initial optimized candidate exceeded the bound (17,130,275 bytes).
Additional pass/profile experiments were insufficient and were not adopted.
Final delivery stores the same default font losslessly compressed in WASM
(native font path unchanged) and omits implementation panic locations only in
release WASM. Typed public errors and source-language spans remain. All 75 SVG/
PDF tests pass, including every-font-byte equality; actual WASM Unicode PDF
output is byte-identical to the original storage path.

Added the browser diagnostic executor using the actual owned Worker. Real
browser testing exposed a worker receipt validator that admitted only f64 BRep
snapshots; it now correlates exact snapshot encoding/carrier/topology with the
admitted program without promoting metadata into a geometry certificate.
Five actual disposable workers passed on the installed package with no artifact
alias/compiler substitution: single/multiple Full Round output, invalid native
selection, cancellation/recovery and main-loop responsiveness. Wrong carrier,
encoding and face-list binding are explicitly rejected. All five workers were
terminated and the lane returned idle.

12 release ABI cases, 59 ordinary installed-kernel tests (exact snapshot/scene
cases now run without opt-in), 17 Node packaging tests (one optional native
optimizer case skipped), both TS checks and the production Vite build pass.
Package audit has exact roundtrip, no imports and unchanged export names/kinds;
this is not by itself whole-engine semantic equivalence. Vite reports large
chunk warnings. 11 clean files unchanged; the six other recorded architecture
violations remain, with no pins raised. Receipt, browser/ABI/font evidence,
build logs and source snapshots:
`../qualification/full-round-worker-delivery-2026-11-02/results.json`.

The real worker is still the explicit diagnostic lane, not a newly registered
production BRep provider or automatic fallback. Full app/distribution/performance
qualification, general edit/history/lineage transfer, broader rational operations
and exchange, and the complete initial parity scope remain open.


### Native exact Full Round history/regeneration — 2026-11-02

Added immutable proof-owned native History: source rebasing resolves the same
three authored source face IDs (including a reordered native face array), then
actually reconstructs/admit-proves Full Round. Explicit selection edits rebuild
on the current original source; a tested opposite-end selection changes actual
geometry. Undo/redo retain every revision; edits after undo require an explicit
prefix branch and preserve the original redo. Failed edits are transactional.

`exact-full-round-history/1` stores frozen choices and full expected artifacts.
Import freshly proves every revision, including hidden redo, verifies source-ID
continuity/selection semantics and preserves exact coefficients/naming/cursor.
It refuses forged geometry, even an independently valid body with wrong source
ID ancestry, inconsistent operation tags and insufficient caller budgets.
Actual material pair/atom/ray work is counted natively and aggregated over all
retained revisions; saved allowances cannot expand the caller's policy.

8 history tests plus 15 material regressions passed (23 total). Receipt:
`../qualification/full-round-native-history-2026-11-02/results.json`.
11 clean files unchanged; no architecture pins raised, six other recorded
worktree violations remain. The byte limit is a serialized-document allowance
when codec is enabled; this is not a total accounting of every independent
geometry proof subsystem.

This is native Rust feature history on the admitted extrusion family. Bridge/
WASM/editor/scene history integration is not yet done and the installed kernel
is not claimed to expose it. General source-construction/dependency graphs,
lineage transfer for changed source IDs, branch merges, general exact downstream
operations and the complete initial parity scope remain open. No proprietary
history code was transferred and Plasticity runtime history equivalence has not
been established.


### Full Round history public bridge and actual WASM ABI — 2026-11-02

Added `cad_full_round_exact_history` and an immutable TypeScript transport helper.
The router uses native bounded replay before every action, including creation;
unknown credit fields refuse. Three public bridge tests and fourteen real
import-free release WASM ABI cases pass. Native history documents and actual
material work counters replay identically; live source edits, actual reselection,
undo/redo and prefix branching are exercised. Hidden redo forgery, wrong edit
semantics and insufficient caller budgets refuse.

Evidence: `docs/qualification/full-round-history-bridge-wasm-2026-11-02/`.
Raw tested WASM SHA: b657f30eb89365e8cc695f915549dc0bf77eaf1c4d4826244cd9132c5950408c.
Installed history endpoint delivery and editor/scene history integration remain
open, followed by general dependency/lineage history and the entire original
parity scope. No runtime equivalence with Plasticity is claimed.


### Installed exact history endpoint — 2026-11-02

Optimized and installed history-capable WASM SHA
`e552c5d978af4690563c20211a9e40e745b87a832fe265129d5ff72ee5523fac`
(16,243,008 bytes, unchanged 16 MiB bound). All 14 optimized ABI cases,
14 ordinary installed-package history/snapshot/scene tests, TypeScript and
five actual Chromium Workers pass. Browser tests cover existing scene worker
behavior, not editor history actions. Added direct persisted-document restore
transport with fresh native replay; responses remain immutable.
Receipt: `docs/qualification/full-round-history-bridge-wasm-2026-11-02/results.json`.
Prior package preserved in `/tmp/rea-full-round-history-delivery-before`.
Editor/scene history ownership, general dependency/changed-ID lineage/branch
merges and all original library parity requirements remain open. Clean baseline
11 files unchanged; six other recorded architecture violations remain.


### Exact Full Round scene mesh history edits — 2026-11-02

Added `editExactFullRoundSceneMesh`: native history restore/current-body binding,
actual native edits, replacement mesh/BVH/edges and exact snapshot with persisted
history. Undo/redo survive existing scene serialization; display detail does not
change exact revision. Original source program remains attribution only.
Two installed-WASM integration tests and TypeScript pass. Valid but mismatched
history and native failures refuse without changing the input mesh. Multiple
triangle-dependent provenance runs require future lineage mapping and refuse.
Receipt: `docs/qualification/full-round-history-scene-2026-11-02/results.json`.
This is the scene library API, not editor command/store or history Worker wiring.
Aggregate scene metrics, history Worker cancellation, general dependency/lineage
history and the entire original parity scope remain open.


### Exact Full Round history browser Worker — 2026-11-02

Added an explicitly invoked browser worker lane for the native history scene-edit
API. Each operation owns a disposable actual Worker; input buffers are cloned,
not transferred. Bounded transport, concurrent admission refusal, hard abort,
deadline, remote failure and subsequent recovery are exercised in Chromium on
the installed package with no candidate substitution. Real platform Worker API
instrumentation records six constructions and six terminations; very short
expiry can precede Chromium's debug Worker event. Undo/redo exact revisions,
source edits, display-detail independence and original input retention pass.
TypeScript passes; clean baseline 11 unchanged. Existing six architecture
violations remain and Vite reports large chunks; no bounds changed.
Receipt: `docs/qualification/full-round-history-worker-2026-11-02/results.json`.
Editor commands/stores, aggregate scene metrics, general dependency/lineage
history, external Plasticity runtime comparison and all original parity scope
remain open. This lane is not a registered production provider.


### Earlier native Full Round selection regeneration — 2026-11-02

`History::regenerate_selection` now edits root/earlier explicit selection
parameters and freshly rebuilds every descendant, including hidden redo, while
retaining the cursor. Source rebases inherit authored face IDs; later explicit
selections retain their choices. Saved source models remain exact inputs.
Actual work/storage budgets are enforced and failed later-ID resolution is
transactional. Three new native tests and eight history regressions pass.
Receipt: `docs/qualification/full-round-history-regenerate-2026-11-02/results.json`.
Clean baseline 11 unchanged; six other recorded architecture violations remain.
Bridge/WASM exposure of this new method, earlier source-parameter edits,
source-expression dependency replay, general lineage/branch merges and the full
original Plasticity library parity scope remain open. Installed kernel unchanged.


### Актуализация полного остатка — 2 ноября 2026

Консолидированный текущий план: [plasticity-remaining-2026-11-02.md](plasticity-remaining-2026-11-02.md).
Регрессия текущего endpoint `regenerate-selection`: 4 bridge-теста, 17 raw ABI,
17 actual installed ABI и 7 настоящих browser Workers прошли. Текущий пакет
`1a70b8d7c3c94767c70df5c99b4a5fcc136161151df4a12cbf8a67cbe40ce99c`
появился в общей рабочей папке и не заменялся нашим raw-кандидатом; его
происхождение не выводится из совпадения endpoint-проверок. 11 clean-файлов
неизменны; свежий architecture gate красный на 7 файлах.
Receipt: `docs/qualification/plasticity-status-refresh-2026-11-02/results.json`.
Полный scope пересечений/Boolean, offset/replace, blends, quad/G2, patterns/
history, exchange и внешней квалификации остаётся открытым.
