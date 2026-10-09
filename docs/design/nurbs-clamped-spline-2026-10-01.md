# Кубический сплайн с касательными на концах: NURBS-0108

`clamped_spline_curve(points,parameters,start_tangent,end_tangent)` задаёт
2..86 пространственных точек и dP/dt на обоих концах. Внутренние касательные
вычисляются из системы кубических моментов для C2-сопряжения в вещественной
арифметике. Это отличается от натуральных условий (нулевая вторая
производная на концах) и от Hermite с независимой касательной в каждом узле.

Параметры t конечны, строго возрастают и безразмерны. Координаты и
касательные задаются в mm; выходной u нормирован на [0,1], а dP/du на
концах равна (t_last-t_first)*dP/dt. Нулевые касательные допускаются;
регулярность, отсутствие overshoot и самопересечений не сертифицируются.
Внутренние узлы кратности 3 требуют односторонних trimmed производных.
C2 не является сертификатом округлённого результата binary64.

Ограничения и отказы натурального сплайна сохраняются. Дополнительно
проверяется конечность входных касательных, переполнение и underflow
при нормировании касательных, representability endpoint-контролей.
Конечные заданные касательные копируются непосредственно в вход Hermite
для построения endpoint-контролей, без подмены округлённым решением моментов.

Rust: `natural_spline::clamped`; JSON: `curve_clamped_spline`;
TypeScript: `clampedSplineNurbsCurve`; Rush: `clamped_spline_curve`.
Пример: `examples/rush/clamped-spline-extrusion.r`.

Проверки Rust: `cargo test --manifest-path crates/nurbs-core/Cargo.toml --features transport natural_spline`:
7 тестов прошли, включая регрессии натурального сплайна. Независимый
кубический эталон проверяет позиции, первые и вторые производные на
неравномерных интервалах; двухточечный пример совпадает с Hermite.
Языковое ядро пересобрано; TypeScript проверен. Геометрический WASM
собирается нативным Binaryen 116. Все геометрические и экспортные
проверки должны быть повторены после завершения упаковки.
Статус после повторной проверки ниже — integrated; визуальная и STEP-квалификация не выполнены.

Полная проверка Rust общего решателя:
`cargo test --manifest-path crates/nurbs-core/Cargo.toml --features transport`:
256 тестов и 2 doctest прошли. TypeScript и Rush-тест также прошли;
в Rush-прогоне 2 геометрических теста были исключены фильтром до упаковки.

Сборка WASM полностью завершена, размер 9 774 226 байт.
`npx vitest run tests/nurbsClampedSpline.test.ts tests/nurbsNaturalSpline.test.ts tests/nurbsHermiteCurve.test.ts tests/nurbsPrimitiveGraph.test.ts tests/nurbsTranslationSweep.test.ts tests/geometryPackingPublication.test.ts tests/wasmArtifact.test.ts`:
201 тест в 7 файлах прошёл. Проверены все геометрические тесты без фильтра;
44 примера проходят JSON, OBJ, PLY и вход редактора. Это квалификация
отображаемой сетки, не непрерывной геометрической ошибки и не STEP.
NURBS-0108 переведён в integrated.
