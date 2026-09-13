# Актуализация B-rep — 2026-09-12

Дополнено 2026-09-13 после проверки Code → Solid и истории изменений Solid.

Пакет подготовлен по исполняемому коду и проверкам рабочей копии
`open-scad-viewer`. Полное завершение B-rep **не подтверждено**. Точная матрица
реализации, ограничений и свидетельств находится в
[аудите завершения](../../design/brep-completion-status.md).

## Наблюдаемая реализация

- Рациональные цилиндры, конусы, трубы, сферы и торы; вращение профилей,
  включая частичные обороты и полюса. Поверхности сохраняются отдельно от сетки.
- Boolean над плоскими телами и ограниченным семейством параллельных
  выдавливаний из прямых и окружностей: union, intersection, difference, XOR.
  Поддерживаются разные высоты, ступени, глухие карманы, внутренние полости,
  отверстия, несколько тел и канонический пустой результат.
- Ступенчатые тела повторно участвуют в операциях после сериализации,
  поворота и отражения. Внутренние грани удаляются по границам исходных ячеек;
  сопоставление не восстанавливает поверхность из треугольников.
- ModelGraph JSON/text предоставляет `brep_extrude_curves` для вложенных
  ссылок на 2D NURBS-контуры. Прямые и рациональные круговые дуги сохраняются.
- Новый внутренний SemanticProgram backend исполняет примитивы, профили,
  поддержанные Boolean, преобразования и прямое выдавливание. Он сохраняет
  типизированную пустоту, неизменяемые снимки и проверяемое владение ресурсами.
  Адаптер сцены сохраняет отдельные объекты, цвета, исходные идентификаторы,
  грани и разные хэши программы, исходника, геометрии и детализации.
- Диагностические browser/Node worker исполняют этот backend с принудительной
  отменой, таймаутами, проверкой согласованности программы/снимков/меша и
  завершением worker до публикации в Node. Проверены реальные native-отказы,
  восстановление и совпадение browser/Node. Production route этим не активирован.
- Исправлен независимый тестовый interval-oracle: underflow больше не выдаёт
  ложный точный ноль. Добавлены крайние значения и сравнение квадратов расстояний;
  это проверочная основа, а не сертификация текущих Boolean-операций.
- Добавлен отдельный Rust-кандидат `cad-predicates`: знаки ориентации 2D/3D
  и сравнение квадратов расстояний с точными разложениями, неизменяемыми
  контекстами и жёсткими лимитами. Debug/release проходят по 17 тестов;
  независимый корпус даёт 172 правильных решения и один явный отказ по точности.
  Подключение к геометрическому runtime и конструкции ещё не выполнены.
- В версии 2 этого кандидата добавлено пересечение двух прямых с сохранением
  формулы построения, исходных ссылок и интервальных координат/параметров.
  Последующая ориентация работает с этой формулой, не с округлённым центром.
  Матрица из 200 случаев проверена отдельной рациональной арифметикой.
  Итоговые debug/release: по 23 теста; Clippy без предупреждений.
  Общие кривые, цепочки конструкций и подключение к B-rep пока остаются открытыми.
- Версия 3 добавляет цепочки 2D пересечений с общими неизменяемыми зависимостями:
  до 32 уровней и 256 узлов, повторное точное вычисление без округлённых центров.
  Проверены границы лимитов, сохранение исходных точек при отказе и 100 вложенных
  пересечений по отдельному рациональному oracle. 3D и интеграция ещё открыты.
  Итог версии 3: debug/release по 29 тестов, Clippy без предупреждений.
- Исправлен Code → Solid: перенос сохраняет исходный B-rep, рациональные веса
  и размещение. В production UI корпус сохраняет свойства поверхностей и
  повторную тесселяцию; native-объём 7619,8099 мм³ не меняется с детализацией.
  Пересчёт сетки теперь сохраняется в истории; Undo возвращает прежнюю сетку.
  Исправлена потеря нового документа при первом асинхронном открытии Solid.
  Проверки переноса/пустых результатов: 11; проверки Vue-интерфейса: 21 — проходят.
- Тесселяция использует общие индексы рёбер из B-rep-топологии. Массовые
  свойства интегрируются по исходным поверхностям; результат остаётся
  численной оценкой, не сертифицированной границей ошибки.
- Сжатие WASM без потерь использует общий ограниченный Brotli-декодер.
  Проверка дистрибутива распаковывает geometry, HarfBuzz, photogrammetry
  и сам bootstrap, сравнивая их с исполняемыми исходными артефактами.

## Что остаётся открытым

Общий Boolean по произвольным NURBS-поверхностям, полный pipeline пересечений
и UV-trim, сертифицированные геометрические предикаты и конструкции,
аналитические скругления, общий offset/shell, STEP/IGES и полная квалификация.
Постоянный production provider `openscad-viewer/brep-1` ещё не активирован.
Внутренний SemanticProgram backend не означает его доступность через
production browser/MCP route. Явная политика равномерной тесселяции
не доказывает исполнение исходных chord/angle tolerances.
Последний общий JS-прогон имеет 2490 успешных проверок и четыре отказа frozen
oracle/квалификации. Два mesh-byte расхождения точно объяснены прежними изменениями
куба и нормализации, но три числовых ID граней несовместимы между версиями.
Ожидаемые хэши не переписаны; полная квалификация остаётся незавершённой.

