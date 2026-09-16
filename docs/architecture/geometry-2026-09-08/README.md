# Пакет знаний: собственная геометрия и нарезка

Дата проверки: 2026-09-08 (пакет). Перепроверка MCP + re-ingest: 2026-09-10. Статус: **пять документов манифеста заингестены** (upsert, revision 2); receipt — [ingest-receipt.json](ingest-receipt.json). Hotpath-заметка вне манифеста в RAG как raw/`proposed`.

Главный документ — [архитектура](../../design/native-geometry-architecture.md). Это синтез текущего кода, требований пользователя, прежних исследований и новых первичных источников. Новые структуры/crates/этапы помечены как предложения, не как выполненная реализация.

## Что прочитано и сверено

1. Rust workspace из восьми crates; публичные APIs, native/WASM dispatch и ModelGraph → scene publication. [Аудит](../../design/geometry-architecture-audit-2026-09-08.md) указывает конкретные исходники.
2. Старые `architecture.md`, `rust-brep-nurbs-kernel.md`, `brep-nurbs-14-stage-master-plan.md`, текущая матрица native-modeling и реконструкции. Старые «два постоянных движка» согласованы с позднейшим требованием собственного polygon-ядра; frozen artifacts не изменены.
3. Предыдущие обзоры geometry processing, CAD/HCI, Plasticity и второй исследовательский проход. Они использованы как историческая подборка. Две прежние сотни репозиториев **не перепроверялись здесь по одному**.
4. RAG: catalog → wiki search → raw search → выбранные документы. Общий поиск `SDF subdivision` дал омонимы из других проектов; эти результаты исключены. Семантический wiki-поиск дважды завершился timeout; точечный текстовый поиск отработал.
5. Curvex: текущие `path_offset.rs` и `geometry_predicates.rs` проверены после нахождения через RAG. Переносимый опыт: atomic offset, explicit fill rule, split numeric/UX tolerances. Там используются сторонние Rust-алгоритмы; это не доказательство существования собственной реализации здесь и не предложение автоматически подключить зависимости.
6. [14 первичных источников по нарезке](../../design/slicing-evidence-2026-09-08.md); дополнительно CGAL robustness и Lévy mesh CSG в основном документе. [Независимая проверка архитектуры](../../design/geometry-architecture-review-2026-09-08.md). Это внутренние независимые контексты проверки, не внешняя сертификация.
7. Hotpath `#[inline(always)]` в `math-core`: **observed** 2026-09-10 (microbench ~190.7 ms → ~1.2 ms / 2e6 iters); раунд 2 (inline + `unsafe`): `sdf-core` mesh distance 334.6 → 167.4 ms, `polygon-core` edges ~5–8%, bench-примеры `bench_sdf_cpu` / `bench_mesh_kernels`; см. `docs/rag-inline-hotpath-optimization.md` / URI `docs://rag-inline-hotpath-optimization`.

Охват — релевантные знания о текущем проекте, геометрии, топологии, нарезке, исполнении и печати. Это не заявление о прочтении всей базы из 82 844 документов. Идентификаторы запросов/источников, состояние подключения и ограничения поиска: [retrieval.json](retrieval.json).

## Соединение mcp-rag

Проверка 2026-09-10: Cursor MCP namespace `user-rag-mcp` (`ready`), конфиг `~/.cursor/mcp.json` → `http://127.0.0.1:7432/mcp` (gateway `local.rag-mcp`). Вызваны `status` и `list_documents`. DuckDB: `ready_for_search=true`, `fts_ready=true`, `embedding_manifest_match=true`, ollama/`nomic-embed-text`/768, **82 844** документов / **643 994** chunks, **642** документов в wing `open-scad-viewer`.

Подключение к работающему HTTP gateway уже есть; второй локальный stdio-writer на ту же DuckDB не нужен. `ingest_roots_configured=true`; запись пакета через `ingest_file` на 2026-09-10 подтверждена (см. receipt). Не запускать второй writer или широкую синхронизацию workspace с generated/output/node_modules.

## Как заингестить / обновить

1. Проверить hashes по `ingest-manifest.json` относительно файлов. Если документ или source evidence изменились, пересобрать пакет и указать новую дату проверки.
2. Вызвать `ingest_file` для **пяти Markdown-файлов** из `ingest-manifest.json` → `documents[]`, последовательно (upsert by URI):
   - `docs/design/native-geometry-architecture.md`
   - `docs/design/geometry-architecture-audit-2026-09-08.md`
   - `docs/design/slicing-evidence-2026-09-08.md`
   - `docs/design/geometry-architecture-review-2026-09-08.md`
   - `docs/architecture/geometry-2026-09-08/README.md`  
   Каждый entry содержит path, uri, title, wing, room, metadata_json.
3. Проверить возвращённые document IDs через `get_document`; сравнить content/hash, revision и metadata. Не удалять старые unrelated источники.
4. Если нужны compiled wiki pages: `get_schema`, затем write/read-CAS с raw IDs. Разделить `observed`, `proposed`, `historical`, `not_implemented`. После записей — rebuild_index и контрольные запросы.
5. Обновить [ingest-receipt.json](ingest-receipt.json) с document IDs, hashes, временем. Только актуальный receipt разрешает утверждать «заингестил».
6. Hotpath `#[inline(always)]` (`docs://rag-inline-hotpath-optimization`) — **observed** 2026-09-10; при новых leaf-оптимизациях обновлять тот же URI + receipt с цифрами.

Нельзя обновлять frozen планы/manifest, чтобы убрать конфликт старого текста. Новая wiki должна объяснять изменение scope и ссылаться на старые документы как на исторические. RAG не участвует в вычислении геометрии и не получает доступ к принтеру.

## Контрольные запросы после ingest

- «Почему NURBS для нарезки не нужно целиком превращать в mesh?» — раздел 9 архитектуры, источники S01/S02/S05/S07.
- «Готовы ли general NURBS Boolean?» — аудит: нет; архитектура: intersection/trim/classify/sew pipeline и ограниченные capabilities.
- «Что общего у четырёх библиотек?» — ADR A03/A06 и таблица зависимостей; не универсальная mesh conversion.
- «Почему выбирается треугольник, а не поверхность?» — аудит source mapping и ревизий; native artifact должен доходить до scene.
- «Есть ли у нас слайсер и G-code?» — аудит: реализации в crates нет; печатный pipeline предложен.
- «Можно ли доверять N уровням SubD как заданной точности?» — нет, refinement и limit evaluation различаются.

В ответах нельзя превращать эти проверки извлекаемости знаний в доказательство геометрической корректности реализации.
