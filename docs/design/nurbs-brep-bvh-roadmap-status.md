# NURBS, B-rep и BVH: что сделано и что осталось

Состояние ветки `nurbs-brep-bvh-roadmap` относительно `577e15542`.
Источник списка: аудит 2026-11-10 и `nurbs-priorities.md`.

## Сделано в этой ветке

| Область | Изменение | Где |
|---|---|---|
| BVH | Запросы по боксу, радиусу, k ближайших; `refit` без пересборки | `crates/mesh-query/src/bvh_range.rs` |
| BVH | Версионированная сериализация дерева с проверкой структуры | `crates/mesh-query/src/bvh_codec.rs` |
| NURBS | Аналитические производные порядка до 16 для кривых и поверхностей | `crates/nurbs-core/src/core/derivatives.rs` |
| NURBS | Инверсия точки сходится для точек на кривой (раньше каждая пятая упиралась в лимит итераций) | `crates/nurbs-core/src/analysis/point_inversion.rs` |
| NURBS | Тесты инвариантов ядра на сгенерированных кривых | `crates/nurbs-core/tests/kernel_invariants.rs` |
| NURBS | Удалено 70 файлов (22k строк), которые не компилировались ни одной целью | `crates/nurbs-core/src` |
| NURBS | Снято `allow(dead_code, unused_imports)` на весь крейт; 55 известных мест помечены точечно с причиной | `crates/nurbs-core/src/lib.rs` |

Проверка: `nurbs-core`, `nurbs-intersect`, `mesh-query` на базовом коммите
4481 тест прошёл, 2 упали; на ветке 4501 прошёл, упали те же 2
(`tessellation_shading::collapsed_cone_row_uses_analytic_limit_without_winding_flip`,
`ideal_volume_atlas::open_spatial_atlas_covers_original_arc_bishop_jets_and_distinct_moving_carriers`).
Зависимые крейты (`brep-core`, `cad-step`, `geometry-bridge`) проходят
`cargo check --all-targets`; их тесты на ветке не запускались, так как
B-rep код не менялся.

Вторая волна:

| Область | Изменение | Где |
|---|---|---|
| NURBS | Локальная C1 кубическая интерполяция (9.3.4) | `foundation/local_interpolation.rs` |
| NURBS | `third_jet` на точных производных вместо конечных разностей | `analysis/higher_derivative_map.rs` |
| NURBS, исправление | Сертификат отклонения теперь доходит до уровня округления: удаление только что вставленного узла больше не отклоняется с оценкой 7e-3 | `analysis/curve_deviation.rs` |
| NURBS, исправление | `refit_curve` больше не падает, когда абсцисса Гревилля выходит за домен на 1 ulp | `foundation/common.rs` |
| NURBS | Инварианты поверхностей (`edit_axis`, `trim`, `iso`), удаление узла и понижение степени | `tests/kernel_invariants.rs` |
| BVH | Точный тест треугольник-бокс (`intersect_aabb`) | `mesh-query/src/bvh_range.rs` |
| STEP/IGES | Тест устойчивости 16 импортёров к повреждённым файлам: 25 920 запусков без паник | `cad-step/tests/malformed_input_robustness.rs` |
| B-rep | Внеосевые пары сфера-тор с доказуемым промахом дают пустой результат вместо `UnsupportedSurface` | `brep-intersect/src/sphere_torus.rs` |

OBB-дерево проверено: подключить его как broadphase к сертифицированным
пересечениям нельзя, листья возвращают копии контрольных точек без
диапазонов параметров. Нужно либо добавить домены в листья, либо удалить
публичный API; это решение владельца.

Отступление от плана: вместо `proptest` используется генератор с
фиксированным зерном. `proptest` нет в `Cargo.lock`, а добавление
зависимости меняет общий lock-файл, который правят другие агенты.

## Что осталось

### Уборка и верификация

- **Мёртвый код: решение по 25 элементам.** Помечены
  `no caller in this crate; remove or wire up`. Крупнейшая группа: конвейер
  маршировки и сечений филлета в `offset/fillet_surfaces/{marching,sections}.rs`
  (13 функций). Нужно решить: подключать или удалять.
