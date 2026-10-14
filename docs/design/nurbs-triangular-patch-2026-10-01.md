# Трёхграничный рациональный патч: NURBS-0058

Статус: integrated. Визуальная и STEP-квалификация не завершены.

`triangular_patch::patch(base,side_a,side_b)` принимает непериодические
рациональные 3D-кривые с ориентацией A→B, A→C, B→C. Концы кривых должны
точно совпадать. Активные домены обрезаются до clamped-представления.
Конструктор строит четвёртую границу C→C с весами, согласованными с двумя
сторонами, затем создаёт однородный Coons-патч.

В вещественной арифметике сохраняются все три исходные границы. Домены
результата нормированы [0,1]²; V=1 схлопывается в вершину C. Это намеренная
параметрическая сингулярность: нормаль в вершине не определена. Остальная
регулярность, инъективность, отсутствие самопересечений и solid-топология
не сертифицированы. Возможны вырождения и overshoot.

Согласование степеней/узлов и положительность однородных контрольных весов
наследуются от Coons; бюджет поверхности — 32 контроля по каждой оси.
Недопустимые веса или несовместимые binary64-угловые условия приводят к
отказу. Конструктор не snapping-ит границы и не исправляет их ориентацию.
Binary64-погрешность сохранения всей границы не сертифицирована.

JSON: `surface_triangular_patch` с полями `base`, `side_a`, `side_b`.
TypeScript: `triangularNurbsPatch`; Rush: `triangular_patch(base,left,right)`.
Пример: `examples/rush/triangular-patch.r`.

`cargo test --manifest-path crates/nurbs-core/Cargo.toml --features transport triangular_patch`
— 5 тестов прошли. Проверены независимый affine-треугольник, рациональный
сектор конуса, сохранение кривых сторон, разновесные стороны и согласование
схлопнутой границы, намеренно неопределённая нормаль вершины, JSON и отказ
при несовпадающих концах.

Полный native regression:
`cargo test --manifest-path crates/nurbs-core/Cargo.toml --features transport`
— 294 теста и 2 doctest прошли. Обе WASM-сборки завершены; пакет
9 797 522 байта, SHA256
`f0594b2a454ec58574a8753952da4d364dbd3fa678cac9f48f4f25ddb83cc002`.
`npx vue-tsc --noEmit` прошёл.
`npx vitest run tests/nurbsTriangularPatch.test.ts tests/nurbsGordon.test.ts tests/nurbsClosedLoft.test.ts tests/nurbsClampedLoft.test.ts tests/nurbsNaturalLoft.test.ts tests/nurbsPrimitiveGraph.test.ts tests/nurbsGridSpline.test.ts tests/nurbsNaturalSpline.test.ts tests/nurbsClampedSpline.test.ts tests/nurbsClosedSpline.test.ts tests/wasmArtifact.test.ts tests/geometryPackingPublication.test.ts`
— 249 тестов в 12 файлах прошли. Проверены affine-треугольник, веса сторон,
схлопнутая вершина и неопределённая нормаль, независимое уравнение конуса,
отказы несовпадающих концов и числа Rush-границ. 52 фиксированных примера
проходят JSON/OBJ/PLY/editor regression. Допуск display mesh 1e-5 mm
не сертифицирует непрерывную поверхность.
