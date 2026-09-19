# Анализ retained solid: измерения и граница Rust/TypeScript

Дата: 2026-09-19. База: `5d0e33087ceb35db7c52dd8216bf3a48517a979a`.
Это продолжение [аудита](optimization-audit-2026-09-19.md), пункт 3.
Статус: совместный анализ реализован и проверен на локальном WASM;
общая квалификация и перенос остальных стадий остаются отдельной работой.

## Изменение

До изменения `CadKernelOps.analyzeSolid` клонировал тело для нормалей,
вызывал `getMesh`, а парсер и legacy-assembler отдельно строили BVH и рёбра.
Каждый такой вызов загружал вершины и индексы из TypeScript обратно в WASM.
Внутри старых ABI-вызовов `read_f32`/`read_u32` создавали ещё и Rust-векторы.

`abi_analyze_solid` теперь принимает существующий handle тела. Rust строит
display mesh, BVH и semantic edges из одних f32-буферов. Результат живёт в
общем реестре analysis handles до копирования в отдельные TS-owned buffers;
`finally` освобождает все native-результаты одним вызовом. Промежуточное
клонирование CAD-тела и обе обратные загрузки больше не нужны.

Оба потребителя получают готовые `bvh` и `semanticEdges` через обязательные
поля `CadKernelSolidAnalysis`. Алгоритмы BVH и рёбер остаются общими с
отдельными API для импортированных сеток. Углы нормалей/рёбер, merge-пары,
порядок треугольников и provenance сохраняются.

Объём и площадь по-прежнему берутся из cached inspection. Хеширование,
`inferSurfaceIds`, проверки публикации и экспорт не включены в этот вызов.
Внутренний export snapshot для нормалей также остаётся. Поэтому это ещё не
полный `analyze_solid` из архитектурного плана.

## Результаты

Apple M4 Max, darwin arm64 25.6.0, Node 22.23.2 / V8 12.4.254.21.
Отдельный процесс на фикстуру, full quality, 3 прогрева, 9 измерений,
без профилировщиков и параллельных сборок. В таблице p50, миллисекунды.

| Нагрузка | Треугольники | Build до | Build после | Analyze до | Analyze после |
| --- | ---: | ---: | ---: | ---: | ---: |
| small-bracket | 540 | 3.329 | 3.439 | 1.017 | 0.990 |
| many-bodies, 256 кубов | 3 072 | 24.746 | 18.862 | 15.435 | 9.846 |
| dense-sphere, 3 сферы | 48 384 | 114.464 | 113.987 | 85.580 | 84.985 |

На 256 телах медиана анализа ниже на 36.2%, всей сборки на 23.8%.
Диапазоны анализа в этом прогоне: до 14.667..15.967 мс, после
9.623..10.769 мс. Для маленькой детали и плотного mesh надёжного вывода
об ускорении нет: изменения сопоставимы с разбросом. На плотной модели
по-прежнему доминирует вычисление внутри ядра, а не число ABI-вызовов.

Подписи входных фикстур, геометрии и всех самостоятельных стадий
(`outputSignatures`, включая BVH, рёбра, inspection, STL) совпали между
прогонами. Объём, площадь, число вершин и треугольников также совпали.
Геометрические SHA-256:

```text
small-bracket eb23471cddc59d5e2cb7ac8625a42ee5fdf1c8df1bd768c566213f868b2ebea0
many-bodies   a80f466dbec7a3974ee1b90948d9cb9fa36e533afdc2c5296a7deb1f142645e7
dense-sphere  fe8442fa6ea5f08f7075a04252b7d8123309c818ccf4fa2a5003e38f735f9483
```

Локальные полные отчёты с сырыми выборками и source manifests:

- [До](../../tmp/performance/solid-analysis-before/report.json), source digest
  `3bc399ed2d85d271070791d2960935ba5d98fe6b7ef806c5b1d804c142c1ff4d`.
- [После](../../tmp/performance/solid-analysis-after/report.json), source digest
  `3fb2fd985a10bcd317da61c75564515094fb5d75677842a938c057f8458cec98`.

`tmp/` не входит в Git. Команда для каждой стороны сравнения:

```sh
npm run bench:cpu -- --fixtures small-bracket,many-bodies,dense-sphere --iterations 9 --warmups 3 --out tmp/performance/solid-analysis-NEW
```

