# План рефакторинга

Обоснование каждой задачи: [`ARCHITECTURE.md`](ARCHITECTURE.md). Числа: [`BASELINE.md`](BASELINE.md). Порядок ступеней обязателен: каждая следующая опирается на гарантии предыдущей.

Размер: S до часа, M до дня, L несколько дней, XL недели. Риск относится к изменению поведения. Задачи, помеченные **контракт**, меняют публичные пути Rust-крейтов, состав крейтов или процесс квалификации и требуют решения владельца до начала.

## Сделано в итерации 2026-11-10

| ID | Задача | Результат | Проверка |
|---|---|---|---|
| R0.1 | Инструмент измерений `tools/refactor-audit` | одна команда `tools/refactor-audit/run.sh` пересчитывает все метрики и таблицы инвентаризации | прогон целиком за 1 мин 50 с без линтеров |
| R0.2 | Удалить доказуемо устаревшие некомпилируемые копии | 72 файла, 17 945 строк; список в [`inventory/removed-2026-11-10.md`](inventory/removed-2026-11-10.md) | `cargo check --locked --workspace --all-targets` на чистой копии `0d752bebf` после удаления: 0,28 с, ничего не пересобрано |
| R0.3 | Убрать тесты, которые выполнялись дважды | `source_support_shell/tests/mod.rs` (9491 строка) и `source_shell_incidence/tests/mod.rs` (2555 строк); остались более полные копии в `projective_qualification.rs` | см. `BASELINE.md`, раздел «Дубли тестов» |

## Сделано в итерации 2026-11-10, вторая половина дня

См. статусы в таблицах ниже. Проверки: `cargo check --locked --workspace --all-targets`, WASM-сборка `geometry-wasm`, `languages-wasm`, `photogrammetry-wasm`, тесты затронутых модулей, `tsc` (оба проекта), 229 тестов Vitest вокруг удалённых модулей, `node scripts/audit-architecture.mjs` и 13 тестов к нему.

### Находки: тесты из потерянных копий, которые не проходят на текущем коде

Код не менялся. Тесты лежат в некомпилируемых файлах, которые оставлены до решения владельца.

| Тест | Файл-источник | Где падает |
|---|---|---|
| `three_exact_crossings_share_one_vertex_without_collapsed_cuts` | `crates/nurbs-core/src/chord_arrangement.rs` | `offset/chord/arrangement.rs` |
| `nonbinary_concurrent_intersections_share_an_exact_recipe` | то же | то же |
| `concurrent_closed_profile_materializes_connected_filled_loops` | то же | то же |
| `adjacent_binary64_rays_have_exact_angular_order` | `crates/nurbs-core/src/chord_faces.rs` | `offset/chord/faces.rs` |
| `fixed_normal_arc_line_uses_exact_length_image_but_charges_retained_endpoints` | `crates/nurbs-core/src/sweeps/progressive_sweep/tests/fixed_normal_arc_line.rs` | `progressive_sweep::rmf_transport` |

Это либо потерянная функциональность (копию правили, а живой файл нет), либо устаревшие ожидания. Решение за владельцем алгоритма.

### Находка: падающие тесты на `main`

`brep-core`, `operations::tests::convex_profile_extrusion_and_connected_edge_chains_are_manifold` (`operations/tests.rs:692`) и `healing::tolerant_sew::tests`: `user_cap_skip_preserves_measured_gap_and_input_model`, `cumulative_cap_skip_is_typed_and_atomic`, `mixed_accepted_and_over_cap_gaps_roll_back_without_partial_markers` падают уже на коммите `0d752bebf` (проверено на чистой копии без правок этой итерации; `tolerant_sew.rs:363`, `:378`, `:404`). Файл последний раз менялся в `0d752bebf`. Чинить отдельным изменением.

