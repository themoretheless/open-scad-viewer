# Третий исследовательский проход: ещё 10 идей с новым пользовательским результатом

Срез: **2026-07-18**. Этот проход ищет новые пользовательские результаты, а не
ещё десять названий для существующего backlog. Некоторые низкоуровневые
примитивы и отложенные Plasticity-кандидаты при этом осознанно развиваются до
более полного workflow. Перед отбором были повторно сверены:

- 15 пунктов [приоритетного backlog](second-pass-recommendations.md);
- реализованные и отложенные [паттерны Plasticity](plasticity-patterns.md);
- выводы обзоров [CAD/HCI](cad-hci-literature.md) и
  [geometry pipeline](geometry-pipeline-literature.md);
- переносимые практики обеих непересекающихся сотен репозиториев.

Идея считалась новой только тогда, когда у неё другой пользовательский
результат. Поэтому, например, dimensional lint не дублирует display units,
printability heatmap — topology diagnostics или SDF repair, Design Gallery —
обычные Customizer sliders, а assembly interference — проверку валидности
отдельной сетки. Общие примитивы вроде BVH, provenance и Worker можно и нужно
переиспользовать.

Две связи с прежним списком отмечены явно: printability-карта развивает
отложенный face-direction/draft shader до проверки overhang + build volume, а
матрица пересечений превращает отложенную interference coloring в bounded
проверку пар тел с объёмом, centroid, tolerance и ignore rules. Это не новые
GPU-примитивы, но новые законченные пользовательские результаты; исходные
кандидаты перечислены в [паттернах Plasticity](plasticity-patterns.md).

## Приоритетный top-4

1. **Исполняемые контракты модели** — дают проверяемый результат и CI-защиту
   при небольшом новом UI.
2. **Профилировщик пересборки** — показывает, где дальнейшая оптимизация
   действительно окупится.
3. **Матрица пересечений сборки** — находит конфликтующие пары тел и даёт
   измеримую геометрию пересечения.
4. **Карта пригодности к FFF-печати** — даёт сильную практическую пользу поверх
   готовых normals, BVH и GPU overlays.

Первые две функции остаются code-first; анализ пересечений использует viewport
как инспектор результата, не превращая его в отдельный источник геометрии.

## 1. Исполняемые контракты модели

- **Ценность:** автоматически обнаруживать поломку параметрической модели при
  изменении кода, параметров или версии kernel.
- **Ограниченный MVP:** поддержать нативный OpenSCAD `assert()` и три
  post-build контракта в metadata comments: диапазон объёма, число связных тел
  и допустимые bounds. Выполнять их только на актуальной full-сборке; показывать
  pass/fail и выгружать компактный JSON или JUnit report.
- **Риск:** площадь и объём зависят от tessellation и floating-point. Контракты
  на геометрию должны требовать явного допуска и фиксировать quality settings.
