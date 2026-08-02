# MCP: синтез десяти независимых обзоров

Дата среза: 2026-07-31.

Цель обзора — не просто добавить MCP endpoint, а определить минимальную
архитектуру, в которой локальный агент может воспроизводимо читать, проверять,
сравнивать, изменять и экспортировать OpenSCAD-модели, не превращая приложение
в небезопасный универсальный backend. Дополнение от 2026-08-01
фиксирует общий MCP contract для двух постоянных engine class:
Manifold и собственного Rust B-rep/NURBS.

Актуальная protocol-ветка сверена с официальной документацией TypeScript SDK:
[support for MCP 2026-07-28](https://ts.sdk.modelcontextprotocol.io/v2/migration/support-2026-07-28).

## Десять углов обзора

| № | Фокус | Главный вывод |
| --- | --- | --- |
| 1 | Protocol/API | Поддерживать современный discovery 2026-07-28 и legacy handshake; реальные success/error payload обязаны соответствовать опубликованным схемам. |
| 2 | DuckDB | Каталог должен быть typed repository, а не SQL console; revision CAS, retention и связанные записи должны меняться транзакционно. |
| 3 | Security | Ограничить вход, геометрическую очередь, исходящие данные и diagnostics; тяжёлый kernel в конечном счёте нужно вынести из stdio-процесса. |
| 4 | Agent UX | Агенту нужны инструкции, capabilities, prompts, стабильные коды ошибок и конкретный `next_action`, а не только список низкоуровневых tools. |
| 5 | OpenSCAD domain | Самые полезные операции — чистая проверка, сравнение измеримых свойств, Customizer и точный выбор исторической revision. |
| 6 | Tests/observability | Проверять wire protocol обоих поколений, публичные JSON Schema, subprocess stdio и redaction; неожиданные ошибки коррелировать, но не раскрывать internals. |
| 7 | Performance | Нужны admission/backpressure и затем content-addressed compile cache; очередь без hard-kill не останавливает один зависший kernel call. |
| 8 | Product workflows | Основные сценарии агента: review → diagnose → customize → compare → export, причём read-only шаги не должны засорять build history. |
| 9 | Modern MCP | Использовать instructions, prompts, cache hints и immutable resources; не строить новый дизайн вокруг устаревающей task-модели. |
| 10 | Red team | Запретить arbitrary SQL, implicit browser sync, extension autoload и безлимитные artifacts; не называть metric delta точным geometric diff. |

## Неподвижный контракт двух движков

`ManifoldPlanBackend` и Rust `BrepBackend` — не временная миграция, а два
постоянных peer-класса. Rollout B-rep не удаляет и не депрекирует
Manifold. Класс движка выводится из source language contract:

| Language contract | Authoritative engine class | Fallback |
| --- | --- | --- |
| `legacy/current` | `manifold` | `never` |
| `openscad-viewer/brep-1` | `brep` | `never` |

UI и MCP не могут переопределять этот маршрут. Если в request когда-либо
появится `engine`, он может быть лишь assertion уже вычисленного класса.
Несовпадение отвечает typed error; ошибка, timeout, missing capability или
unavailable provider никогда не запускают другой класс.

`shadow` — только diagnostic mode: он может посчитать оба кандидата,
но публикует только contract-authoritative result. Он не является
третьим language contract и не разрешает fallback.

Текущая матрица до deployment supervisor публикуется без прикрашивания:

| Engine class | Provider status | MCP isolation сейчас | Результат запроса |
| --- | --- | --- | --- |
| `manifold` | available, production legacy | синхронный in-process call под process-wide serialized queue | authoritative для `legacy/current`; cancel не hard-kill |
| `brep` | unavailable, contract/design | provider не развернут | `engine_unavailable`, без fake execution и Manifold fallback |

Присутствие `brep` в registry и discovery не означает регистрацию
фиктивного provider: manifest честно описывает недоступность и recovery.

## Что вошло в текущий пакет

### Воспроизводимые инструменты

- `openscad_check` проверяет inline source или сохранённую revision без записи
  build history.
- `openscad_compare` возвращает bounded delta размеров, метрик и topology; это
  явно не exact boolean difference. Он возвращает execution provenance обеих
  сторон: `same_engine_metrics` для одного engine contract и
  `cross_engine_metrics_only` с `geometric_equivalence: false` для разных.
- `openscad_customize_model` атомарно применяет Customizer и сохраняет source с
  `expected_revision`.
- `openscad_catalog_stats` показывает использование квот и outcomes без
  пользовательского SQL.
- check, compare, analyze, customize и export принимают точную историческую
  `revision`, если источником служит сохранённая модель.
- Все geometry tools остаются общими: дубли `*_manifold`/`*_brep` не
  создаются, а source contract маршрутизирует вызов.

### MCP surface

- Сервер явно поддерживает MCP 2026-07-28 и совместимые legacy-клиенты.
- SDK закреплён на точной версии `2.0.0`: lifecycle hook покрывает также
  handlers, которые modern stdio устанавливает после factory; upgrade требует
  повторного exact wire regression.
- `openscad_list_engines` и динамический `openscad://capabilities` перечисляют
  оба постоянных engine class, их provider status/maturity, language contracts,
  formats/features, effective limits, isolation, fingerprints и
  `automatic_fallback: false`; mutable status не кэшируется на сутки.
- Immutable engine manifest доступен как
  `openscad://engines/{engine_class}/capabilities/{manifest_version}`; parity resource
  ссылается на corpus/version, fingerprint, policy и последнюю qualification.
  Манифест также фиксирует exact limits/isolation, dependency/SBOM reference
  и rollback compatibility конкретного engine artifact.
- Добавлены prompts для review и безопасного customize workflow.
- Instructions и cache hints помогают агенту отличать immutable ресурсы от
  изменяемого catalog head.
- Каждый tool публикует wire schema как `success | public_error`; обе ветки
  проверяются тестами по реально отданной JSON Schema.
- Geometry success и persisted build несут один bounded execution descriptor:
  source revision/hash, language contract, required capabilities, engine class/key/fingerprint,
  `SemanticProgram` и capability-manifest versions, purpose, quality,
  representation/evidence, effective limits и `automatic_fallback: false`.
- Analyze и export используют явные purpose `analysis` и `export`; общий
  compile result не подменяет их policy/limits.

### DuckDB и конкурентность

- Source revisions неизменяемы, запись guarded CAS и квоты выполняются в одной
  serial transaction.
- Source-only update перечитывает актуальный head и сохраняет конкурентное
  name-only переименование.
- Ограничены модели, revisions, суммарный source, builds, artifacts и artifact
  bytes; pruning транзакционный.
- Build history хранит engine execution provenance, а не только метрики;
  историческую запись нельзя интерпретировать через current default
  engine после upgrade.
- DuckDB extension install/autoload и external access отключены; произвольный
  SQL через MCP отсутствует.

### Защита процесса

- Публичные ожидаемые ошибки имеют стабильные коды, retryability и следующий
  шаг; в engine-aware набор входят `engine_unavailable`,
  `language_contract_unsupported` и `capability_unavailable`. Неожиданные ошибки
  редактируются и получают correlation ID.
- Секреты, исходные excerpts и внутренние пути не попадают в persisted build
  diagnostics.
- Общий stdio transport ограничивает активные request IDs, игнорирует duplicate
  active IDs, удерживает cancelled slot до фактического settlement handler и
  сериализует запись; modern subscriptions имеют отдельный предел.
- Нормальный output и короткие overload replies имеют отдельные конечные
  очереди; burst не завершает сервер аварийно, а строковый request ID ограничен,
  чтобы busy replies не размножали большой входной frame.
- До SDK-очереди доходят только нужная инициализация и одна отмена на реально
  активную операцию или подписку; остальные notifications и входящие responses
  coalesce/drop до накопления в памяти.
- Геометрическая очередь имеет отдельный process-wide предел.

## Ранжированный следующий этап

| Приоритет | Идея | Зачем | Критерий готовности |
| --- | --- | --- | --- |
| P1 | Dual-engine geometry supervisor/watchdog | Один синхронный Manifold call пока блокирует stdio loop и не реагирует на cancel; будущий B-rep provider нельзя встраивать в stdio process. | Main process владеет только stdio/admission/DuckDB; оба provider изолированы в Worker/subprocess, имеют раздельные queues/quotas/watchdogs, без network/filesystem/secrets; deadline hard-kill перезапускает child; допустим только same-engine retry с тем же fingerprint/policy; MCP/DuckDB остаются живы. N-API требует отдельного review. |
| P2 | Content-addressed compile cache | Повторные check/analyze/compare сейчас заново компилируют одинаковый source. | Ключ включает точный source hash, language contract, engine class/fingerprint, `SemanticProgram` и capability-manifest versions, purpose/quality, policy и effective budgets; cache bounded/LRU; persisted build semantics не меняются. |
| P2 | Явный browser ↔ MCP handoff bundle | Vue workspace в IndexedDB и MCP catalog в DuckDB сознательно независимы, но пользователю нужен безопасный перенос. | Только явные export/import; bundle содержит source, document/revision metadata, hash, language contract и required capabilities; конфликт никогда не решается last-writer-wins, а imported execution record не переопределяет route. |
| P2 | Bounded parameter sweep | Агент сможет подобрать допустимые Customizer варианты и сравнить метрики. | Лимит комбинаций/времени/triangles; preview-first; отмена; лучшие кандидаты возвращаются без implicit save. |
| P3 | Granular resource change stream | Полезен долгоживущим MCP-клиентам после появления большого каталога. | Subscription не раскрывает source без read, coalesces bursts и не создаёт неограниченную очередь. |

## Что сознательно не строим сейчас

- Arbitrary DuckDB SQL, загрузку расширений и доступ к внешним файлам/URL.
- Неявную двустороннюю синхронизацию IndexedDB и DuckDB.
- Облачную синхронизацию, generic jobs/tasks framework и отдельный apps layer до
  появления реального сценария, которому недостаточно typed tools.
- Exact geometry diff: текущий compare сообщает только измеримые bounded deltas.
- Автоматическую запись каждого check/compare в историю: read-only диагностика
  должна оставаться чистой.
- Runtime engine selector, automatic cross-engine fallback и capability substitution.
- Фиктивный B-rep provider до появления общего `SemanticProgram` seam,
  развертывания и qualification.
- Отдельные дубли MCP tools на каждый engine.

## Граница IndexedDB и DuckDB

IndexedDB остаётся активным browser workspace: быстрый autosave, CAS между
вкладками и crash-recovery journal. DuckDB принадлежит только локальному MCP
sidecar и хранит явно сохранённый agent catalog, revisions, builds и artifacts.
Объединять эти хранилища внутри Vue не нужно: это добавило бы native dependency
в browser graph и создало бы неочевидные конфликты. Улучшение — явный,
проверяемый handoff bundle, а не скрытая синхронизация.