Сначала нужно дождаться полного завершения `node scripts/build-geometry-kernels.mjs`:
после сообщения cargo ещё выполняются `wasm-opt` и Brotli-упаковка.
Нельзя запускать тест или бенч на старом `bytes.ts` во время этой стадии.

Фикстура dense-sphere была уменьшена с `$fn=256` до `$fn=128` и разнесена
до x=0/90/180 **до обоих прогонов**. Это исправление baseline, не ускорение.
Прежняя нагрузка превышала ресурсные ограничения. `medium-csg` в данном
сравнении не участвовал; масштабирование BSP этим изменением не исправлено.

## Корректность и ограничения

`tests/solidAnalysis.test.ts` сравнивает байты display mesh, merge-пар,
face IDs, BVH и рёбер для куба, сфер, отверстия, пустого тела и отражённого,
неравномерно масштабированного, повёрнутого тела с большим переносом.
Проверяются независимость повторных результатов, исходные метрики и
provenance, exclusive buffers, освобождение исходного handle, `memory.grow`,
structured-clone transfer и восстановление после отказа ABI.

Выбранные 218 тестов парсера, kernel lifecycle, legacy-adapter, независимого
oracle и реальных worker-границ прошли с `--maxWorkers 2`. TypeScript
приложения и MCP прошёл проверку типов; Rust/WASM и Vite собраны.

Первый общий прогон: 3096 passed, 63 failed, 1 необработанная ошибка.
В нём присутствовали 250-мс readiness timeouts под параллельной нагрузкой,
запрет sandbox на HTTP listen, расхождения frozen fingerprints, ожидания
старых B-rep/MCP контрактов и photogrammetry artifact mismatch. Повторный
прогон с `--maxWorkers 2` и разрешённым localhost listen: **3126 passed,
33 failed**, 306 успешных файлов из 326. В нём остались ошибки join
disposable worker за 1000 мс, fingerprints и B-rep/MCP assertions. Новые
тесты анализа прошли. Причины всех оставшихся сбоев ещё не локализованы;
их нельзя автоматически считать ни регрессиями этого изменения, ни
доказанно независимым долгом.

Логи: [общий повторный прогон](../../tmp/performance/solid-analysis-tests-bounded.log),
[218 целевых тестов](../../tmp/performance/solid-analysis-targeted-final.log),
[Vite build](../../tmp/performance/solid-analysis-build.log).
Эти результаты не дают основания объявлять общий gate зелёным или менять
историческую квалификацию. `verify-dist` обнаружил два HarfBuzz-чанка вместо
одного (`harfbuzz-bytes-CWdd5xXk.js`, `harfbuzz-bytes-Cu8NF59X.js`).
Новый WASM имеет SHA-256
`7fe90559f9d65f10adc9d0edcb44cd6a7b8dae701c6827803f453cf10b74eaee`;
он не соответствует опубликованному `ownRustCadEvidence`. Нужны отдельная
проверка и новая запись evidence с сохранением исторических артефактов.

### Продолжение: общие WASM-пакеты

`openScadTextRuntime.ts` теперь является ленивой границей инициализации
HarfBuzz со статическими импортами payload и JS bindings. Раньше отдельные
динамические импорты внутри `Promise.all` давали разные namespace wrappers
в main и worker bundle: одинаковый payload попадал в два файла. После
выделения runtime оба bundle используют один `harfbuzz-bytes` размером
179520 байт. Общий размер `dist/assets` при одинаковом photogrammetry
артефакте уменьшился с 6281238 до 6100721 байт, на **180517 байт**.
Runtime остаётся ленивым; кеширование promise и retry после ошибки остаются
в `openScadText.ts`.

Пакет photogrammetry затем перегенерирован штатным скриптом для согласования
с локальным WASM. Согласованный generated `bytes.ts` сохранён в изменениях.
Все 17 тестов `openScadText`/`wasmBrotliPacking` и проверка типов проходят.
`verify-dist` теперь проходит проверки количества и содержимого WASM-пакетов,
но останавливается на общем размере: **6335456 > 5600000 байт**. Бюджет не
увеличивался; оптимизация всей сборки остаётся открытой.

### Продолжение: локализация worker join

