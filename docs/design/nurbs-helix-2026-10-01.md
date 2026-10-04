# Цилиндрическая винтовая линия — NURBS-0021

Статус: integrated. Rust/JSON/WASM/TypeScript/Rush, программный вход редактора
и JSON/OBJ/PLY проверены. Полная оценка ошибки с округлением, визуальная
и STEP-квалификация ещё ожидаются.

`helix::approximate(center,radius,height,turns,phase_degrees,max_deviation)`
аппроксимирует P(t)=center+[r*cos(phase+2π*turns*t),r*sin(...),height*t],
t∈[0,1]. Ось фиксирована вдоль Z. Center/radius/height/budget используют
длину модели; turns безразмерен и может быть отрицательным, phase в градусах.
Радиус и бюджет строго положительны, turns ненулевой. Высота может быть
отрицательной или нулевой. Phase нормируется modulo360.

Кубическая Hermite-кривая использует аналитические позиции и касательные
в равномерных угловых станциях, единичные веса и C0-кратности узлов.
В вещественной арифметике касательные соседних кубиков совпадают, но
согласованность binary64-jet не сертифицирована. Число spans от1до85
(до256контролей) выбирается по оценке E=√2*r*(|2π*turns|/spans)^4/384.
Для каждой координаты XY четвёртая производная ограничена r*ω^4;
скалярный Hermite remainder ≤M*h^4/384, √2 учитывает обе координаты.
Z линеен, его интерполяционный остаток в вещественной арифметике равен нулю.

Это оценка ошибки идеальной вещественной интерполяции, вычисленная binary64.
Она не включает округление sin/cos, станций, контролей и вычисления кривой.
Report содержит realArithmeticErrorEstimate, budget, spans,
continuousBound:false и roundingCertified:false. Требования больше85spans
и непредставимые оценки отклоняются. Точная рациональная helix, сертификация
общей ошибки с округлением, произвольная ось, variable pitch/radius,
самопересечения и solid-топология не входят в текущий контракт.

JSON: `curve_helix` с теми же snake_case полями; возвращает curve и report.
Тесты сравнивают1001точку с независимой аналитической формулой для1и−2.5
оборотов, проверяют влияние допуска, отказы и JSON-роли полей. Выборочная
проверка не заменяет непрерывный binary64-сертификат.

TypeScript-адаптер `approximateHelixNurbsCurve` возвращает curve и типизированный
report целиком, включая continuousBound:false и roundingCertified:false.
`vue-tsc --noEmit` прошёл. Геометрический WASM ещё не упакован с этой
операцией; вызов адаптера через пакет, Rush/editor и экспорт ожидают проверки.

Добавлены graph-schema и исполнитель `helix_curve`, сохраняющий report в
constructionReports. Наличие этого отчёта запрещает общий флаг
error_bound_certified:true даже при foundation-сертификате самой NURBS-кривой.
В Rush обязательны center, radius, height, turns, phase_degrees, max_deviation;
phase проходит angular-проверку runtime, turns безразмерен, остальные поля
используют длину. Пример `examples/rush/helix-extrusion.r` содержит четверть
оборота с budget0.0001mm и небольшую экструзию; такой размер укладывается
в32-control бюджет поверхности. Более плотная кривая может потребовать
разбиения на несколько поверхностей: существующая единичная экструзия
не снимает этот бюджет. TypeScript и native frontend test прошли; runtime/
languages-bridge прошли cargo check. Упакованные WASM и экспорт ещё ожидаются.

Языковой WASM пересобран. `npx vitest run tests/nurbsHelix.test.ts
-t 'checks dimensions'` прошёл: размерные turns, длина вместо phase и угол
в max_deviation отклоняются. Геометрические тесты в этом запуске skipped.
Подготовлены сквозные проверки аналитической helix, влияния бюджета и
constructionReport с несертфицированными флагами. Пример включён в общую
JSON/OBJ/PLY/editor suite из58фиксированных примеров. Собственная сборка
геометрического WASM с native wasm-opt ещё выполняется; её результат и
геометрия через TS/editor/export пока не подтверждены. TypeScript прошёл.

Собственная геометрическая WASM-сборка завершилась:9798134bytes.
Полный `cargo test --manifest-path crates/Cargo.toml -p nurbs-core
-p modelgraph-runtime -p modelgraph-text --quiet` прошёл:318тестов nurbs-core
и2doctest, а также frontend/runtime.
`npx vitest run tests/nurbsHelix.test.ts tests/nurbsTwistSweep.test.ts
tests/nurbsTwoGuideSweep.test.ts tests/nurbsScaledSweep.test.ts
tests/nurbsScaledSweepLanguage.test.ts tests/nurbsPrimitiveGraph.test.ts
tests/wasmArtifact.test.ts` —259тестов прошли. Все58примеров прошли JSON/OBJ/PLY
round-trip и программный вход редактора. Curve-only regression подтверждает:
foundation_certificate присутствует, но error_bound_certified:false из-за
аппроксимации исходной helix. Source report сохранён и после экструзии.
Выборочные ошибки и ideal remainder estimate не заменяют общий непрерывный
сертификат с округлением. Визуальный viewport и STEP этим не проверены.
