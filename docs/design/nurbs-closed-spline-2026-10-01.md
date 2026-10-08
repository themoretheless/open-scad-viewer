# Замкнутая кубическая интерполяция: NURBS-0109

`closed_spline_curve(points,parameters)` интерполирует 4..86 пространственных
точек, включая точно повторённую конечную точку. Параметры конечны и строго
возрастают. Циклическая система кубических моментов согласует первые и
вторые производные на внутренних стыках и шве в вещественной арифметике.
Выходной параметр нормирован на [0,1]. Представление имеет clamped knots,
`periodic:false`: это замкнутая геометрия, а не периодическая кодировка узлов.

Решатель ограничен 85 неизвестными. Точки шва не подгоняются по допуску.
Binary64 continuity не сертифицируется; регулярность, overshoot,
самопересечения и solid-топология не проверяются. Вычисленные касательные
могут содержать cancellation roundoff около нуля. При материализации
контролей такая погрешность может округлиться; это не допуск на авторские
касательные Hermite, чей строгий precision guard сохранён. Для производных
на шве и внутренних узлах используются односторонние trimmed сегменты.

Rust: `closed_spline::interpolate`; JSON: `curve_closed_spline`;
TypeScript: `closedSplineNurbsCurve`; Rush: `closed_spline_curve`.
Пример: `examples/rush/closed-spline-extrusion.r`.

`cargo test --manifest-path crates/nurbs-core/Cargo.toml --features transport closed_spline`:
3 теста прошли. Симметричный пример сравнивается с независимыми кубическими
полиномами; неравномерный пространственный контур проверяет все стыки,
включая производные на шве. Проверены точная closure, отказ от snapping,
параметры и бюджет 86/87. Полная проверка, WASM, экспорт и визуальный
просмотр ещё не завершены. Статус native.

Полная проверка Rust:
`cargo test --manifest-path crates/nurbs-core/Cargo.toml --features transport`:
261 тест и 2 doctest прошли, включая строгие precision-отказы авторского
Hermite. TypeScript и Rush-прогон прошли; в Rush-прогоне 2 геометрических
теста исключены фильтром до новой упаковки. Геометрический WASM ещё
оптимизируется; общие проверки 45 примеров пока не запускались на нём.

Упаковка WASM завершена, размер 9 780 988 байт.
`npx vitest run tests/nurbsClosedSpline.test.ts tests/nurbsClampedSpline.test.ts tests/nurbsNaturalSpline.test.ts tests/nurbsHermiteCurve.test.ts tests/nurbsPrimitiveGraph.test.ts tests/nurbsTranslationSweep.test.ts tests/geometryPackingPublication.test.ts tests/wasmArtifact.test.ts`:
208 тестов в 8 файлах прошли. 45 примеров проверены через JSON, OBJ, PLY
и вход редактора; геометрические тесты шва выполнены без фильтра.
NURBS-0109 переведён в integrated. Ранние записи об ожидании сборки
описывают предшествующее состояние. Визуальная и STEP-квалификация
остаются незавершёнными.