`geometry-bridge` (юнит-тесты): `mesh_render::tests::matches_reference_for_primitives_and_dense_smooth_meshes`, `mesh_render::tests::preserves_repeated_incidence_degeneracy_and_unused_vertices`, `mesh_analysis::cooperative_tests::cancellation_at_every_cooperative_step_leaves_no_jobs_or_results`, `tests::layered_fillet_bridge_checks_chain_and_cavity` тоже падают на чистом `0d752bebf`.

### L5 выполнено

`createOpenScadMcpServer` (1305 строк одной функцией) разделена на `src/mcp/createServer/register/*`: контекст и девять модулей регистрации, каждый до 371 строки. Порядок всех 33 регистраций сохранён (сверено по именам), `tsc -p tsconfig.mcp.json` и 21 файл тестов MCP (157 тестов) проходят, файл ушёл из закреплённого списка аудита.

## Сделано в итерации 2026-11-10, вечер

### M1 применён к рабочему дереву

Код STEP и IGES из `brep-core` теперь живёт в `cad-step`, расходящаяся копия удалена (подробности и таблица перенесённых тестов в `m1-step-move.md`). Проверки на рабочем дереве: `cargo build --workspace --all-targets`; `cargo test -p cad-step` (160 юнит-тестов и все интеграционные); полный прогон юнит-тестов `brep-core` (2193 проходят, падают только 4 известных теста с `main`); сборка `geometry-wasm`; `audit-architecture` и `audit-clippy`. Файлы STEP для задания `Independent STEP OCCT` побайтно совпадают с теми, что давал прежний тест ядра.

Изменение поведения, которое стоит знать: экспорт STEP v6 теперь пишет авторскую карту граней без канонического зеркалирования (так было в `brep-core`; в старой копии `cad-step` нет). Тест `cad-step/tests/source_body_step.rs` сравнивает импорт с оригиналом напрямую, проверка стала строже.

Закрепления в `architecture-baseline.json` перенесены на новые пути: длинных функций 531 → 519, предупреждений clippy 2801 → 2790 (23 переехали из `brep-core` в `cad-step`).

Интеграционные тесты `geometry-bridge` `box_sphere_tessellation::box_sphere_results_tessellate_within_their_vertex_bounds`, `gear_tessellation::gears_take_part_in_booleans`, `tolerant_sweep_tessellation::tolerant_results_across_a_placement_sweep_tessellate_closed` падают и после M1, и на экспорте `HEAD` без правок рефакторинга. Это существующие ошибки, к M1 не относятся; раньше они не попадали в список, потому что падение юнит-тестов `geometry-bridge` останавливало прогон до интеграционных целей.

### Исправления в инструментах аудита

- `audit-architecture --update` терял ключ `clippy`, которым владеет `audit-clippy`. Теперь чужие ключи сохраняются, есть тест. Пины clippy восстановлены из истории.
- Правило слоёв не видело импорты `use crate::{a::X, b::Y}`. После исправления закреплённых восходящих рёбер 13, а не 8 (см. M7).

### O2: размер WASM

После M1 `geometry_wasm.wasm` весит 19 460 653 байта до `wasm-opt` и 17 268 337 после. Прежний замер 16,60 МБ сделан на `0d752bebf`, с тех пор в дерево попала и чужая работа, поэтому вклад M1 этим сравнением не выделить. Нужен замер «до и после» на одном дереве.

## Ступень 1. Фундамент

Без этих ворот всё, что ниже, деградирует обратно.

