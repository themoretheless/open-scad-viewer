# Натуральная бикубическая интерполяция сетки: NURBS-0111

Текущий статус: integrated. Визуальная и STEP-квалификация не завершены.

`grid_spline_surface(points,parameters_u,parameters_v)` интерполирует
прямоугольную сетку 2..11 точек по каждой оси. Сетка индексируется [u][v];
координаты в mm, параметры безразмерны, конечны и строго возрастают.
Натуральные кубические сплайны сначала строятся по U, затем по V для
каждого промежуточного контроля. Результат имеет степень 3×3 и единичные
веса, максимум 31×31 контроль, нормированные [0,1]² домены.

В вещественной арифметике поверхность интерполирует исходные sites и
имеет C2 по каждой оси с натуральными условиями второй производной на
краях. Binary64-ошибка не сертифицируется; ограничения параметров и
представимости контролей наследуются от натуральной кривой. Внутренние узлы имеют
кратность 3; для производных на них нужны односторонние trimmed patches.
Вырождения, overshoot и самопересечения не исключены. Это не интерполяция
неупорядоченного облака и не guided loft.

Rust: `grid_spline::interpolate`; JSON: `surface_grid_spline`;
TypeScript: `gridSplineNurbsSurface`; Rush: `grid_spline_surface`.
Пример: `examples/rush/grid-spline-surface.r`.

`cargo test --manifest-path crates/nurbs-core/Cargo.toml --features transport grid_spline`:
3 теста прошли. Проверены неравномерный билинейный эталон и производные,
исходные точки, независимое произведение двух натуральных кубических
функций, rectangular/count/parameter отказы и бюджет 11×11.
WASM, полный regression suite, export и visual review ещё ожидают проверки.
Статус native.

Полный Rust-прогон:
`cargo test --manifest-path crates/nurbs-core/Cargo.toml --features transport`:
266 тестов и 2 doctest прошли. TypeScript и отдельный Rush-тест прошли;
2 геометрических теста в Rush-прогоне исключены фильтром до новой упаковки.

Первый полный WASM-прогон: 218 прошли, 4 проверки примера панели отказали
из-за вычисленной почти нулевой касательной, ошибочно проверенной как
авторская. Натуральный решатель теперь материализует inferred tangents
через отдельный внутренний путь Hermite. Для clamped-схемы авторские
концевые касательные проверяются на действительных соседних интервалах,
а не на полном параметрическом span. Прямой Hermite остаётся строгим.
Добавлен native regression именно для примера панели; новая сборка запущена.
Это не сертификат ошибки вычисленных производных.

Повторная упаковка завершена: WASM 9 785 032 байта.
`npx vitest run tests/nurbsGridSpline.test.ts tests/nurbsHermitePatch.test.ts tests/nurbsClosedSpline.test.ts tests/nurbsClampedSpline.test.ts tests/nurbsNaturalSpline.test.ts tests/nurbsHermiteCurve.test.ts tests/nurbsPrimitiveGraph.test.ts tests/nurbsTranslationSweep.test.ts tests/geometryPackingPublication.test.ts tests/wasmArtifact.test.ts`:
223 теста в 10 файлах прошли. Выполнен независимый нелинейный тензорный
эталон через WASM; ранее отказавший пример панели проходит все четыре
проверки JSON/OBJ/PLY/editor. Проверены 47 примеров. Clamped-регрессия
проверяет specific precision refusal на действительном коротком первом
интервале. Native-прогон после исправления: 267 тестов и 2 doctest прошли;
TypeScript также проверен. NURBS-0111 повышен до integrated.
Ранние записи об ожидании сборки и отказах описывают предыдущие состояния.