## Состояние mcp-rag

Запись этого пакета **не выполнена**. После первоначального отказа подключения
gateway `http://127.0.0.1:7432/mcp` снова ответил 2026-09-13. Выполнены MCP
initialize, tools/list, status, doctor и ограниченный запрос каталога; сервер
представился как `rag-mcp`, protocol `2025-06-18`.

Однако `status` показывает другую базу: `Documents/Sources/rag/rag.duckdb`,
26 документов / 322 chunks, `mock` / `text-embedding-3-small` / 1536.
Исторический receipt указывает `~/.local/share/rag-mcp/rag.duckdb`,
82 844 документа / 643 994 chunks, ollama / nomic-embed-text / 768.
Это не подтверждённое восстановление прежнего корпуса. Пользователю направлен
вопрос о целевой базе; до ответа документы не записывались. Новый writer,
новая база, ремонт чужого содержимого и замена embedding-модели не запускались.
Сопоставление возвращённых идентификаторов сохранено в
[gateway-observation.json](gateway-observation.json).

`ingest-manifest.json` содержит последовательные upsert-аргументы и текущие
хэши только выбранных документов. Перед записью их следует перепроверить:
рабочая копия продолжает изменяться. После получения document IDs требуется
прочитать записанное содержимое и проверить хэши; только затем можно создать
receipt со статусом completed. Исторические qualification-пины и прежние RAG
receipts этим пакетом не изменяются. Широкая индексация generated/output/
node_modules и запуск второго writer к существующей DuckDB не нужны.

### Кандидат 4 — точные 3D-конструкции

Добавлены пересечение прямой с плоскостью и проверка стороны плоскости
с сохранением исходного графа построений. Проверены рациональные координаты
и параметры, перестановки плоскости, точная принадлежность, случай 1/3,
вложенные построения до глубины 32 и вырожденные случаи. Debug/release:
32 теста проходят; Clippy без предупреждений. Это численный фундамент,
пока не подключённый к runtime; полная готовность B-rep не доказана.

### Кандидат 5 — конечные области пересечения

Для сохранённого единственного пересечения добавлена точная классификация
относительно исходных отрезка и треугольника: концы, вершины, рёбра,
внутренняя и внешняя область. Решения принимаются по точным числителям
параметров. Проверены 735 комбинаций границ и перестановок, рациональные
наклонные плоскости, различие в один шаг binary64 и вложенные построения.
34 теста проходят в debug/release, Clippy без предупреждений.
Копланарные перекрытия и подключение к runtime остаются открытыми.

### Кандидат 6 — копланарные перекрытия

Добавлено точное отсечение отрезка треугольником в общей плоскости.
Различаются пустое пересечение, касание и общий участок; исходные
построения сохраняются, параметры выдаются как внешние интервалы.
Проверены 1 344 комбинации геометрии, размещения и ориентации, а также
вложенные построения глубины 32. Debug/release: 36 тестов; Clippy чистый.
Создание точек концов перекрытия, общие trim-структуры и интеграция
с runtime ещё не завершены.

### Кандидат 7 — точные концы перекрытия

Концы копланарного перекрытия теперь строятся как `ConstructedPoint3`
с сохранением исходных точек и выбора границы. Их можно использовать
в последующих пересечениях и проверках без округлённой переавторизации.
Проверены координаты на преобразованных примерах, точная принадлежность
плоскости, равенство концов касания и сохранение 4/3 до глубины 32.
38 тестов проходят в debug/release, Clippy без предупреждений.
Общая топология обрезки и интеграция в runtime остаются открытыми.

### Кандидат 8 — единый запрос отрезок–треугольник

Обычные и копланарные случаи объединены в запрос с результатами: пусто,
точка или общий участок. Точки сохраняют точные построения и положение
на границах исходных объектов. Оба конца должны завершиться в общем
бюджете и уложиться в совместный лимит узлов; частичный результат
при отказе не возвращается. 40 тестов проходят в debug/release,
Clippy без предупреждений. Общая B-rep-топология и runtime ещё не закрыты.

### Точное сравнение 3D-точек

Добавлено точное лексикографическое сравнение исходных и построенных
точек без округления и привязки к идентичности рецепта. Проверены
эквивалентные построения, отличие точной 1/3 от binary64, порядок
по следующим координатам и отмена. 41 тест проходит в debug/release,
Clippy без предупреждений. Это основа для объединения совпадающих
пересечений; топологическая идентичность и runtime пока не реализованы.

### Пересечение некопланарных треугольных граней

Добавлена сборка общего отрезка или касания из точных пересечений рёбер.
Совпадающие точки объединяются по точным координатам. Проверены обмен
гранями и смена ориентации; копланарность и вырождение выделены отдельно.
42 теста проходят в debug/release, Clippy без предупреждений.
Копланарный многоугольник пересечения и общая B-rep-топология ещё открыты.

### Кандидат 9 — копланарные контуры