| ID | Задача | Размер | Риск | Эффект | Зависит от | Статус |
|---|---|:--:|:--:|---|---|---|
| F1 | Запретить коммиты, при которых `HEAD` не собирается сам. Локальный хук или первый шаг CI: `cargo check --locked --workspace --all-targets` (42 с с нуля) | S | низкий | 60 подряд красных прогонов CI на `main` были вызваны в том числе частичным коммитом (`gear_spec.rs` без `gear_with_report.rs`) | | todo |
| F2 | Ворота «каждый `.rs` под `crates/*/src` достижим из корня крейта», со списком исключений | S | низкий | не появляются мёртвые копии и потерянные тесты | R0.1 | сделано: `scripts/rustModuleTree.mjs`, правило в `audit-architecture.mjs`, 39 файлов закреплены |
| F3 | Счётчик предупреждений clippy на крейт в `architecture-baseline.json`: может только уменьшаться | M | низкий | 2906 предупреждений перестают расти | | сделано: `scripts/audit-clippy.mjs`, шаг в задании `rust` CI, 2801 предупреждение закреплено по крейтам |
| F4 | Снять `#![allow(dead_code, unused_imports)]` с корня `nurbs-core`, убрать 60 мёртвых элементов и 5 лишних импортов | M | низкий | компилятор снова видит мёртвый код в 260 тыс. строк | | сделано: `allow` снят, 23 мёртвые функции удалены, 26 помечены `cfg(test)` |
| F5 | Дефект: `abi_alloc` в `languages-bridge/src/abi.rs:13` делает `set_len` сразу после `with_capacity`; clippy считает это ошибкой (`uninit_vec`). Тот же код есть в `photogrammetry-ffi/src/lib.rs:117` под `allow` и в `geometry-bridge/src/abi.rs`. Свести три копии к одной реализации на `MaybeUninit` в общем крейте ABI | S | средний | убирает неинициализированный `Vec<u8>` и тройное дублирование ABI | тест на `abi` | сделано: `Box::new_uninit_slice` в `languages-bridge` и `photogrammetry-ffi`, тест `abi::tests` |
| F6 | Дефект: `npm run shaders:export` и `shaders:check` вызывают `-p raster-core`, которого нет в рабочем пространстве | S | низкий | рабочие или удалённые команды | решение: удалить или направить на `forma` | сделано: команды удалены, генератор живёт в `forma` |
| F7 | Лимит на функцию (длина, когнитивная сложность) в `audit-architecture.mjs`, текущие нарушения закреплены | M | низкий | останавливает рост 1425 функций длиннее 100 строк | R0.1 | сделано: `scripts/functionSizes.mjs`, лимит 150 строк, 531 функция закреплена |
| F8 | Запрет новых файлов `methods_*`, `*_impl_N`, `*_cases_N`, `*_part_N` | S | низкий | прекращает механическую нарезку | | сделано: правило в `audit-architecture.mjs`, 62 имени закреплены |
| F9 | Убрать неиспользуемые `#![feature(gen_blocks, yield_expr, super_let)]` из 20 крейтов и `#![allow(unused_features)]` | S | низкий | честный список зависимостей от ночной сборки | | сделано: в 20 крейтах оставлены только используемые возможности (5 крейтов) |

## Ступень 2. Страховка

| ID | Задача | Размер | Риск | Эффект | Статус |
|---|---|:--:|:--:|---|---|
| S1 | 57 тестов в некомпилируемых файлах: вернуть в сборку или удалить. Крупнейшие: `geometry-bridge/src/svg/tests/tests_fixtures.rs` (28), `nurbs-core/src/surface_offset_transport_tests.rs` (10), `geometry-bridge/src/progressive_annular_tests.rs` (6) | M | низкий | либо возвращается покрытие, либо уходит 9 тыс. строк | частично: 23 из 29 потерянных тестов возвращены и проходят; 5 падают на текущем коде (находки ниже), 1 не перенесён |
| S2 | Остальные дубли тестов: `nurbs_ss_general/tests.rs` и `tests/ss_report_admission.rs` (9), `surface_quotient_injectivity` (9), `sweep_miter_owned` (8), `trim_simplicity` (5), `gear` (4) | S | низкий | ещё около 1 тыс. строк и время прогона | сделано: 2 модуля-подмножества удалены целиком, 8 совпадающих тестов в 3 парах |
| S3 | Зафиксировать тестами формат сообщений ядра перед L4 | M | низкий | страховка для генерации валидаторов | todo |
| S4 | Разметить медленные доказательные тесты. Пять тестов `source_support_shell` и `source_shell_incidence` занимают от 226 до 548 с каждый в отладочной сборке | S | низкий | управляемое время CI | todo |
| S5 | Оставшиеся 43 некомпилируемых файла с уникальным содержимым (список в `inventory/duplicates.md`) | M | низкий | решение по каждому: подключить или удалить | todo, нужен владелец |

