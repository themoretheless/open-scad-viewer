# Rational conic twist sweep — NURBS-0064

Статус: integrated. Rust/JSON/WASM/TypeScript/Rush, программный вход редактора
и JSON/OBJ/PLY проверены. Визуальная и STEP-квалификация ещё ожидаются.

`twist_sweep::sweep(profile,path,origin,axis,start_degrees,sweep_degrees)`
строит S(u,v)=origin+C(v)-C(0)+R_axis(v)*(P(u)-origin).
Origin — центр вращения профиля в единицах модели; axis — ненулевой
конечный вектор направления. Его нормализация масштабируется для защиты
от overflow. Профиль и путь трёхмерны, профиль — до32контролей. Путь
непериодичен и trimmed до активного домена, независимо нормированного
на [0,1]; U-домен и periodic-флаг профиля сохраняются.

Закон вращения образован circle_arc единичного радиуса в плоскости XY,
с заданными начальным углом и sweep в градусах. Косинус/синус используются
в формуле Rodrigues. Параметр v соответствует рациональной conic-дуге:
угол не линеен по v и не равномерен по длине пути. При sweep=0 создаётся
постоянный рациональный закон вращения, а не геометрический отрезок нулевой
длины. Поддерживаемые ненулевые углы наследуют ограничения circle_arc.

На объединении узлов пути и conic-закона Bernstein-произведение строит
рациональную поверхность алгебраически без sampled fit. Degree V — сумма
степеней пути и закона вращения, до25; до32контролей на ось. Внутренние
узлы кодируют C0; общие концы spans должны совпадать в binary64.
Положительные представимые веса и конечные контроли проверяются;
binary64-округление не сертифицировано. Направление оси фиксировано;
автоматическое RMF, перпендикулярность профиля пути, линейный angular twist,
arc-length twist, регулярность, отсутствие самопересечений и solid-топология
не входят в этот контракт. Полный оборот вращения не замыкает открытый путь.

JSON: `surface_twist_sweep`, profile, path, origin, axis, start_degrees,
sweep_degrees. Три целевых native-теста прошли: независимое рациональное
решение quarter-turn вместе с переносом; другая ось, явный центр и далёкое
начало пути; рациональные веса профиля при полном обороте и нулевом twist.

Добавлены `twistSweepNurbsCurve`, схема/исполнитель графа и Rush lowering.
Вызов: `twist_sweep(profile,path,origin:[...],axis:[...],start_degrees:...,
sweep_degrees:...)`. Origin использует длину, axis безразмерен, углы
проходят angle-проверку runtime. Пример: `examples/rush/twist-sweep.r`.
`vue-tsc --noEmit`, native frontend test примера и `cargo check` runtime/
languages-bridge прошли. Сборка и проверки упакованного WASM ещё ожидаются;
геометрия через TS, единицы через Rush-пакет, редактор и экспорт не подтверждены.

Языковой WASM пересобран. `npx vitest run tests/nurbsTwistSweep.test.ts
-t 'enforces length'` прошёл: mm вместо angular sweep, размерная ось и deg
в origin отклоняются. Геометрические тесты пропущены в этом языковом запуске.
Подготовлены независимая quarter-turn формула, другая ось/центр/далёкий путь,
полный оборот с рациональными весами профиля и нулевой sweep при начальном
угле45градусов. Пример включён в общую JSON/OBJ/PLY/editor suite из57примеров.
Новая геометрическая сборка текущего репозитория выполняется с native wasm-opt;
сквозная геометрия и экспорт после неё пока ожидаются. TypeScript проверка прошла.

Сборка текущего дерева завершилась: геометрический WASM9800228bytes.
Полный `cargo test --manifest-path crates/Cargo.toml -p nurbs-core
-p modelgraph-runtime -p modelgraph-text --quiet` прошёл:315тестов nurbs-core
и2doctest, а также проверки языка/runtime.
`npx vitest run tests/nurbsTwistSweep.test.ts tests/nurbsTwoGuideSweep.test.ts
tests/nurbsScaledSweep.test.ts tests/nurbsScaledSweepLanguage.test.ts
tests/nurbsPrimitiveGraph.test.ts tests/wasmArtifact.test.ts` —252теста прошли.
Проверены первые, смешанные и вторые производные quarter-turn рациональных
функций, постоянная ориентация45градусов и полный оборот. Все57фиксированных
примеров прошли JSON/OBJ/PLY round-trip и программное построение через
редактор. Это не визуальный просмотр viewport и не STEP-квалификация.