Пересечение копланарных треугольников теперь собирается в выпуклый
контур из 3–6 точных построенных вершин. Касания остаются точкой
или отрезком. Проверены порядок обхода, отсутствие лишних коллинеарных
вершин, шестигранное пересечение и размещение в разных плоскостях.
44 теста проходят в debug/release, Clippy без предупреждений.
Общие криволинейные обрезки и интеграция с B-rep/runtime ещё открыты.

### Рациональная проверка контуров

172 невырожденные пары треугольников сверены с отдельным рациональным
алгоритмом последовательного отсечения. Проверены количество и координаты
вершин, дробные пересечения и пустые результаты. 45 тестов проходят
в debug/release; Clippy без предупреждений. Это дополнительное
регрессионное свидетельство, не закрытие полной готовности B-rep.

### Принадлежность точки обеим граням

Добавлена точная классификация исходной или построенной точки относительно
произвольного треугольника. Проверены все признаки границы и обе стороны
плоскости; вершины пересечений в 172 парах проверены на обеих гранях.
46 тестов проходят в debug/release, Clippy без предупреждений.
Постоянная B-rep-топология и интеграция с runtime остаются открытыми.

### Точные вершины в общей топологии

`brep-topology` поддерживает геометрию вершины, задаваемую владельцем,
и проверку её исходного контекста через callback. Интеграционный тест
хранит точный контур пересечения в индексированной грани без округления.
106 debug-тестов ядра/топологии, пять release-тестов топологии и проверка
всего Rust workspace проходят. Clippy самого модуля чистый; полный
запуск с зависимостями выявляет замечания в math-core. Производственное
подключение предикатов и сериализация рецептов пока открыты.

### Неизменяемые снимки топологии

Добавлены снимки с сохранённой политикой проверки вершин и правкой
изолированной копии. Проверены независимость соседних снимков, откат
при ошибке и отказ при точке из чужого контекста. Пять тестов топологии
проходят в debug/release, Clippy модуля чистый. Версионное хранилище,
защита от устаревшего commit и производственные транзакции ещё открыты.

### Версионная публикация топологии

Добавлено хранилище, принимающее подготовленную правку только для
соответствующих хранилища и исходной ревизии. Проверены устаревшие
и чужие транзакции, возврат к прежней геометрии и переполнение счётчика.
Шесть тестов топологии проходят в debug/release; Clippy модуля чистый.
Производственная маршрутизация транзакций и их сохранение ещё открыты.

### Ограниченная история Undo/Redo

Версионное хранилище сохраняет до 32 прошлых снимков. Undo/Redo
увеличивает ревизию, новая ветка правок очищает Redo. Проверены
вытеснение истории, устаревшие правки и сохранение точных вершин.
Семь тестов проходят в debug/release; Clippy модуля чистый.
Подключение истории к рабочему интерфейсу и её сохранение ещё открыты.

### Проверка всей геометрии перед публикацией

Снимок может сохранять валидатор всей модели. Интеграционный тест
с реальным brep-core отклоняет повреждённые NURBS и отрыв кривой
от вершины, сохраняя исходное состояние. Семь тестов топологии
и один тест интеграции проходят в debug/release; Clippy модуля чистый.
Сила проверки ограничена существующим валидатором ядра; полная
геометрическая сертификация и производственная маршрутизация открыты.

### Финальная проверка публикации

Добавлена проверка отмены/условий владельца непосредственно перед заменой
снимка. Отказ сохраняет модель, ревизию и Undo/Redo; устаревшая правка
отклоняется до вызова проверки. Два теста интеграции проходят в debug/release,
семь тестов топологии — в debug; Clippy модуля чистый. Подключение к
производственному выполнению операций остаётся открытым.

### Транзакции полной модели ядра

ModelStore хранит геометрию вместе с таблицами идентификаторов и lineage.
Булево вычитание с изменением топологии проверено через транзакцию,
Undo/Redo и отказ при повреждённых идентификаторах. Проходят 111
debug-тестов ядра/топологии, 10 целевых release-тестов и проверка
всего Rust workspace; Clippy топологии чистый. Производственная
маршрутизация UI/WASM и сохранение рецептов ещё открыты.

### Сохранение снимка модели

Добавлен строгий JSON-формат снимка с версией, явными идентификаторами
топологии и лимитом 8 МиБ. Загрузка повторно валидирует ядро; дубликаты
ключей, неизвестные поля/версии и повреждённая геометрия отклоняются.
Четыре транзакционных теста проходят в debug/release, тесты value-codec
— в debug. История, ожидающие транзакции и точные рецепты пока не сохраняются.

### Сохранение истории модели

Архив сохраняет текущий снимок, Undo и Redo с повторной проверкой всех
моделей. Лимиты: 32 записи истории и 32 МиБ на архив. Восстановление
создаёт новое хранилище и отклоняет старые подготовленные транзакции.
Пять интеграционных тестов проходят в debug/release, восемь тестов
топологии — в debug; Clippy модуля чистый. Точные рецепты построений
и подключение к сохранению проекта остаются открытыми.

### Неизменность политики допуска