## Ступень 3. Структурные перемещения без правки логики

Каждая задача это отдельная серия изменений; перемещение и правка логики не смешиваются.

| ID | Задача | Размер | Риск | Эффект | Зависит от | Статус |
|---|---|:--:|:--:|---|---|---|
| M1 | **контракт** STEP и IGES: единственный владелец `cad-step`. Перенести оставшиеся модули, переключить 16 ссылок в `geometry-bridge`, удалить копии из `brep-core` | L | средний | минус около 19 тыс. строк в `brep-core`, минус 9,7 тыс. строк дублей | ADR-0011 | сделано в рабочем дереве (не закоммичено): код `brep-core` живёт в `cad-step`, тесты ядра со STEP разделены на проверку модели с фикстурой в ядре и круговой обмен в `cad-step`; STEP для OCCT побайтно совпадает с прежним — см. `m1-step-move.md` |
| M2 | **контракт** Запросы пересечения: владелец `nurbs-intersect`; в `brep-core` остаются `pub use` | L | средний | минус 8 тыс. строк дублей | ADR-0011 | todo |
| M3 | **контракт** Решатели пар аналитических поверхностей уходят под `brep-core`; `brep-intersect` остаётся адаптером к `Model` | L | средний | минус 5,7 тыс. строк дублей | ADR-0011, M2 | todo |
| M4 | Удалить каталоги-остатки `crates/{modelgraph-runtime, math-core, tensor-core, raster-core, modelgraph-text}` (43 файла, 8,5 тыс. строк). На них ссылаются `scripts/audit-gcode-branch.mjs`, `scripts/bench-profile-triangulation.mjs`, `integrations/curvex/qualify.py`, комментарии генератора шейдеров | S | низкий | убирает исходники без `Cargo.toml` | F6 | сделано: 43 файла удалены |
| M5 | Удалить остатки `geometry-bridge/src/svg*` | S | низкий | минус 2,6 тыс. строк | S1 | сделано: 5 файлов; все 28 тестов и все функции уже есть в `bridge-svg` |
| M6 | `nurbs-core`: разложить 60 плоских модулей (51 тыс. строк) по доменным каталогам, старые пути через `pub use` | L | низкий | крейт получает дерево вместо списка | | частично: 16 модулей перенесены (12 `offset_*` и `trimmed_offset_contact` в `offset/`, 2 `sweep_*` в `sweeps/`, `bernstein_interpolation` в `numerics/`), старые пути через `pub use`. Остальные 44 ждут порядка слоёв из ADR-0012 |
| M7 | `nurbs-core`: разделить `numerics`, убрать 17 обратных рёбер (109 ссылок), включить ratchet слоёв | L | средний | цикл на 90% кода исчезает | ADR-0012, M6 | почти сделано: правило слоёв в аудите видит и импорты в фигурных скобках; восходящих рёбер 13 → 2. Перенесены целиком `conditioning` → `core`, `bezier_extraction`, `canonical`, `sections::sections` → `foundation`, `curve_jets`, `curve_deviation` → `distance`, `trimmed_surface_distance` → `trim`, `section_phase` → `join`, `curve_extension` → `editing`, `continuity::deviation` → `foundation::surface_deviation`. Выделены общие части: интервалы и вставка узлов → `numerics::outward_interval`, произведение Бернштейна → `numerics::bernstein_product`, типы и струи границ шва → `analysis::surface_boundary`. Тесты `fixed_normal_path` собраны в `mod tests` (вызов `transport` был тестовым). Старые пути работают через `pub use`. Остались `analysis -> sweeps` (`affine_volume` использует внутренний `ideal_volume::cap_difference`) и `offset -> construct` (`natural_loft`, `grid_spline`, `primitives::circle_arc`; обратная ссылка `construct -> offset` образует цикл). Нужна переработка, перенос файлов не поможет |
| M8 | `brep-core`: сгруппировать 244 модуля в каталоги семейств, включить ratchet слоёв | XL | низкий | навигация и основа для деления на крейты | M1..M3, подтверждение семейств | todo |
| M9 | `src/`: закончить выделение `semantic`, `importers`, `mcp/server`; 43 недостижимых файла заменить актуальным разбиением или удалить | L | средний | минус 9 тыс. строк мёртвого кода, три файла уходят из исключений аудита | тесты MCP и импортёров | частично: 36 недостижимых копий в `src/semantic`, `src/importers`, `src/mcp/server` удалены (≈8 тыс. строк); новое разбиение не начато |
| M10 | `src/services`: перенос по доменам с проверкой границ импортов | XL | низкий | 562 файла получают структуру | M9 | todo |
| M11 | Пары `*_legacy` в `brep-core`: `atlas_legacy`, `rounding_legacy`, `rational_loft_legacy`. В `atlas_legacy*` 51 мёртвый элемент | M | средний | удалить или обосновать вторую реализацию | | todo |
| M12 | 8 мест `clippy::duplicate_mod`: один файл компилируется как два модуля (`error_upper`, `interval_vec3`, `exact_curve_segments`) | S | низкий | меньше кода в сборке | | сделано по сути: `sweep_support` дублировал `numerics` (3 модуля); копии заменены реэкспортами. Оставшиеся `duplicate_mod` это общие тестовые фикстуры |