Изолированный `mcpDirectGeometrySupervisor.test.ts` воспроизводит отказ
завершения production worker даже с одним test worker. Диагностический
прогон с join timeout 5000 мс (только в локальном probe) показал: корректный
результат куба на 386 мс, вызов terminate на 387 мс, exit на 1417 мс.
В production timeout остаётся прежним, 1000 мс.

Отдельные контрольные worker завершаются за 1..3 мс после регистрации tsx,
прогрева WASM и даже `CadKernelOps.analyzeSolid` куба. Вызов полного
`parseOpenSCAD('cube(2)')` воспроизводит примерно 1050 мс на join. Отключение
WASM tier-up в диагностическом Node-процессе и удаление наследуемых
`execArgv` проблему не устранили. Причина ещё не доказана; следующий шаг
должен локализовать orchestration парсера, не менять геометрию или тайм-аут
на основании этих гипотез. Локальные probes лежат в `tmp/performance/`.

### Исправление: один владелец завершения worker

Последующая проверка уточнила причину наблюдаемой задержки: production
worker самостоятельно закрывал `parentPort` сразу после отправки terminal,
а supervisor одновременно вызывал `terminate()`. Когда порт остаётся
открытым до terminate-and-join, тот же успешный куб в probe завершает worker
за 3 мс (terminal 333 мс, terminate 335 мс, exit 338 мс), вместо примерно
1030 мс. Версия Node та же, join deadline не менялся. Предыдущие гипотезы
о стоимости самого CAD-анализа или загрузчика это наблюдение не подтверждает.

Теперь supervisor единолично завершает worker после опубликованного
terminal. Если даже fallback-terminal не удалось отправить, дочерний worker
по-прежнему закрывает порт: родитель распознаёт exit без результата как
crash. Протокол, bounded cancellation, очередь и запрет выдачи результата
до подтверждённого join сохраняются.

38 тестов в пяти файлах (supervisor, ModelGraph, profiles, assembly, MCP CLI)
прошли, включая новую реальную последовательность failure -> success.
`bench:workers` добавляет воспроизводимую проверку штатного lifecycle без
увеличения тайм-аутов. Первый прогон: 15 started / 15 joined, 0 admitted,
0 queued, no quarantine; p50 полного build-and-join 335.58 мс, refusal-and-join
339.08 мс, capabilities-and-join 334.07 мс. Это 5 выборок на операцию и
свежий worker для каждого запроса; цифры описывают один локальный прогон.
Отчёт: `tmp/performance/worker-lifecycle-fixed.json`. Бенч включает startup,
поэтому его значения не сопоставимы напрямую с `bench:cpu`.

Аналогичное владение завершением применено к Node-адаптеру B-rep diagnostics;
браузерный адаптер не менялся. Финальная целевая проверка пяти файлов:
43 passed / 1 failed; все тесты direct supervisor, B-rep diagnostics,
изоляции и real worker boundaries прошли. Оставшийся NURBS/CSG-тест
`builds mesh CSG through the NURBS graph and handles empty intersections`
превысил 5000 мс даже при `--maxWorkers 1`. Общий прогон перед последним
изменением B-rep адаптера: 3141 passed / 19 failed. Это не зелёный общий gate.
Логи: `tmp/performance/worker-join-full-tests.log` и
`tmp/performance/worker-join-final-tests.log`.

Повторный изолированный бенч на окончательном коде:
`tmp/performance/worker-lifecycle-isolated-final.json`, 15/15 worker joined,
p50 build 343.84 мс, refusal 342.62 мс, capabilities 331.69 мс. Отдельная
попытка параллельно с typecheck отказала по 250-мс readiness timeout и не
используется для timing-сравнения. Это воспроизводимый риск cold startup
под нагрузкой, который исправление join не устраняет.

Совместный анализ удлиняет одну непрерываемую WASM-секцию: между нормалями,
BVH и рёбрами больше нет JS yield. Проверки отмены до/после анализа тела
сохраняются; нужна отдельная работа над отменой внутри ядра для крупных
тел. Эти Node-замеры не измеряют latency редактирования в браузере,
время GPU или пиковую память Rust-аллокатора.

## System Design и Следующие Шаги

### Дополнение: NURBS Subprocess Completion

