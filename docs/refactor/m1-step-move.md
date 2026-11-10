# M1: перенос STEP и IGES из `brep-core` в `cad-step`

Решение владельца (2026-11-10): единственным владельцем становится код `brep-core`, а живёт он в `cad-step`. Расходящаяся копия в `cad-step` удаляется.

## Что проверено на изолированной копии дерева

Сценарии лежат в `tools/refactor-audit/m1/` и применяются по порядку к каталогу `crates/`:

| Шаг | Сценарий | Что делает |
|---|---|---|
| 1 | `m1_move.py` | удаляет копии в `cad-step`; переносит из `brep-core` модули `step_interchange`, `step_interchange_v3`, `step_part21`, `step_pmi`, `step_v3_export`, `step_tessellated`, `step_presentation`, `step_schema`, `step_xde`, `nurbs_step_*`, `iges_interchange_v2`, `source_exchange_step` вместе с тестами; пути `crate::` к ядру заменяет на `brep_core::` |
| 2 | `m1_libs.py` | убирает объявления и реэкспорты из `brep-core/src/lib.rs`, объявляет модули в `cad-step` с прежней видимостью |
| 3 | `m1_policy.py` | переносит фасады `import_step_with_policy`, `import_iges_v2_with_policy` в `cad-step/src/import_policy.rs`; `apply_import_policy` в ядре становится `pub` |
| 4 | `m1_complex.py` | удаляет из ядра обёртки `export/import_complex_step/iges`, посимвольно совпадающие с уже существующими в `cad-step` |
| 5 | `m1_uselists.py` | разбирает импорты вида `use crate::{...}` на свои модули и `brep_core` |
| 6 | `m1_consumers.py` | переключает `geometry-bridge`, интеграционные тесты и примеры `brep-core`, тесты `cad-step` на `cad_step::` (80 имён); `brep-core` получает dev-зависимость на `cad-step` |
| 7 | `m1_cuboid.py` | конструкторы `freeform_cuboid_solid`, `freeform_cuboid_with_bump_face` и помощники `as_uniform_bicubic`, `is_uniform_bicubic_positive` к STEP не относятся и остаются в ядре (`brep_core::freeform_cuboid`); старые пути работают |

Ручные правки после сценариев: старый тестовый файл копии `cad-step` удалён (все 35 его тестов по именам есть среди 60 тестов `brep-core`); тест `prism` разделён на часть со STEP (в `cad-step`) и без него (в ядре); `material_wall_coverage::{plane, same_plane}` открыты как `#[doc(hidden)] pub` для теста STEP.

Результат: `cad-step` со всеми целями собирается с кодом из `brep-core`; `geometry-bridge`, интеграционные тесты и примеры `brep-core` собираются. Ядро теряет около 20 тыс. строк кода форматов обмена.

## Перенос тестов ядра со STEP

Юнит-тесты ядра не могут вызывать `cad-step`: тестовая сборка `brep-core` и библиотечная, от которой зависит `cad-step`, дают разные типы `Model`. Двенадцать тестов, проверявших модели круговым экспортом STEP, разделены на две части:

- в ядре тест строит модель, проверяет всё, что не относится к STEP, и сравнивает модель с фикстурой `brep-core/tests/fixtures/exchange/<имя>.json` (`crate::exchange_fixtures::assert_matches`, допуск 1e-9 относительный; `BREP_UPDATE_EXCHANGE_FIXTURES=1` перезаписывает фикстуры);
- в `cad-step` (`src/tests/kernel_fixtures.rs`) модель читается из той же фикстуры и проходит экспорт и импорт STEP с прежними проверками.

