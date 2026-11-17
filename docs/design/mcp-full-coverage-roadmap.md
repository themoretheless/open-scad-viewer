# Мастер-план полного покрытия MCP (`open-scad-viewer`)

Дата фиксации: 2026-11-17 (ветка `main`, базовый коммит `754ada0a1`, runtime v13 `61b587`).

## 1. Цель и архитектура покрытия

Полное покрытие MCP объединяет четыре взаимосвязанных уровня:

1. **Функциональное покрытие всех 8 доменов продукта**:
   - **OpenSCAD + каталог DuckDB (`openscad`)**: проверка, анализ, сравнение метрик, Customizer, перебор параметров, сохранение/чтение/списки/удаление моделей и ревизий, экспорт артефактов (`stl`, `obj`, `step`), независимый CLI/официальный OpenSCAD, handoff-бандлы (`openscad-viewer/handoff-bundle-v1`).
   - **Rush & NURBS & сборки (`rush`)**: компиляция и проверка `rush/ir-1` и `rush/nurbs-1`, инспекция интерференций и зазоров сборок, модификация графов, генераторы деталей, плоские SVG-контуры.
   - **Direct Modeling & B-rep диагностика (`direct`)**: ресурс схемы `direct://schema`, создание/инспекция `DirectDocument`, атомарные транзакции, автономный решатель 2D-эскизов (`sketch_solve`), проверка самопересечений и контактов граней (`brep_inspect_contacts`), измерение расстояния между B-rep телами (`brep_solid_distance`).
   - **Инженерный обмен и ремонт мешей (`exchange`)**: конвертация полигональных форматов (`mesh_convert`), диагностика дефектов и толщины/зазоров (`mesh_inspect`), консервативный и полный ремонт Manifold (`mesh_repair`), импорт/экспорт STEP и IGES v2 (`cad_import_step`, `cad_import_iges`, `cad_export_iges`).
   - **Конструкторские 2D-чертежи (`drawing`)**: ортогональные виды + изометрия + штрихованное сечение + таблица технологичности печати + векторный PDF (`cad_drawing_sheet`), допуски и посадки ISO 286 (`cad_iso286_fit`).
   - **CAM / 3D-печать / Лазер (`cam`)**: проверка готовности к печати (`cam_print_readiness`), нарезка в G-code (`cam_slice_gcode`), инспекция G-code (`cam_inspect_gcode`), подготовка контуров лазерной резки (`cam_laser_section_plan`).
   - **CAE / Механика / FEM (`cae`)**: линейный, buckling и модальный расчёт ферм (`cae_truss_analyze`), расчёт 3D-рам (`cae_frame_analyze`), МКЭ-анализ сцепленных Tet4-слоёв печатного тела (`cae_printed_solid_analyze`), руководство `cae://guide`.
   - **Мост с локальным редактором (`localEditor`)**: неблокирующая диагностика готовности вкладки (`openscad_editor_status`) и загрузка модели в активную вкладку Vite (`openscad_open_in_editor`).

2. **Профилируемая загрузка (`toolsetProfiles`)**:
   - Профили: `openscad`, `rush`, `direct`, `exchange`, `drawing`, `cam`, `cae` (по умолчанию активны все для обратной совместимости; в CLI доступны через `--profiles <csv|all>` и `OPENSCAD_VIEWER_MCP_PROFILES`).

3. **Единый wire-контракт и строгие схемы**:
   - Новые доменные инструменты публикуют `inputSchema`, `annotations` и строгую `toolOutputSchema(...)` (ветка успеха или типизированный `PublicToolError` с `code`, `retryable`, `next_action`).
   - Направленные подсказки агенту даются через 7 MCP-промптов.

4. **Общий runtime в HTTP-транспорте**:
   - `createRushGraphHttpServer` переиспользует общий `DirectGeometrySupervisor` и `HeadlessGeometryService` (включая `GeometryCompileCache`) на уровне сервера вместо пересоздания супервизора на каждый POST-запрос, корректно закрывая его при остановке HTTP-сервера.

5. **100% матричная верификация (`tests/mcpFullCoverageMatrix.test.ts` и `tests/mcpDomainExpansion.test.ts`)**:
   - Автоматический прогон `tools/list` без единого непокрытого инструмента (fail-closed): минимум 1 успешный и 1 ошибочный вызов для каждого инструмента с валидацией `structuredContent` по `outputSchema`, проверкой неизменности каталога DuckDB при `readOnlyHint: true`, чтением всех ресурсов/шаблонов и проверкой всех промптов.