## Ступень 4. Логические рефакторинги

Порядок внутри ступени: модули `P1` из `INVENTORY.md` (высокая сложность и высокий churn), затем `P2`.

| ID | Задача | Размер | Риск | Эффект | Статус |
|---|---|:--:|:--:|---|---|
| L1 | Диспетчеры-гиганты заменить таблицей обработчиков: `Compiler::operation` (cog 451), `Evaluator::eval_node_cases_1..4`, `nurbs-core::transport::dispatch_other` (613 строк), `geometry-bridge::dispatch::nurbs::handle` (634 строки), `Emitter::emit_cases_1` | L | средний | по одному обработчику на операцию, тестируемость | todo |
| L2 | Объекты параметров для функций с 8 и более параметрами (488 функций), начиная с `qualify_linear_region` (17), `refine_pixel` (19), `qualify_interior_contact` (16) | M на группу | низкий | снимает 83 `allow(clippy::too_many_arguments)` | todo |
| L3 | 82 механически нарезанных файла собрать в осмысленные модули | M | низкий | имена отражают ответственность | сделано: все 62 механических имени заменены именами по ответственности (`node_dispatch`, `primitive_nodes`, `geometry_nodes`, `checks`, `tokens`, `grammar`, имена методов вместо `methods_*`); список в аудите пуст |
| L4 | **контракт** Валидаторы сообщений ядра в TypeScript генерировать из схемы | XL | средний | убирает функции со сложностью до 937, один источник правды | отложено решением владельца 2026-11-10 |
| L5 | `createOpenScadMcpServer` (1305 строк одной функцией): регистрация по одному инструменту на модуль | L | средний | входит в M9 | сделано, см. выше |
| L6 | Политика `unwrap`: `clippy::unwrap_used` сначала в адаптерах (`geometry-bridge`, `bridge-*`, `languages-bridge`), затем в ядре | M на крейт | средний | при `panic = "abort"` каждый `unwrap` это потеря воркера | todo |
| L7 | Отчёты с 20..30 публичными полями (`ExpressionWallReport`, `RationalClosedReport`) разбить на вложенные типы с инвариантами | M | низкий | 2813 структур с публичными полями это отсутствие инкапсуляции | todo |
| L8 | Разбор по чеклисту «метод» и «класс» модулей `P1`: 80 модулей Rust, 5 каталогов TypeScript | XL | по задаче | основной объём работы, идёт параллельно по модулям | todo |