- **Две потерянные группы тестов.** У удалённых `surface_offset_transport_tests.rs`
  (10 тестов) и `sweeps/progressive_sweep/tests/fixed_normal_arc_line.rs`
  (1 тест) нет живой копии. Они не компилировались и до удаления; если
  покрытие нужно, восстановить из истории и подключить.
- **Исторические релиз-скрипты** `scripts/release-own-rust-v28..v32.mjs`
  перечисляют удалённые пути и при повторном запуске упадут.
- **Fuzzing с покрытием** (`cargo-fuzz`): мутационный тест импортёров есть,
  но для `import_nurbs_step_trimmed` нет валидной затравки, а JSON-диспетчер
  `nurbs-core/transport` не покрыт.
- **Классификация `.unwrap()` вне тестов** (в `src` их около 12 тысяч вместе
  с тестовыми). Не начато.
- **Бенчи на `criterion`**, бенчи пересечений и тесселяции. Не начато.
- **8 модулей с именами не в snake_case** (сгенерированные
  `methods_*__*`), предупреждения были и до этой ветки.
- Обновить `docs/nurbs-core-reorg-plan.md`: цифры устарели, реорганизация
  фактически завершена.

### NURBS-ядро

- **OBB-дерево без потребителей.** `numerics/obb_tree.rs` никто не вызывает.
  Подключить как broadphase в `intersection/ss_intersection`, `self_surface`,
  `self_curve`, замерить; без выигрыша удалить.
- **Водонепроницаемая тесселяция с LOD** (пункт 144 `nurbs-priorities.md`):
  согласованные рёбра между патчами, аналитические нормали, единый winding.
- **Локальная бикубическая интерполяция поверхности** (кривая сделана).
- **IGES 126/128**: полноценный импорт и экспорт рациональных B-сплайнов.
  Сначала выяснить статус `brep-core::iges_interchange_v2`, который сейчас
  перерабатывается.
- **T-сплайны**: отсутствуют, потребителя нет. Отдельное исследование.

### B-rep

Ничего из этого этапа не начато: каждый пункт требует сертификатов в стиле
ядра (точные представления, без численного отката) и соизмерим с отдельным
проектом.

- **Сфера×тор, общий случай с пересечением.** Промахи решены; настоящий
  внеосевой контакт (кривая 4-го порядка) по-прежнему `UnsupportedSurface`.
- **Булевы: расширение матрицы.** Конус, тор, непустые смешанные пары сейчас
  дают типизированные отказы.
- **Freeform×freeform булевы.** Зависят от расширения `nurbs-intersect/nurbs_ss`
  на касательные и почти вырожденные случаи.
- **Лечение положительных зазоров** в `brep-core/healing`.
- **Сборки и внешние ссылки STEP** в `cad-step`.
- **Пункты «Partial»** из `docs/design/brep-completion-status.md`: NURBS
  foundation, canonical primitives, extrusion/revolution, routing,
  SemanticProgram, legacy adapter, воспроизводимость workspace.
- **Падающие тесты на базовом коммите** (до этой ветки): в `brep-core`
  12 из 3842 падают в 6 целях: `--lib` (в том числе `healing::tolerant_sew`,
  3 теста, и `operations::tests::convex_profile_extrusion_and_connected_edge_chains_are_manifold`),
  `region_network`, `retained_uv_boolean`, `rooted_retention`,
  `stepped_prism`, `trihedral_graft`.
- **Тесты NURBS, падающие на базовом коммите:** `tessellation_shading`
  (вырожденный конус) и `ideal_volume_atlas`.

### BVH

- **SAH-сборка** (binned) как опция рядом с медианным разбиением.
- **Параллельная сборка** за feature-флагом.
- **Общее ядро узлов** для трёх BVH (`bvh`, `distance_bvh`, `scene_bvh`).
- **Обход через итератор** вместо `Vec` для запросов по объёму.
- **Проброс новых запросов** в `geometry-bridge` и WASM.
- `clippy -D warnings` в `mesh-query` падает на `semantic_pick.rs`
  (9 аргументов) и падал до этой ветки.
