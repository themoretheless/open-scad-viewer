# Кубический loft с заданными концевыми производными: NURBS-0113

Статус: integrated. Rust, JSON, WASM, TypeScript и Rush подключены;
пример и export regression проверены. Визуальная/STEP-квалификация остаются.

`natural_loft::clamped(sections,parameters,start_tangent,end_tangent)`
интерполирует 2..11 рациональных 3D-сечений. Согласование степеней,
домена и узлов наследуется от `loft_aligned`. Параметры конечны,
строго возрастают и безразмерны. Выход имеет нормированные [0,1] домены,
степень 3 по V, максимум 32 U-контроля и 31 V-контроль.

Каждый концевой tangent — постоянный Cartesian-вектор dP/dt по всему
соответствующему профилю, в единицах длины модели. По нормированному V
производная равна tangent × (parameters[last]-parameters[first]).
Это управление общей производной профиля, а не произвольное векторное
поле по U и не автоматическое G1/G2-сопряжение с соседней поверхностью.

В однородных координатах задаются W'=0 и (XW,YW,ZW)'=W × tangent.
Clamped cubic interpolation решает условия для каждого контроля профиля.
Общие масштабы весов отдельных сечений влияют на промежуточную поверхность.
Все интерполированные контрольные веса должны оставаться положительными;
иначе конструктор отказывает. Граничные производные используют строгие
проверки представимости clamped spline на реальных соседних интервалах.

Реальная арифметика даёт интерполяцию сечений, концевые условия и C2
внутри V. Binary64-ошибка и регулярность не сертифицированы. Внутренние
узлы имеют кратность 3; производные на стыках требуют односторонних trim.
Вырождения, overshoot и самопересечения возможны; замыкание V и solid
топология не обеспечиваются.

JSON: `surface_clamped_loft`, поля `curves`, `parameters`,
`start_tangent`, `end_tangent`.

Native tests проверяют масштаб концевых производных на неравномерных
станциях и разных весах, сохранение среднего сечения, независимый
двухсекционный Hermite-эталон и JSON-границу. Общий native suite также
проверяет предыдущий natural loft после выделения общей реализации.

`cargo test --manifest-path crates/nurbs-core/Cargo.toml --features transport`
— 277 тестов и 2 doctest прошли.

TypeScript: `clampedLoftNurbsCurves`. Rush: `clamped_loft_surface`
с позиционными ссылками на сечения и именованными `parameters`,
`start_tangent`, `end_tangent`. Касательные используют единицы длины,
параметры — безразмерные величины. Пример:
`examples/rush/clamped-loft-surface.r`.

Языковая и геометрическая упаковки WASM завершены; геометрический WASM
9 778 495 байт. `npx vue-tsc --noEmit` прошёл.
`npx vitest run tests/nurbsClampedLoft.test.ts tests/nurbsNaturalLoft.test.ts tests/nurbsPrimitiveGraph.test.ts tests/nurbsGridSpline.test.ts tests/nurbsNaturalSpline.test.ts tests/nurbsClampedSpline.test.ts tests/nurbsClosedSpline.test.ts tests/wasmArtifact.test.ts tests/geometryPackingPublication.test.ts`
— 227 тестов в 9 файлах прошли. Проверены независимый Hermite-эталон,
концевые производные при разных весах, неравномерные станции, ошибки
единиц и отказ отрицательных интерполированных контрольных весов.
49 фиксированных Rush-примеров проходят JSON/OBJ/PLY/editor regression.
Допуск 1e-5 mm относится к display mesh, не к непрерывной поверхности.