Подготовка транзакции проверяет сохранение политики допуска модели.
Тест с ошибочной реализацией владельца подтверждает отказ без изменения
состояния. Девять тестов топологии проходят в debug/release, пять тестов
интеграции ядра — в debug; Clippy модуля чистый. Полная готовность B-rep
и производственная интеграция остаются открытыми.

### Проверка аналитических моделей в истории

Сфера и тор проверены через сохранение/загрузку и Undo/Redo: сохраняются
полная геометрия, идентификаторы, вершины полюсов и биты рациональных
весов. Шесть интеграционных тестов транзакций проходят в debug/release.
Общие криволинейные операции и сохранение точных рецептов ещё открыты.

### Пустой результат в транзакциях

Проверен цикл вычитания тела из себя, сохранения пустой модели, загрузки
и Undo/Redo без оставшихся граней или идентификаторов. После загрузки
проверено добавление нового тела и возврат к пустому состоянию. Семь
интеграционных тестов проходят в debug/release. Производственное
подключение и полная готовность B-rep остаются открытыми.

### Переносимый граф 3D-построений

Экспорт сохраняет исходные биты/дроби, индексы листьев, операции,
общие зависимости и параметры контекста без адресов памяти и округлённых
центров. Проверены рациональные данные и общий DAG. 47 тестов предикатов
проходят в debug/release; Clippy чистый. Безопасное восстановление графа
и подключение к файлу проекта пока открыты.

### Восстановление точного графа

Добавлено восстановление в уже допущенном контексте с проверкой
исходных значений и повторным выполнением операций. Сохраняются
общие зависимости и точные дробные результаты. Подмена данных,
ссылки вперёд, лишние узлы и неверный контекст отклоняются.
49 тестов проходят в debug/release; Clippy чистый. Кодирование
в файл и интеграция с проектом остаются открытыми.

### Бинарный формат точных графов

Добавлен BRG3 v1 с исходными битами/дробями, зависимостями и лимитом
256 КиБ. Декодирование не создаёт допущенную геометрию: нужен отдельный
replay в проверенном источнике. Проверены точный −1/9, все обрезанные
префиксы, лишние байты и некорректные длины. 50 тестов проходят
в debug/release; Clippy чистый. Интеграция с проектом ещё открыта.

### Исправление стоимости восстановления

Тест глубины 32 выявил исчерпание бюджета из-за повторного вычисления
предков. Replay переведён на однократное вычисление узлов с ограниченным
кэшем точных значений. Цепочка теперь восстанавливается в стандартном
бюджете с точной 4/3. 50 тестов проходят в debug/release, Clippy чистый.
Версия реализации — candidate-10; общая интеграция ещё открыта.

### Exact vertex recipes through the indexed topology codec

The shared Vertex/Model codec now supports application-owned vertex payloads
through explicit value-codec trait bounds. The default binary64 wire shape is
unchanged; ConstructedPoint3 still has no implicit rounded serialization.
A native integration test exports all six exact intersection vertices to bounded
binary recipes, serializes the indexed contour as JSON, strictly decodes it,
replays against the admitted source, and revalidates incidence, exact equality
and membership in both source triangles. Curve/surface payloads in this test
are unit placeholders, so this demonstrates retained exact vertices and incidence,
not a certified trimmed surface or solid archive.

Validation: 121 tests across brep-core, brep-topology and value-codec pass in
debug; all nine topology tests pass in release. Module-only topology Clippy
with warnings denied and the native workspace check pass. Project UI/WASM
persistence integration and complete B-rep certification remain open.

### Atomic bounded vertex batch replay

`replay_point3d_batch` now decodes and restores an ordered set of binary vertex
recipes against one admitted source/context and one cumulative work budget.
Limits are 256 roots, 256 KiB per record and 8 MiB total encoded input; excessive
root count is rejected before scanning records. Decoding bytes consume work.
Malformed records, foreign source metadata and any indeterminate replay discard
the whole result; consumed work is not rolled back. This is atomic result
publication, not a topology/solid certificate or hard real-time cancellation.

The indexed six-vertex contour roundtrip now uses this batch path. Regressions
cover exact rational recovery, cumulative budget exhaustion after a successful
first vertex, corruption and foreign source in later records, count/per-record/
total byte limits, empty batches and cancellation. All 60 predicate/topology
tests pass in debug and release; module-only all-target Clippy passes with
warnings denied. Production project persistence and general curved B-rep
operations remain incomplete.

### Bounded batch export and publication checks

`export_point3d_batch` completes the save side of the vertex archive path:
ordered roots share one predicate work budget and the existing 256-root/8-MiB
batch limits. Individual recipes retain exact authored values and construction
DAGs. Failed exports return no partial archive and retain consumed work. Both
export and replay check cooperative cancellation/deadline immediately before
returning a successful batch; this does not prevent cancellation after return.
The six-vertex indexed topology integration now uses batch export and replay.

Regression evidence covers single-versus-double cumulative export cost,
second-root budget exhaustion, root count bounds, and cancellation/deadline on
empty batches. 61 predicate/topology tests pass in debug and release;
module-only all-target Clippy with denied warnings passes. This remains native
candidate infrastructure; UI project persistence and general curved B-rep
certification are still open.