| Тест ядра | Фикстура | Тест `cad-step` |
|---|---|---|
| `step_roundtrip_not_faceted` | нет | `tests::kernel_exchange::step_roundtrip_not_faceted` (модель строится публичным API) |
| часть с обменом из `close_topology/tests/mod.rs` | нет | `kernel_exchange::mixed_complex_round_trips_through_step_and_iges`, `malformed_complex_step_is_refused` |
| `edited_rational_bridge_body_matches_the_exchange_fixture` | `face_cut_bridge` | `edited_rational_bridge_body_round_trips_step_with_both_original_clocks` |
| `compacted_graph_result_matches_the_exchange_fixture` | `face_cut_path_graph` | `step_round_trip_exports_the_actual_compacted_graph_result` |
| `edited_hole_cap_matches_the_exchange_fixture` | `face_cut_hole` | `step_round_trip_exports_the_edited_hole_cap_and_original_rational_clocks` |
| `mixed_ss_result_matches_the_exchange_fixture` | `ss_open_network_bridges` | `step_exports_the_actual_mixed_ss_result_with_its_new_curves_and_native_clocks` |
| `one_model_retains_both_general_bridge_graphs_and_matches_the_exchange_fixture` | `ss_open_network_graph` | `one_model_retains_both_general_bridge_graphs_and_edited_step_topology` |
| `projective_tetrahedron_has_closed_edges_and_manifold_vertex_links` | `projective_tetrahedron` | `projective_tetrahedron_round_trips_step_v6_with_identity` |
| `projective_generated_end_caps_keep_exact_poles_and_replay` | `projective_capped_end_caps` (модель и определение тела) | `projective_capped_end_caps_export_step_v6_and_source_step` |
| `assert_projective_root_exchange`, `root_partition_control` | нет | не нужен: подготовка обмена вынесена в `brep_core::source_exchange_plan`, проверки счётчиков и обрезок остались в ядре |

`source_exchange_plan::prepare` содержит всю проверяемую часть подготовки исходного обмена (точки, обрезки, счётчики, оценку ошибки концов); писатель `cad-step::source_exchange_step::prepare` только переводит план в текст. Помощник `assert_step_identity` переехал из ядра в `cad-step`. Функция `source_region_restore::test_limits` открыта как `#[doc(hidden)] pub`, чтобы `cad-step` мог восстановить тело.

Задание `Independent STEP OCCT` получает файлы для OCCT из теста `cad-step`. Перед переносом оба файла STEP (`projective-capped-v6.step`, `projective-capped-source-v6.step`) сверены побайтно с выводом исходного теста ядра: совпадают.

Две ручные отладочные переменные, которые не читает ни одно задание CI, удалены: `CAD_PROJECTIVE_ROOT_STEP_OUTPUT` и `CAD_SOURCE_CURVED_STEP_OUTPUT` (в ядре; в `cad-step/tests/source_exchange_step.rs` вторая осталась).

## Расхождение семантики STEP v6 между копиями

Старая копия в `cad-step` при любом экспорте, включая v6, приводила грани к каноническому направлению (`canonical_face_senses`). Код `brep-core` (коммит `fdfde96b5`) для v6 пишет авторскую карту и направления граней без изменений и помечает грани маркером `STEP_V6_AUTHORED_CHART_MARKER`; немаркированный v6 импортируется по-старому. После переноса тест `cad-step/tests/source_body_step.rs` упал: его помощник `tests/common/mod.rs::assert_step_identity` сравнивал импорт с канонизированным оригиналом, то есть закреплял поведение старой копии. Помощник теперь сравнивает импорт с оригиналом напрямую. Проверка стала строже: v6 обязан вернуть модель без зеркалирования. Импорт старых файлов не затронут: новый импорт файла, записанного старым экспортом, даёт прежний результат (оригинал с каноническими направлениями граней; проверено прежним помощником).

Для внешних потребителей это изменение формата: файлы v6, записанные после переноса, содержат грани в авторской карте, а не в канонической.

## Находка

`brep-core/tests/rational_section_loft.rs` и `cad-step/tests/rational_section_loft.rs`, а также пара `guided_loft_step.rs` — старые расходящиеся копии (19 и 20 тестов, разные проверки весов). Существовали до M1; объединение требует решения, какие проверки считать эталонными.
