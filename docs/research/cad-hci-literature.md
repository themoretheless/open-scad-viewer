# CAD/3D HCI и engineering UX для code-first OpenSCAD viewer

Дата обзора: 2026-07-14.

Изучены 24 первичных или официальных источника. Главный вывод для проекта: не превращать OpenSCAD viewer в упрощённую Plasticity, а добавить «двунаправленную инспекцию». Код остаётся единственным источником модели, а viewport помогает найти объект, измерить его, понять происхождение и подготовить безопасное изменение кода.

## Материалы и выводы

### Selection и preselection

1. [Grossman, Balakrishnan — The Bubble Cursor, CHI 2005](https://doi.org/10.1145/1054972.1055012)

   Динамическое расширение зоны указателя до ближайшей цели ускоряет выбор относительно точечного курсора. Для CAD переносится как screen-space tolerance и предварительная подсветка ближайшего кандидата, но без скрытого выбора объекта за передней поверхностью.

2. [AutoCAD — Cycling Through and Filtering Subobjects](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-89EFC58E-D14E-4B62-87D3-A6E26146D85E.htm)

   Официальный паттерн: предвыбор подсветкой, cycling перекрывающихся кандидатов и фильтры `object/face/edge/vertex`. Особенно полезен на плотной геометрии.

### Навигация и ViewCube

3. [Fitzmaurice et al. — Safe 3D Navigation, I3D 2008](https://doi.org/10.1145/1342250.1342252)

   Потерю ориентации уменьшают устойчивый pivot на модели, ограничение опасных перемещений, явное состояние ориентации и возможность Rewind. Тесты показали пользу и новичкам, и опытным пользователям.

4. [Khan et al. — ViewCube, I3D 2008](https://doi.org/10.1145/1342250.1342253)

   В эксперименте drag по ViewCube был почти вдвое быстрее и предпочтительнее click-based переключения. В работе также описаны 26 направлений, hover-feedback, snapping к стандартным видам и анимированные переходы.

5. [Hachet et al. — Navidget for 3D Interaction](https://doi.org/10.1016/j.ijhcs.2008.09.013)

   Навигация строится вокруг выбранной точки интереса с preview будущего ракурса и плавным переходом. Это лучше непредсказуемого orbit вокруг центра сцены.

### Поиск команд

6. [Matejka et al. — CommunityCommands, UIST 2009](https://doi.org/10.1145/1622176.1622214)

   В AutoCAD item-based рекомендации дали в 2,1 раза больше полезных новых команд, чем сравниваемые подходы. Рекомендации должны быть ненавязчивыми, dismissible и зависеть от контекста.

7. [Bota et al. — Characterizing Search Behavior in Productivity Software, CHIIR 2018](https://doi.org/10.1145/3176349.3176395)

   Анализ миллионов сессий примерно миллиона пользователей Office показал: основная цель встроенного поиска — исполнение команд, а повторный поиск ранее использованных команд встречается часто. Следствие: MRU и локальная история важнее алфавитного порядка.

### Direct manipulation и code-first CAD

8. [Shneiderman — Direct Manipulation, IEEE Computer 1983](https://doi.org/10.1109/MC.1983.1654471)

   Базовые свойства хорошего direct manipulation: постоянное представление объекта, быстрые инкрементальные действия, немедленный feedback и обратимость.

9. [Gonzalez et al. — Understanding the Challenges of OpenSCAD Users, CHI 2024](https://doi.org/10.1145/3613904.3642566)

   Интервью с 20 пользователями выявили конкретные проблемы: связь кода с видом, пространственные трансформации, измерение и проверка размеров, organic shapes, debugging и повторное использование моделей.

10. [Gonzalez et al. — Facilitating Parametric Definition in Programming-Based CAD, UIST 2024](https://doi.org/10.1145/3654777.3676417)

    Анализ 30 OpenSCAD-моделей показал, что геометрические свойства чаще всего выражены линейными комбинациями переменных. В исследовании с 11 пользователями извлечение выражения через выбор геометрии уменьшало ошибки и математическую нагрузку.

11. [Mathur et al. — Interactive Programming for Parametric CAD](https://doi.org/10.1111/cgf.14046)

    Система синтезирует устойчивые программные selection queries из выбора во viewport. GUI+programming значительно улучшили скорость и точность по сравнению с программированием без визуальной поддержки.

12. [Hempel et al. — Sketch-n-Sketch, UIST 2019](https://doi.org/10.1145/3332165.3347925)

    Прямое изменение результата может обновлять исходный текст, сохраняя его основным и всегда редактируемым представлением. Ключевая инфраструктура — provenance между кодом и результатом.

13. [Camba et al. — Parametric CAD Modeling and Design Reusability](https://doi.org/10.1016/j.cad.2016.01.003)

    Структура зависимостей определяет устойчивость модели к изменениям. Формальные стратегии превосходили неструктурированное построение; лишние parent-child связи делают модели хрупкими.

### Latency и progressive feedback

14. [Jota et al. — How Fast Is Fast Enough?, CHI 2013](https://doi.org/10.1145/2470654.2481317)

    В диапазоне 1–50 мс рост latency ухудшал прямое манипулирование; особенно страдала финальная точная фаза. Числа получены для touch, поэтому переносить их буквально на mouse/WebGPU нельзя, но визуальный feedback на hover/drag должен оставаться локальным и мгновенным.

15. [Stolper et al. — Progressive Visual Analytics, IEEE TVCG 2014](https://doi.org/10.1109/TVCG.2014.2346574)

    Промежуточный результат полезнее блокирующего ожидания, если система явно показывает его неполноту и позволяет отменить или перенаправить вычисление.

### Undo и история

16. [Berlage — Selective Undo Based on Command Objects, TOCHI 1994](https://doi.org/10.1145/196699.196721)

    Отдельную старую операцию можно отменять только когда её применение к текущему состоянию имеет осмысленную семантику. Для code-first продукта безопаснее source snapshots, чем попытка обратить произвольные CSG-операции.

17. [Autodesk Fusion — Parametric Timeline](https://help.autodesk.com/view/fusion360/ENU/?contextId=DESIGN_HISTORY)

    Timeline связывает шаги, параметры, recompute и просмотр промежуточных состояний. История модели при этом отделена от истории положения камеры.

### Measurements и section planes

18. [Autodesk Fusion — Analysis Tools](https://help.autodesk.com/cloudhelp/ENU/Fusion-Model/files/SLD-INSPECT-TOOLS.htm)

    Measure, section analysis, interference, center of mass и другие проверки являются сохраняемыми analysis-объектами, которые можно включать и скрывать независимо от модели.

19. [Li et al. — Interactive Cutaway Illustrations, SIGGRAPH 2007](https://doi.org/10.1145/1276377.1276416)

    Хороший разрез не просто отбрасывает половину модели: он сохраняет внешний контекст и помогает исследовать целевые внутренние структуры.

20. [ISO 16792:2021 — Digital Product Definition Data Practices](https://www.iso.org/standard/73871.html)

    Стандарт поддерживает 3D model-only product definition и подчёркивает необходимость однозначного представления размеров и аннотаций. Для OpenSCAD без заданных единиц корректная подпись по умолчанию — `model units`.

### Canvas accessibility

21. [WCAG 2.2](https://www.w3.org/TR/WCAG22/)

    Существенны keyboard operation, альтернатива drag-жестам, минимальная цель 24×24 CSS px, видимый focus и доступные status messages.

22. [WHATWG HTML — Canvas](https://html.spec.whatwg.org/multipage/canvas.html#the-canvas-element)

    Canvas должен иметь содержимое с эквивалентной функцией; для интерактивного canvas стандарт рекомендует взаимно-однозначное соответствие interactive regions и focusable fallback controls.

23. [Siu et al. — shapeCAD, ASSETS 2019](https://doi.org/10.1145/3308561.3353782)

    Co-design с тремя незрячими пользователями и проверка с пятью BVI-программистами показали ценность динамической невизуальной обратной связи рядом с OpenSCAD-кодом.

24. [Zhang et al. — A11yShape, ASSETS 2025](https://doi.org/10.1145/3663547.3746362)

    Наиболее прямой референс: semantic hierarchy, доступные описания, version history и синхронная подсветка между кодом, деревом, описанием и 3D-видом. Исследование с четырьмя участниками exploratory, поэтому результат перспективный, но не окончательный.

## Восемь переносимых функций

| Приоритет | Функция | MVP для нашего viewer | Сложность |
|---|---|---|---|
| P0 | Preselection и selection cycling | Hover-outline с screen-space tolerance; depth-sorted список кандидатов; переключение перекрывающихся объектов; сначала object-level | Средняя |
| P1 | Safe camera и полный ViewCube | Pivot по выбранной точке/объекту, drag cube, 26 snap-видов, плавные переходы, стек Previous View | Средняя |
| P1 | Контекстная command palette | RU/EN aliases, fuzzy search, MRU, shortcuts, фильтрация по состоянию и объяснение недоступных команд; история только локальная | Низкая–средняя |
| P0 | Code ↔ viewport provenance | Стабильный `nodeId` и source range из AST через Worker в mesh; выбор модели открывает код, курсор кода подсвечивает результат; затем preview предлагаемого source patch | Высокая |
| P0 | Preview → full-quality rendering | Быстрый результат с ограниченным `$fn`, затем финальный full-quality расчёт; явные `preview/stale/full`, отмена superseded job; экспорт разрешён только из full | Средняя–высокая |
| P1 | История успешных моделей | Snapshot исходника после успешного render, именованные checkpoints, diff и restore; отдельно хранить историю камеры | Средняя |
| P0 | Inspect workspace | Distance между picked points, bounding box, area/volume, координаты и section plane; единицы по умолчанию `model units`; clip plane не изменяет экспорт | Средняя–высокая |
| P0 | Semantic accessible viewport | DOM-дерево объектов, keyboard select/focus/isolate/measure, live-region для compile/error/stats, model summary, high contrast, reduced motion и кнопочные альтернативы drag | Высокая |

## Архитектурный порядок

Сначала стоит провести `nodeId + sourceRange` через parser → geometry worker protocol → renderer hit-test. Это основа сразу для provenance, semantic tree, доступности и осмысленной истории.

MVP должен связывать viewport с верхнеуровневыми AST-результатами. Нельзя обещать стабильный выбор B-Rep-граней по индексам треугольников: после boolean и ретриангуляции такая идентичность разрушается. Face-level provenance следует добавлять только при наличии устойчивой поддержки со стороны геометрического ядра.

Самая сильная следующая версия по соотношению пользы и риска: provenance/cross-highlight + measurement/section + двухступенчатый preview. Это напрямую закрывает проблемы реальных пользователей OpenSCAD, а не только копирует внешний вид Plasticity.

## Ограничения доказательной базы

- Результат ViewCube «почти вдвое быстрее» относится к конкретной экспериментальной задаче ориентации, а не ко всем CAD-сценариям.
- Исследование latency Jota et al. выполнено для direct touch; из него следует важность локального feedback, но не обязательный числовой бюджет для desktop mouse.
- OpenSCAD CHI 2024 — качественное исследование 20 пользователей; UIST 2024 — 11 пользователей. Они очень релевантны предметной области, но выборки ограничены.
- shapeCAD и A11yShape основаны на малых выборках BVI-пользователей. Их сильная сторона — participatory design и прямое соответствие code-first CAD, а не статистическая обобщаемость.
- ISO 16792 задаёт требования к product definition data, а не готовый UI-паттерн; его следует применять к единицам и аннотациям, а не копировать интерфейсно.