### Ruled surface sections in either parameter direction

Surface/plane queries now admit ruled rational surfaces linear in U with matching
row weights, in addition to the existing linear-V case. The numerical algorithm
runs on the transposed retained surface and maps parameter boxes, unresolved
regions, samples and procedural traces back to original UV coordinates.
The new serialized `ruled_u` trace retains the source surface and V interval;
the TypeScript trace union includes this representation.

Tests cover horizontal/oblique cylinder sections and generator lines with shifted,
scaled knot domains, serialized trace evaluation, original-surface residuals and
budget-exhausted UV regions. All 10 intersection unit tests pass in debug and
release; native workspace check passes. Transposed floating-point evaluation can
differ in the last bit, so geometry is checked numerically, not claimed exact.
Coverage remains numerical_uncertified with topology changes forbidden. Browser
WASM packaging has not been rebuilt in this checkpoint; general surface/surface
intersection and curved Boolean certification remain open.

### Linear-U sections reach the packaged WASM adapter

The geometry WASM was rebuilt with linear-U sections. Four TypeScript/WASM
intersection tests pass, including JSON roundtrip of ruled_u traces, shifted
UV domains, cylinder section residuals, generator lines, invalid fractions and
budget-exhausted parameter regions. vue-tsc --noEmit passes. The packed geometry
literal was decoded through verifyPackedWasmChunk and matched the raw module
byte-for-byte (6,446,961 bytes). This verifies the generated adapter artifact;
a fresh production dist/browser interaction was not exercised here. General
curved Boolean and certification work remains open.

### Whole-domain admission of retained section traces

Trace evaluation now validates both line endpoints or both ruled interval ends
against the retained surface domain before interpolation. A valid requested
fraction cannot hide a malformed unused endpoint. Finite, in-domain reversed
intervals remain admitted for oriented traversal. Linear-U traces inherit the
same checks through parameter transposition.

All 11 native intersection tests pass in debug/release, including nonfinite and
out-of-domain endpoints and reverse traversal. Geometry WASM was rebuilt; all
four adapter tests pass with malformed serialized U/line traces rejected even
at fraction zero. vue-tsc --noEmit passes. These are parameter admission checks,
not a geometric completeness certificate; general curved Boolean remains open.

### Rational rulings with unequal endpoint weights

Surface/plane sections now admit linear-U/V rational rulings with unequal
positive endpoint weights. Trace evaluation computes both boundary homogeneous
weights using one normalization and solves the weighted plane equation for the
ruling parameter. Control-coefficient exclusion and unresolved-region handling
remain in place; no tessellation or fitted curve is introduced.

Native tests verify both parameter directions and serialized traces against the
analytic half-height parameter 1/4 for a 1:3 boundary weight ratio. WASM tests also
vary the ratio along the surface and compare UV against an independent quadratic
Bernstein formula. All 12 native intersection tests pass in debug/release; after
WASM rebuild all five adapter tests pass, and vue-tsc --noEmit passes. Query
coverage remains numerical_uncertified and cannot authorize topology changes.
General curved surface/surface Boolean remains incomplete.

### Knot-refined linear-U surfaces

Surface/plane orientation dispatch now uses degree one in U/V rather than
requiring exactly two global control rows/columns. Knot-refined linear-U
surfaces are transposed and decomposed into retained knot-span patches before
sectioning, so geometry-preserving knot insertion no longer disables support.
Tests insert a midpoint knot in a cylinder's linear direction and verify
sections on both sides, original UV, patch parameter boxes and cylinder residuals.

All 13 native intersection tests pass in debug/release. Rebuilt WASM passes all
six intersection adapter tests; vue-tsc --noEmit passes. This does not establish
sewing/completeness at knot-boundary contacts or authorize topology changes;
full curved Boolean and B-rep certification remain incomplete.

### Sections coincident with a ruling-span boundary

When one entire boundary coefficient row lies numerically on the plane and the
opposite row has a strict Bernstein side, sectioning emits the retained UV
isocurve instead of repeatedly subdividing an unresolved boundary band. A
continuous interior knot is owned by the preceding span to avoid duplicate
curves. This uses the query's existing numerical zero convention, not a new
exact certificate. Disconnected full-multiplicity knots remain rejected by
NURBS admission; they require separate geometry nodes.

All 14 native intersection tests pass in debug/release, including a section
exactly through an inserted knot and disconnected-input rejection. Rebuilt WASM
passes six adapter tests, now covering both exterior boundaries and the shared
knot. General branch sewing, curved Boolean and full certification remain open.

### Regression checkpoint after ruled-section expansion

The complete debug suites for brep-core, brep-topology and nurbs-core pass:
130 tests, including kernel operations, identities, archives, mass integration,
incidence and intersection queries. Six related TypeScript/WASM test files pass
43 tests covering analytic primitives, mass, regularized/stepped Boolean,
ModelGraph and intersections. These results establish regression evidence for
the current tested envelope, not the full qualification matrix.

The main requirements table now reflects candidate-10 recipe persistence,
versioned native history archives and the expanded ruled-section matrix.
General CC/CS/SS coverage, UV arrangements, production transaction routing,
STEP/IGES and certified curved Boolean are still missing; no completion status
or frozen qualification evidence was promoted by this checkpoint.