## Ступень 5. Оптимизация по замерам

| ID | Задача | Замер «до» | Статус |
|---|---|---|---|
| O1 | Время тестов: убрать повторное выполнение | 48 тестов, см. `BASELINE.md` | сделано (R0.3) |
| O2 | Размер WASM после M1..M3: дубли кода линкуются дважды | `geometry-kernel.wasm` 16,60 МБ, упакованный чанк 5,10 МБ | после M1: 17,27 МБ на текущем дереве; вклад M1 не выделен, нужен замер на одном дереве |
| O3 | Время сборки после M7: деление `nurbs-core` на крейты | тестовая сборка `brep-core --lib` с нуля 2 мин 38 с; `cargo check --workspace --all-targets` 42 с | todo |
| O4 | Горячие пути ядра: профиль на существующих бенчмарках (`bench_manifold`, `bench_validation`, `bench_decode`, `bench_brep_dispatch`, `bench:step-lod0`) | не снят | базовые замеры сняты (`BASELINE.md`, «Горячие пути ядра»); профиль и правки не начаты |
| O5 | Профиль тестовой сборки: в CI уже `CARGO_PROFILE_TEST_OPT_LEVEL=1`; проверить, окупается ли это локально на самых долгих тестах | замерено на 63 тестах: 2 182 с против 702 с, но сборка с нуля 2 мин 38 с против 16 мин 56 с | замерено; локально не окупается |
| O6 | Скрипт `verify-native-test-partition.mjs` собирает и перечисляет все тесты трижды | около 15 минут | todo |

Кандидаты для O4 без профиля не правятся. Статический список мест с аллокациями в циклах лежит в `inventory/worst-functions-rust.md`.

## Ступень 6. Чистка

| ID | Задача | Размер | Статус |
|---|---|:--:|---|
| C1 | **контракт** `docs/qualification`: 3,29 ГБ в git. Вынести в LFS или артефакты релизов | L | todo, решение владельца |
| C2 | Лишние зависимости: `geometry-bridge` объявляет `gcode-core`, `laser-core`, `slicer-core`, которые использует только через `bridge-cam`; `resvg`, `usvg`, `svgtypes` нужны только тестам; `bridge-analysis` не использует `value-codec` | S | сделано: убраны 4 зависимости, 2 переведены в dev |
| C3 | **контракт** Переход на стабильный Rust: заменить 43 `do yeet`, 9 блоков `try`, 4 `deref!` | M | todo, решение владельца |
| C4 | `architecture.md` описывает приложение на Vue 3 и ссылается на `src/App.vue`; после перехода на Svet файл называется `src/App.svt` | S | сделано |
| C5 | Четыре версии `glam` и по две версии `syn`, `miniz_oxide`, `rustls-webpki`, `webpki-roots` в `Cargo.lock` | S | разобрано: `glam` 0.30..0.33 тянет `nalgebra` из зависимостей `forma`, `syn` 2/3, `miniz_oxide`, `rustls-webpki`, `webpki-roots` приходят транзитивно (`serde`/`clap`, `png`/`flate2`, `rumqttc`/`ureq`); исправляется только в `forma` и при обновлении зависимостей |

## Как вести работу

- Одна задача, одна серия изменений, одна цель. Перемещения отдельно от правок логики.
- После каждого шага: `cargo check --locked --workspace --all-targets`, тесты затронутого крейта, `node scripts/audit-architecture.mjs`, `tools/refactor-audit/run.sh` для пересчёта таблиц.
- Правки исходников ядра меняют привязки квалификации. Обновление привязок (`refresh-qualification-fingerprints.mjs`) делается отдельным шагом после серии изменений, как это уже принято в репозитории.
- В рабочем дереве одновременно работают несколько агентов. Структурные задачи ступени 3 выполнять в отдельной ветке и сливать, когда в затронутых каталогах нет незакоммиченной работы.