После worker-исправления отдельно исследован остававшийся 5-секундный таймаут
NURBS/CSG. `runOwnNurbs` использует отдельный процесс, не direct worker.
Проба разделила загрузку модулей, build и выход: маленькие CSG-запросы
заканчивали вычисления задолго до естественного завершения процесса.
`process.exit()` после callback записи почти не помог. `--no-wasm-tier-up`
и lazy compilation также не устранили задержку; `--liftoff-only` устранил,
но отключал оптимизацию кода. Эти флаги в продукт не добавлены. Пробы
указывают на фоновую работу V8/WASM, но не являются нативным CPU-профилем
и не доказывают конкретную внутреннюю задачу V8.

Текущий протокол: child завершает stdout после единственного JSON-ответа;
parent дожидается EOF, проверяет bounded JSON, завершает disposable process
через SIGKILL и дожидается `close`. Только подтверждённый SIGKILL после
полного ответа считается штатным завершением; иные аварийные коды отказны.
Дедлайн, отмена и занятый admission slot сохраняются до `close`. Пул,
fallback, таймауты и уровень WASM-оптимизации не менялись.

Это важно для pipe backpressure: нельзя завершать процесс сразу после
`write()` и считать ответ доставленным. См. официальные контракты
[Node writable.end](https://nodejs.org/docs/latest-v22.x/api/stream.html#writableendchunk-encoding-callback)
и [child close](https://nodejs.org/docs/latest-v22.x/api/child_process.html#event-close).

Воспроизводимый бенч `npm run bench:nurbs-process -- <new.json> 5`:
тот же M4 Max, Node 22.23.2, последовательные запросы, без профайлера,
параллельной сборки или тестов. Каждый запрос запускает новый child/WASM;
OS/filesystem caches и валидация родителя могут прогреваться. Warmups
не отбрасывались. До/после содержат по 5 наблюдений на каждую модель.

| Сценарий | p50 до, мс | p50 после, мс | Изменение |
| --- | ---: | ---: | ---: |
| union | 1025.76 | 289.48 | -71.8% |
| intersection | 1055.71 | 288.61 | -72.7% |
| difference | 1049.43 | 284.91 | -72.9% |
| large STL, 3 781 604 байта | 374.05 | 375.04 | +0.3%, значимого выигрыша нет |

Отчёты: `tmp/performance/nurbs-process-before.json` и
`tmp/performance/nurbs-process-after.json`. Fixture SHA-256 и все 20/20
whole-response SHA-256 совпали. Отчёт сохраняет отдельные samples,
выбранные source/kernel fingerprints и среду. Это end-to-end latency
Node API, не ускорение CSG или браузера. Исходный CSG-тест теперь проходит
без изменения 5000-мс таймаута; отдельный тест проверяет полное равенство
multi-megabyte результата прямому расчёту.

Mock-проверки покрывают EOF против `close`, удержание обоих admission slots,
split UTF-8, неверный/обрезанный JSON, чужой exit code, отмену после EOF,
30-секундный deadline до join, overflow, read error и неотправленный kill.
Реальные subprocess-тесты покрывают экспорт, refusal и cancellation recovery.
Финальная целевая проверка: 63 passed в 7 файлах (`--maxWorkers 1`), включая
direct/B-rep lifecycle, production isolation, real worker boundary и CAD
operations. Лог: `tmp/performance/nurbs-process-integration-tests.log`.
`vue-tsc --noEmit`, `tsc --noEmit -p tsconfig.mcp.json` и `git diff --check`
прошли. Полный suite после этого изменения не перезапускался.
Эти результаты не закрывают общий gate: readiness под CPU-нагрузкой,
устаревшие qualification manifests и bundle budget остаются отдельными задачами.

### Дополнение: Rust WASM Bootstrap

Readiness-проверка вызывает `CadGeometryKernel.warm()`: распаковку WASM,
компиляцию, instantiation и создание служебных объектов адаптера.
Отдельная диагностическая проба на Node 22.23.2 / M4 Max показала около
76 мс на decode, 7-8 мс на compile и 22 мс на adapter setup при одном
свежем worker. При четырёх одновременных worker full warm занимал
98-129 мс. Это наблюдение, не гарантия 250-мс готовности под произвольной
нагрузкой. Проба: `tmp/performance/kernel-startup-phases.jsonl`.

Вместо увеличения таймаута проверены Rust-профили независимого
`wasm-brotli` bootstrap. `cargo tree -i brotli-decompressor` и
`cargo tree -i alloc-stdlib` подтвердили: эти зависимости используются
только им, не геометрическим ядром. Новый профиль не меняет ABI,
4/16 MiB input/output caps и 64 MiB maximum linear memory.

Варианты собирались `cargo build --locked --release --target
wasm32-unknown-unknown --manifest-path crates/wasm-brotli/Cargo.toml`
в отдельных `--target-dir`, с одинаковыми `profile.release.strip="symbols"`,
`codegen-units=1`, `panic="abort"` через `--config`. Только четыре
`profile.release.package.<crate>.opt-level` переопределялись на `z`, `2`
или `3`; geometry sources и compressed geometry input не менялись.

Бенч: `npm run bench:bootstrap -- --decoder <z.wasm> --decoder <2.wasm>
--decoder <3.wasm> --out <new.json> --samples 9`. Варианты запускались
по одному в новом worker/isolate, порядок чередовался. Все декодировали
тот же геометрический пакет; полное совпадение с `kernel_bg.wasm`
проверялось вне измеряемого интервала. Время импорта, запуска worker,
подготовки compressed decoder literal, чтения файлов и join не входит;
process-wide V8/OS caches могут прогреваться. Forced GC/profiling не было.

| Профиль | WASM, байт | Packed JS literal, байт | p50 bootstrap, мс |
| --- | ---: | ---: | ---: |
| z | 219 834 | 124 380 | 75.42 |
| 2 | 210 269 | 122 924 | 53.13 |
| 3 | 213 857 | 123 752 | 52.40 |

Выбран `2`: на 29.6% меньше времени и на 1456 байт меньше literal против
`z`; разница с `3` около 0.7 мс при меньшем артефакте. Это сравнение
только данного bootstrap на зафиксированной среде, не правило для
остальных Rust-крейтов. Настройка в `crates/Cargo.toml` ограничена
`wasm-brotli`, `brotli-decompressor`, `alloc-no-stdlib`, `alloc-stdlib`.

Повторная сборка стандартным `buildWasmBrotli()` дала тот же бинарный
SHA-256, что измеренный кандидат:
`8f6e7c931234c7dc9c3dbf6ddd602c0b121916772128271b28a6df8a36f93bff`.
Геометрический WASM не изменился:
`7fe90559f9d65f10adc9d0edcb44cd6a7b8dae701c6827803f453cf10b74eaee`.
Generated decoder literal, как и раньше, восстанавливается сборочным
скриптом и не отслеживается Git.

End-to-end `bench:workers`, по 9 samples, тот же production runtime:

| Сценарий | p50 до, мс | p50 после, мс | Изменение |
| --- | ---: | ---: | ---: |
| refusal + join | 235.82 | 212.49 | -9.9% |
| build + join | 237.88 | 213.26 | -10.3% |
| capabilities + join | 235.47 | 214.30 | -9.0% |

27/27 worker joined в каждом прогоне, без quarantine. Среди
fingerprinted inputs изменились только decoder literal и Cargo-профиль.
Worker и NURBS бенчи теперь включают decoder/packing/profile в fingerprints.
Исторические результаты 343 мс из предыдущей серии не используются как
baseline для этого сравнения: производительность среды между сериями
заметно менялась. Current pair запускался последовательно, без других
сборок/тестов; CPU frequency/isolation не фиксировались.

Raw reports: `tmp/performance/wasm-bootstrap-z-vs-2-vs-3.json`,
`tmp/performance/worker-bootstrap-before.json`,
`tmp/performance/worker-bootstrap-after.json`.
40 TS-тестов packing/warmup/kernel/provider и 3 release Rust-теста прошли.
Общий прогон после изменения: 3162 passed / 14 failed, 327 файлов,
`--maxWorkers 2`, 100.49 с. В этом прогоне не было readiness-таймаутов
или worker join failures. Оставшиеся 14 отказов: 9 проверок исторических
fingerprints/qualification, 2 старых ожидания отказа уже поддерживаемых
B-rep операций, 2 списка MCP-инструментов и 1 отсутствие image attachments
в mechanical MCP response. Лог: `tmp/performance/bootstrap-full-tests.log`.
Это один успешный lifecycle-прогон, не доказательство устойчивости при
любом распределении CPU. Исторические evidence/manifest файлы не менялись.

`vue-tsc --noEmit`, MCP `tsc --noEmit`, `git diff --check` и `vite build`
прошли. `verify-dist` проверил WASM-пакеты, но отказал по total budget:
6 334 000 байт при лимите 5 600 000. Против предыдущей сборки ровно
-1456 байт, что не закрывает общий перерасход. Логи:
`tmp/performance/wasm-bootstrap-vite-build.log` и
`tmp/performance/wasm-bootstrap-verify-dist.log`.
Этот шаг не меняет ни 250-мс readiness timeout, ни startup/job/join budgets,
ни протокол запуска. Отдельное проектирование cold preparation против
readiness остаётся открытым, как и проверка браузерного cold startup.

### Оставшиеся Направления

Следующий проверенный шаг: [exact mesh inspection](mesh-inspection-2026-09-19.md).
Сортированный упакованный индекс рёбер заменил дерево и повторную сборку
топологии; на той же плотной CPU-фикстуре build снизился с 72.77 до 56.39 мс.
Проверки валидности, ориентации и замкнутости сохранены; это не новый сертификат
отсутствия самопересечений.

Продолжение по плотным сеткам: [подготовка display mesh](mesh-render-2026-09-19.md).
Плоская смежность и группы нормалей по исходной вершине сохраняют байты;
render для 48 384 треугольников ускорен с 25.82 до 20.29 мс. Полная сборка
в этой локальной паре: 77.15 → 73.85 мс, а не 21% выигрыша всего пути.

Следующий выполненный шаг: [триангуляция профилей с отверстиями](profile-triangulation-2026-09-19.md).
Плита 4×4 больше не уходит в BSP-fallback (103.31 → 10.39 мс);
поддержаны 64/100 отверстий с проверкой объёма и замкнутости. Это ограниченный
ear-clipping, не замена общего булеана или робастная CDT-триангуляция.

1. У плотного mesh следующий кандидат на профиль: `render_buffers`, включая
   группировку плоскостей, adjacency и ключи нормалей. Перенос дополнительного
   кода в Rust сам по себе не доказывает ускорение уже нативного алгоритма.
2. Для интерактивной работы нужны отмена внутри долгих операций и кеш
   неизменившихся подграфов. Один владелец handle и срок жизни по build job
   должны предшествовать кешу; иначе выигрыш времени превратится в удержание
   native-памяти и публикацию устаревшей геометрии.
3. Робастный boolean и триангуляция отверстий остаются первыми работами по
   расширению допустимых моделей. Они требуют отдельных correctness cases
   и сравнения одинаковых нагрузок, включая отказы, а не повышения бюджетов.
4. Валидация worker payload, `inferSurfaceIds` и анализ imported meshes
   остаются следующими кандидатами на объединение проходов. Глубокую
   проверку внешних данных нельзя просто убрать ради числа в бенче.
5. Перед публикацией: закрыть cold readiness под нагрузкой, согласовать
   текущие артефакты и манифесты, уменьшить общий bundle до его бюджета
   и устранить оставшиеся ошибки общего gate. Join lifecycle и дублирование
   HarfBuzz исправлены выше; это не закрывает перечисленные отдельные проблемы.

Свежая проверка первичных источников конкурентов, 2026-09-19:

- [Manifold Performance Considerations](https://github.com/elalish/manifold/wiki/Performance-Considerations)
  описывает ленивый CSG-граф, переиспользование результатов, flattening и
  порядок операций по размеру. Вывод для проекта: сохранять геометрические
  выражения до материализации и измерять полный путь до готового mesh;
  время одного вызова union не сравнимо между eager и lazy backend.
- [OpenSCAD Playground](https://github.com/openscad/openscad-playground)
  документирует Monaco, autocomplete импортов и символов, стандартные
  библиотеки, PWA и Manifold по умолчанию. Это конкретные ориентиры для
  расширения редактора. Наличие функции в README не является независимым
  тестом её качества или скорости.

Если проектировать с нуля: семантический граф и job ownership отдельно от
геометрического backend; один native analysis result на публикуемый asset;
TS отвечает за UX, идентичности и передачу результатов. Текущий шаг
реализует только совместную подготовку display mesh/BVH/рёбер. Он не
подменяет рефакторинг эвалуаторов, кеширование или выбор boolean-алгоритма.