### Generator sections with proportionally weighted boundaries

The generator reduction now also recognizes a finite positive common ratio
between boundary weights when corresponding control points have equal plane
distances. In this admitted numerical case both boundary plane equations share
the same roots, so unequal weights no longer force an unresolved subdivision
band. The retained surface still controls rational traversal along the ruling.

All 15 native intersection tests pass in debug/release. Analytic cylinder tests
cover U/V orientation and z(t)=12t/(1+2t) for a 1:3 weight ratio. Rebuilt WASM
passes seven intersection adapter tests; vue-tsc --noEmit passes. The common
ratio/equality checks are numerical and do not establish certified completeness
or authorize topology changes. General curve/surface and surface/surface work
remains open.

### Nonproportional-weight section oracle

A regression now varies upper boundary weights by 2, 3 and 5 and cuts the
result with a vertical plane. It verifies that nonempty retained ruled branches
are not mislabeled as whole generator lines. Samples are checked against an
independent quadratic rational Bernstein formula for X and Z. With 128 boxes,
endpoint branch bands remain explicitly unresolved and coverage is incomplete;
this is an observed remaining solver limitation, not a complete section claim.

All 16 native intersection tests pass in debug/release, and eight tests pass
through the current WASM adapter. No runtime algorithm or WASM artifact changed
in this checkpoint. Complete endpoint isolation/branch joining and general
surface intersection remain required for the full B-rep objective.

### Fair bounded traversal across surface spans

A failing regression demonstrated that depth-first section traversal exhausted
eight boxes refining a difficult second span before processing a simple first
span. Surface subdivision now uses a FIFO queue: original spans are visited
before child refinement, then subdivision proceeds by depth. Existing box and
component budgets and explicit unresolved regions remain. Component traversal
order may change; components have no persistent topology identity.

The regression now retains the entire simple span within the same eight-box
budget. All 17 native intersection tests pass in debug/release, rebuilt WASM
passes nine adapter tests, and vue-tsc --noEmit passes. Fair scheduling improves
partial results; it does not resolve endpoint bands or certify completeness.
General intersection/branch joining and full B-rep remain open.

### Fair curve/plane isolation across knot spans

A regression reproduced starvation of an exact endpoint in a later curve span
while four boxes were consumed refining earlier roots. Curve/plane isolation
now uses one FIFO queue across source spans and their subdivisions. Source-span
coefficients are computed lazily after the box-budget check. Existing root
sorting, endpoint admission, initial-span overlap detection and explicit
unresolved outcomes remain in place.

The later endpoint is now retained within the same four-box budget. All 18
native intersection tests pass in debug/release. Rebuilt WASM passes ten adapter
tests; vue-tsc --noEmit passes. The query remains numerically uncertified;
fair scheduling does not establish full root/branch completeness or B-rep
solid certification.

### Native/WASM NURBS curve versus finite segment

Added curve_segment and intersectNurbsCurveSegment, extending the finite CC
query matrix beyond plane targets. Two supporting plane queries share the box
budget. Points retain source curve intervals, segment parameters and geometric
line residuals. Positive-weight control hulls handle finite segment admission;
whole coincident intervals are retained, while partial coincidence and uncertain
boundary bands remain explicit unresolved regions. No polyline approximation
or topology-changing certificate is introduced.

Native tests cover transverse hits, skew/disjoint cases, finite exclusion,
whole/partial overlap, shared-budget exhaustion, degenerate segment rejection,
and a rational circle arc with reversed segment correspondence. All 20 native
intersection tests pass in debug/release; native workspace check passes.
After bridge integration and WASM rebuild, eleven adapter tests pass and
vue-tsc --noEmit passes. General curve/curve pairs, coincident clipping,
certified root correspondence and full curved Boolean remain incomplete.

### Degree-one rational coincidence clipping

Curve/segment queries now clip partial degree-one coincident intervals by
inverting rational parameterization with normalized endpoint weights. Both
segment directions preserve ascending source-curve parameter intervals;
endpoint-only contact is returned as a point. A finite overlap whose parameter
interval collapses numerically remains unresolved. Higher-degree partial
coincidence remains CoincidentTrim rather than being approximated.

For weights 1:3, the independent analytic fixture maps geometric [1/4,3/4]
to source parameters [1/10,1/2]. Tests cover reversal and endpoint-only contact.
All 21 native intersection tests pass in debug/release; rebuilt WASM passes 12
adapter tests and vue-tsc --noEmit passes. This extends the numerical finite
query matrix, not certified B-rep topology operations; general coincidence and
curved Boolean remain open.

### Shared-knot contact uniqueness after rational clipping

A failing regression returned two identical parameter events for a curve that
touches a segment endpoint at its internal knot. Clipped singleton contacts now
use the known segment boundary parameter and reject duplicate source-parameter
events. Deduplication does not compare spatial coordinates: a second regression
keeps two visits to the same point at distinct curve parameters 1/4 and 3/4.