- **Первичная/официальная опора:** [OpenSCAD 2019.05: `assert()`](https://github.com/openscad/openscad/releases/tag/openscad-2019.05).

## 2. Линтер физических размерностей

- **Ценность:** до сборки находить сложение длины с углом, неправильное
  масштабирование площади и другие ошибки инженерных выражений.
- **Ограниченный MVP:** opt-in комментарии `// @unit mm`, `// @unit deg` у
  top-level параметров; AST-инференс для `+`, `-`, `*`, `/`, степеней и
  тригонометрии; только editor diagnostics, без изменения исполнения SCAD.
- **Риск:** OpenSCAD намеренно не задаёт физические единицы. Неполный inference
  создаёт false positives, поэтому неизвестная размерность не должна быть
  ошибкой, а annotations должны оставаться добровольными.
- **Первичная/официальная опора:** Andrew Kennedy,
  [Programming Languages and Dimensions](https://www.cl.cam.ac.uk/techreports/UCAM-CL-TR-391.html),
  и [официальный OpenSCAD User Manual](https://files.openscad.org/documentation/manual/OpenSCAD_User_Manual.pdf).

## 3. Карта пригодности к FFF-печати

- **Ценность:** находить нависания и выход за build volume до экспорта в
  slicer.
- **Ограниченный MVP:** регулируемая WGSL-подсветка overhang по face normal и
  направлению печати; AABB модели против одного пользовательского профиля
  принтера.
- **Риск:** face normal не учитывает bridges, материал, охлаждение и стратегию
  supports, а AABB даёт консервативную проверку габарита. Результат нужно
  называть предварительной проверкой, а не гарантией печати или заменой slicer.
- **Официальная опора:** PrusaSlicer —
  [Highlight overhang by angle](https://help.prusa3d.com/article/paint-on-supports_168584).

## 4. Профилировщик пересборки по SCAD-узлам

- **Ценность:** показать, какой boolean, цикл, высокий `$fn` или будущий
  `minkowski()` создаёт основную задержку и рост сетки.
- **Ограниченный MVP:** ставить `PerformanceMark`/`PerformanceMeasure` вокруг
  вычисляемых AST-узлов в Worker; возвращать inclusive duration, изменение
  triangle count и top-10 горячих source ranges; показывать таблицу и gutter
  heatmap только по явной команде Profile full build.
- **Риск:** вложенные времена нельзя суммировать, единичный прогон шумен, а
  instrumentation сам стоит времени. UI должен различать inclusive/exclusive
  оценки и показывать overhead.
- **Официальная опора:** [W3C User Timing](https://www.w3.org/TR/user-timing/),
  API высокоточных marks/measures доступен и в Worker.

## 5. Синхронный quad-view

- **Ценность:** одновременно видеть front/top/right/iso и замечать ошибки,
  которые скрывает один перспективный ракурс.
- **Ограниченный MVP:** один WebGPU canvas с четырьмя viewport/scissor regions
  и общими mesh buffers; общее selection/crosshair, отдельные камеры, явная
  рамка активного вида и переключение layouts `1/2/4`.
- **Риск:** до четырёх draw-проходов и усложнение pointer-to-ray mapping. На
  слабом adapter нужно разрешить только два вида или снижать MSAA.
- **Официальная опора:** Autodesk,
  [About Model Space Viewports](https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-Core/files/GUID-3E43911D-0A0F-4900-BE32-5EF846AF36D8.htm).

## 6. Анализ допусков и чувствительности параметров

- **Ценность:** проверять не только номинальную модель, но и вероятность выхода
  размера, зазора или пользовательского контракта за допустимый диапазон.
- **Ограниченный MVP:** annotation `diameter = 8; // @tol +/-0.05`; 32–128
  детерминированных preview-сэмплов максимум по трём параметрам; histogram
  выбранной метрики, failure rate и rank наиболее влиятельных параметров.
- **Риск:** серия пересборок дорога, а неверное распределение создаёт ложную
  уверенность. MVP должен показывать seed, sample count и assumptions и не
  заменять worst-case engineering analysis.
- **Первичные опоры:** NIST,
  [Tolerance Synthesis Scheme](https://doi.org/10.6028/NIST.IR.6836), и
  [statistical tolerance analysis with Monte Carlo](https://doi.org/10.1016/j.cad.2011.10.004).

## 7. Воспроизводимый manifest для экспорта

- **Ценность:** однозначно связать STL/OBJ или будущий 3MF с исходником,
  параметрами, зависимостями и версией geometry kernel.
- **Ограниченный MVP:** sidecar JSON с SHA-256 исходника и результата,
  Customizer values, quality/tessellation settings, версиями приложения и
  WASM, topology metrics и списком входных файлов с digest. Добавить локальную
  команду Verify manifest.
- **Риск:** абсолютные пути раскрывают локальные данные, а неподписанный JSON
  можно изменить. Пути нужно нормализовать/редактировать и не называть MVP
  криптографической attestation.
- **Официальная опора:** [SLSA Build Provenance 1.2](https://slsa.dev/spec/v1.2/build-provenance),
  откуда переносится разделение inputs, parameters, builder и output digests,
  без заявления о SLSA-conformance CAD-файла.

## 8. Матрица пересечений сборки

- **Ценность:** находить пары верхнеуровневых тел, которые занимают один объём,
  до экспорта или физической сборки.
- **Ограниченный MVP:** AABB broad phase, затем Manifold intersection в Worker
  максимум для 64 тел или явно выбранного subset; таблица `тело A × тело B`,
  объём и centroid пересечения, временная подсветка конфликтующей области.
- **Риск:** касание, реальное пересечение, намеренная посадка с натягом и
  численный шум требуют явного tolerance. Результаты нужно называть findings,
  разрешить threshold и ignore для ожидаемых пар, а не автоматически объявлять
  модель ошибочной.
- **Официальная опора:** Autodesk Inventor описывает
  [анализ пересечений компонентов](https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-DAAB6834-D9D1-4391-B476-AFE94D561C4A.htm), а официальный API-пример
  [возвращает пары, объём и centroid](https://help.autodesk.com/cloudhelp/2024/ENU/Inventor-API/files/AssemblyComponentDefinition_AnalyzeInterference_Sample.htm).

## 9. Автоматический минимизатор ошибочного SCAD

- **Ценность:** превращать большой падающий проект в короткий воспроизводимый
  пример для пользователя, теста или bug report.
- **Ограниченный MVP:** AST-aware `ddmin`, удаляющий top-level statements и
  неиспользуемые module definitions; каждый кандидат исполняется в новом
  Worker и должен сохранить нормализованную error signature. Ограничить поиск
  100 запусками и экспортировать только новый `repro.scad`.
- **Риск:** timeout, memory failure и недетерминированные ошибки делают
  предикат нестабильным. Нужны повторная проверка финального repro и отдельный
  класс `unresolved`.
- **Первичная опора:** Zeller и Hildebrandt,
  [Simplifying and Isolating Failure-Inducing Input](https://www.st.cs.uni-saarland.de/papers/tse2002/).

## 10. Design Gallery для диапазонов параметров

- **Ценность:** показать взаимодействие параметров и несколько существенно
  разных вариантов быстрее, чем последовательное движение sliders.
- **Ограниченный MVP:** пользователь выбирает два-три уже распознанных
  Customizer parameters; Worker генерирует 12–20 ограниченных preview-builds;
  UI показывает thumbnails с volume/bounds, а клик применяет ровно один набор
  значений в исходник и запускает обычный full-build.
- **Риск:** пространство комбинаций растёт экспоненциально, а thumbnails могут
  отличаться только tessellation noise. Нужны bounded sampling, отмена и
  группировка по простой screen-space/metric distance.
- **Первичная опора:** Marks et al., SIGGRAPH 1997,
  [Design Galleries: A General Approach to Setting Parameters](https://dash.harvard.edu/entities/publication/73120378-7f12-6bd4-e053-0100007fdf3b).

## Рекомендуемый порядок проверки гипотез

Сначала реализовать contracts и profiling как два независимых вертикальных
slice: оба используют текущий Worker protocol и дают измеримый результат.
Затем добавить assembly interference с жёстким лимитом тел и явным tolerance.
Printability стоит начинать только с overhang и build-volume checks. Остальные
шесть идей следует поднимать по пользовательскому спросу, не смешивая их в один
большой релиз.
