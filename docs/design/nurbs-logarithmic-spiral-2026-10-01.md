# NURBS-0013: логарифмическая спираль

Статус integrated. logarithmic_spiral::approximate(center,radius,growth,start,end,budget),
JSON curve_logarithmic_spiral: center/radius/growth/start_radians/end_radians/max_deviation.
XY, ось Z. Углы возрастают, в радианах. Радиус положителен при начальном
угле: r(θ)=radius*exp(growth*(θ−start)). growth — коэффициент на радиан,
может быть отрицательным; ноль даёт круговую предельную форму.

Кубический Hermite, до 85 spans/256 контролей. При нормализованном t
ω=end−start, k=growth*ω. Норма четвёртой производной равна
r(t)*(k²+ω²)². Conservative ideal coordinate remainder:
sqrt(2)*maxRadius*(hypot(k,ω)/spans)^4/384. Касательные аналитические.
Тригонометрия, exp, контролы и evaluation binary64 не сертифицированы:
continuousBound:false, roundingCertified:false. Переполнение/underflow
конечного или промежуточного радиуса и недостижимый бюджет отклоняются.

Два native-теста прошли: независимые формулы в 1001 точке для роста,
сжатия и кругового предела; отказы при переполнении, underflow, нулевом
радиусе и недостижимом бюджете. WASM/TS/Rush/editor/export, visual/STEP
и непрерывная rounding-inclusive оценка ещё ожидаются.

Добавлены TS-адаптер, graph/Rush logarithmic_spiral_curve, строгая схема,
сохранение constructionReport и пример logarithmic-spiral-extrusion.r.
Углы интерфейса в градусах переводятся в радианы; growth остаётся
безразмерным коэффициентом на радиан. vue-tsc и cargo check языковых
crate прошли; language WASM собран. Geometry WASM ещё собирается,
пакетные geometry/editor/export проверки ожидаются.

Пакетная проверка завершена: 294 теста в 11 файлах прошли. Проверены
независимые формулы роста/сжатия/кругового предела, конечная точка graph
с коэффициентом роста на радиан, размерности Rush, обязательные поля и
сохранение несертифицированного отчёта. Программный вход редактора и
JSON/OBJ/PLY round-trip прошли для 63 фиксированных Rush-примеров;
JSON сохраняет definitions/hash, сетка — индексы и координаты с допуском
1e-5 mm. Это не визуальная проверка и не continuous error certificate.
Полный Rust-прогон: 327 NURBS-тестов, 2 doctest и языковые тесты прошли.
WASM: 9812811 bytes, SHA256
`878ee697e8d35ac8633bd359e34d87475cfb4bb6bda16bf16c289babef7266a1`.
Visual/STEP и rounding-inclusive error qualification остаются открытыми.