All 22 native intersection tests pass in debug/release; the extended repeated-
visit regression also passes in both profiles. Rebuilt WASM passes 13 adapter
tests and vue-tsc --noEmit passes. This fixes query event identity within a
single curve, not persistent topology naming or certified contact semantics.
Full B-rep completion remains unproved.

### Higher-degree coincident clipping with explicit root bands

Partial curve/segment coincidences of higher degree now isolate roots against
both segment boundary planes under the existing shared box budget. Source
parameter cells are classified using positive-weight control hulls; admitted
interior cells retain their original curve intervals. Root uncertainty and
unresolved solver regions remain explicit bands, never collapsed to guessed
trim parameters. Exact numerical boundary events are retained when not already
covered by an overlap interval. Cell classification also consumes box budget.

The independent x(t)=t² fixture maps segment [1/4,9/16] to [1/2,3/4], including
reversed segment direction. Nondyadic roots and low budgets remain incomplete.
All 23 native intersection tests pass in debug/release; rebuilt WASM passes
14 adapter tests and vue-tsc --noEmit passes. These numerical query results
still cannot authorize B-rep topology edits. General certified coincidence,
curve/curve and surface/surface completeness remain open.

### Backtracking coincidence and authoritative trim endpoints

An independent x(t)=4t(1-t) regression exposed a false CoincidentTrim band for
one of two visits to the same segment: knot insertion rounded the trimmed
control endpoint outside the boundary. Cell hull classification now evaluates
its two endpoints on the authoritative source curve at the retained parameters,
while retaining interior trimmed controls. No tolerance snapping was added.

The two visits are retained separately as [1/8,1/4] and [3/4,7/8]. Nondyadic
fixtures independently verify all four analytic boundary roots are covered by
unresolved bands and accepted intervals remain inside the segment. All 24
native intersection tests pass in debug/release; rebuilt WASM passes 15 adapter
tests and vue-tsc --noEmit passes. These remain numerical checks, not certified
trim or solid completeness; the full B-rep goal remains open.

### Coplanar NURBS curve clipping to affine surface domains

Curve/surface queries now clip coplanar curves to finite affine rectangles by
isolating roots against all four boundary planes, using the frame's dual vectors
and a shared query budget. Retained source-parameter cells are classified by
positive-weight control hulls with authoritative endpoint evaluation. Curves
along rectangle edges remain admissible; unresolved root bands remain explicit.
Isolated admitted boundary contacts are retained when no overlap covers them.

The x=t,y=t² fixture is clipped to [1/4,3/4] with shifted/scaled surface knot
domains; edge coincidence and two-box refusal are also tested. All 25 native
intersection tests pass in debug/release. Rebuilt WASM passes 16 adapter tests;
vue-tsc --noEmit passes. This is finite affine-domain numerical clipping, not
general lifted UV arrangements, curved-face trimming or certified Boolean.

### Affine clipping shear and corner evidence

Additional native and WASM regressions verify the same source interval after
shearing both the curve and rectangle, and a single parameter/UV event when a
curve touches the common corner of two boundaries. This exercises the dual-frame
projection and boundary-event deduplication rather than only axis-aligned cuts.

All 26 native intersection tests pass in debug/release; 17 tests pass through
the existing WASM adapter, and vue-tsc --noEmit passes. Runtime implementation
and WASM bytes did not change in this checkpoint. The main matrix now also
records finite-segment and affine-domain coincidence clipping; general lifted
UV arrangements and certified B-rep completion remain open.

### Finite affine surface/surface query in Rust and WASM

Added surface_surface / intersectNurbsSurfaceSurface for two finite affine
rectangular patches. Transverse segments retain a trace on each source surface;
the same fraction evaluates corresponding UV/3D points. Finite clipping also
returns isolated contact points and disjoint results. Coplanar area overlap and
curved pairs remain explicitly unresolved. Pair unresolved boxes concatenate
both original UV domains. Correspondence residuals are sampled and numerical;
reports continue to forbid topology changes.

All 27 native intersection tests pass in debug/release, including operand swap
and shared budget. Rebuilt WASM passes 18 adapter tests, covering paired trace
JSON replay, finite endpoint contact, bounded disjointness, parallel separation,
coplanar refusal and unsupported curved pairs; vue-tsc --noEmit passes. General
surface intersection, coplanar areas and certified curved Boolean remain open.

### Coplanar affine surface overlap with paired UV boundaries

Surface/surface queries now clip a coplanar affine rectangle against the four
boundary halfspaces of the second patch. Area results retain an implicitly
closed convex polygon in both UV domains and source-evaluated 3D vertices;
edge and point contacts retain their lower-dimensional result types. Each
clipping pass consumes the shared box budget. Numerically degenerate area or
failed correspondence remains unresolved rather than becoming a solid face.

The square/diamond oracle produces eight vertices and area 3.5. Self-overlap,
shared edge, corner, disjointness and budget limits are covered. All 28 native
intersection tests pass in debug/release; rebuilt WASM passes 19 adapter tests
and vue-tsc --noEmit passes. Release/WASM compilation was slow but verified
live and completed without restart. General curved surface pairs, UV/DCEL
assembly and certified B-rep operations remain incomplete.

