# NURBS-0019: цепная линия

Статус integrated. catenary::approximate(center,scale,start_x,end_x,budget).
JSON curve_catenary с center/scale/start_x/end_x/max_deviation.
Центр — нижняя точка; плоскость XY, ось Z. Локальные x-границы возрастают,
scale положителен. P(x)=center+[x,scale*(cosh(x/scale)−1),0]. Все
координаты, scale, x и budget задаются длинами. Это заданная аналитическая
форма, без решения задачи провиса по длине цепи/натяжению/двум опорам.

Касательная по нормализованному t: [range,range*sinh(x/scale),0].
Высота вычисляется через 2*scale*sinh(x/(2*scale))², чтобы не вычитать
почти равные cosh и 1 около нижней точки. До 85 cubic Hermite spans,
256 контролей. Ideal remainder — scale*cosh(maxAbsX/scale)*
(range/(spans*scale))^4/384; координата X линейна. Binary64 sinh/cosh,
контролы и evaluation не сертифицированы: continuousBound:false,
roundingCertified:false. Method: uniform-abscissa-cubic-Hermite-fourth-derivative-estimate.
Переполнение/непредставимая оценка и недостижимый бюджет отклоняются.

Два native-теста прошли: независимая формула cosh в 1001 точке на
несимметричном диапазоне, крайние производные, малая ненулевая высота
5e-19 около нижней точки, отказы scale=0, переполнения cosh и бюджета
сверх 85 spans. WASM/TS/Rush/editor/export, visual/STEP и непрерывная
rounding-inclusive квалификация ещё ожидаются.

Добавлены TS-адаптер и graph/Rush catenary_curve, schema, контекстные
LENGTH-размерности scale/start_x/end_x, constructionReport с собственным
uniform-abscissa методом и пример catenary-extrusion.r. vue-tsc и cargo
check языковых crate прошли; language WASM собран. Geometry WASM ещё
собирается; geometry/editor/export проверки через пакет ожидаются.

Пакетная проверка завершена: 341 тест в 16 файлах прошёл. Проверены
независимая cosh-формула, малая ненулевая высота, vertex-relative endpoint,
размерности и обязательные поля Rush, собственный abscissa constructionReport
и отсутствие certified error. Программный вход редактора и JSON/OBJ/PLY
round-trip прошли для 70 фиксированных примеров: JSON сохраняет
definitions/hash, сетка — индексы и координаты с допуском 1e-5 mm.
Это не visual review или continuous error certificate. Полный Rust-прогон:
337 NURBS-тестов, 2 doctest и языковые тесты прошли. WASM: 9826333 bytes,
SHA256 `69fab50f6b6a9f75d51107c9cf3571abc859f25c6cdf946cbbcb85870115502c`.
Visual/STEP и rounding-inclusive error qualification остаются открытыми.