### Coplanar pair invariance and UV correspondence matrix

Eight combinations of operand swap, U reversal and UV transposition now verify
the same square/diamond intersection. Source knot domains are shifted/scaled
and one surface's weights are uniformly scaled by 32. Each case checks eight
vertices, physical area 3.5, containment in both analytic polygons, and equality
of each stored 3D vertex with evaluations in both returned UV systems. Boundary
ordering may follow the first UV orientation; geometry is not compared by array
order or guessed identity.

All 29 native intersection tests pass in debug/release; 20 tests pass through
the existing WASM adapter and vue-tsc --noEmit passes. Runtime code and WASM
bytes did not change. The main completion matrix records the finite affine SS
capability; general curved pairs and certified solid completion remain open.

### Algebraic section-trace conversion to NURBS curves

SurfaceTrace::to_curve and intersectionTraceToNurbsCurve now convert supported
traces to retained rational curves without fitting samples. Affine traces and
isoparametric lines use native curve extraction; single-Bezier ruled sections
form homogeneous Bernstein products of the two boundaries and plane equations.
The resulting degree is twice the curved direction's degree, bounded to 24;
a positive-weight representation and normal NURBS validation are required.
Unsupported multi-span/degree/weight representations fail explicitly.

Tests compare converted curves to retained procedural traces in U/V orientations
with unequal boundary weights, oblique/horizontal cuts and generators. Independent
cylinder/plane residual checks use the query's 1e-9 tolerance; generator roots
already carry numerical isolation error and are not claimed exact plane points.
All 30 native intersection tests pass in debug/release, rebuilt WASM passes
21 adapter tests, and vue-tsc --noEmit passes. Floating-point algebra is not a
certified construction, and general curved B-rep completeness remains open.

### Reversed section conversion and ambiguous-ruling regression

Conversion regressions now cover reversed trace fractions on shifted U/V knot
domains for both surface orientations and generator/isoparametric sections.
Converted NURBS points agree with the original trace at the reversed fraction
within 1e-10. A ruling family whose endpoint evaluations succeed but whose
homogeneous conversion has mixed-sign weights is explicitly rejected. This is
a conservative representation refusal, not a claim that endpoint sampling
certifies the interior.

The debug and release intersection suites each pass 32 tests; the existing WASM artifact passes
23 adapter tests, and vue-tsc --noEmit passes. These changes add regression tests
only and require no new runtime artifact. General curved intersections and
certified topology-changing operations remain incomplete.

### Multi-span ruled-trace conversion

Ruled trace conversion now decomposes the requested interval at source knots,
converts each Bezier span algebraically, and assembles one C0 rational curve
with the original parameter domain. Adjacent spans rescale their homogeneous
weights to share an endpoint weight. A shared endpoint is admitted only when
its Cartesian control coordinates are numerically identical; no proximity
welding is performed. Nonmatching endpoints, invalid weight conditioning and
results exceeding 256 control points are explicitly rejected. Reverse traversal
is applied to the assembled curve. This extends retained section geometry,
without certifying intersections or publishing new B-rep topology.

Validation: 33 native intersection tests pass in debug and release, including a valid
degree-12 surface whose converted representation exceeds the output budget.
The rebuilt WASM passes 24 adapter tests; TypeScript checking and packed-WASM
byte verification pass. Multi-span point comparisons include interior knot
locations and reversed traversal.

### Varying-weight multi-span regression

Native and WASM tests additionally cover independently varying weights on both
ruled boundaries with an oblique cutting plane, transposed surface orientation
and reversed traversal. Source-trace agreement and an independent plane residual
are checked to 1e-10, including the interior knot. The intersection suite passes
34 tests in release; the WASM adapter passes 25 tests and TypeScript checking
passes. This regression adds no runtime code and uses the previously rebuilt
artifact.

The full debug command `cargo test --offline --locked -p brep-core
-p brep-topology -p nurbs-core --manifest-path crates/Cargo.toml` completes with
150 passing tests and no failures. This covers current constructor, supported
Boolean, mass-property, identity, transaction and NURBS regressions, in addition
to the query tests; it does not prove the open completion-matrix requirements.

### Tensor-patch UV diagonal lifting

`SurfaceTrace::Line` now converts non-isoparametric UV segments on a single
tensor Bezier patch to a rational curve of degree p+q (maximum 25). The source
is trimmed to the segment's UV bounding rectangle; homogeneous Bernstein
products restrict its tensor polynomial to the diagonal, accounting for both
UV directions. Output parameter fraction is [0,1]. No sampled fitting is used.
Multi-span diagonals remain an explicit refusal; iso-parametric and affine
paths retain their existing behavior. A zero-length UV segment produces a
constant degree-one curve.

This operation lifts the supplied UV path. It does not establish that a
caller-supplied path lies in the trace's plane; evaluated plane residuals remain
separate numerical evidence. Sphere tests independently verify the radius,
source agreement, partial shifted domains, all four UV directions, degree limits
and multi-span refusal. General curved intersection and trim certification
remain open.

Validation: 35 native intersection tests pass in debug and release. The rebuilt WASM passes
26 adapter tests; TypeScript checking and packed-byte verification pass.
