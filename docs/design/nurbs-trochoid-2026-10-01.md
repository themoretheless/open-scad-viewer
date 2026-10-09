# NURBS-0015 / NURBS-0018: циклоида и трохоида

Статус integrated. Общий trochoid::approximate(center,rolling_radius,tracing_radius,
start,end,budget); approximate_cycloid задаёт tracing_radius=rolling_radius.
JSON curve_trochoid: center/rolling_radius/tracing_radius/start_radians/
end_radians/max_deviation; curve_cycloid использует radius вместо двух радиусов.

XY, ось Z. Углы конечны и возрастают, в радианах. Радиус качения положителен,
радиус трассирующей точки неотрицателен. P(θ)=center+[rθ−d*sinθ,r−d*cosθ,0].
Касательная по нормализованному t: (end−start)*[r−d*cosθ,d*sinθ,0].
d<r — укороченная форма, d=r — циклоида с остриём при θ=0, d>r — петли.
d=0 — линейный предельный случай, один кубический span, ideal remainder=0.
Даже этот случай не объявляет binary64 rounding certification.

До 85 кубических Hermite spans/256 контролей. Линейный дрейф не вносит
четвёртой производной, поэтому conservative ideal remainder равен
sqrt(2)*d*((end−start)/spans)^4/384. Binary64 sin/cos, контролы и evaluation
не включены: continuousBound:false, roundingCertified:false. Недостижимый
бюджет/переполнение отклоняются. Самопересечения, regularity в остриях,
профиль шестерни или solid topology не сертифицированы.

Два native-теста прошли: независимая формула в 1001 точке для четырёх
значений d, касательная и положение в острие циклоиды, крайние производные,
отказы отрицательного tracing radius и недостижимого бюджета.
WASM/TS/Rush/editor/export, visual/STEP и rounding-inclusive error ожидаются.
Два существующих каталожных ID сохранены; варианты радиусов не добавляют ID.

Добавлены TS-адаптеры и graph/Rush trochoid_curve/cycloid_curve, schema,
runtime LENGTH для rolling_radius/tracing_radius и ANGLE для границ,
constructionReport и два примера extrusion. vue-tsc и cargo check языковых
crate прошли; language WASM собран. Geometry WASM собирается, программный
editor entrypoint и export geometry через новый пакет ещё ожидаются.

Пакетная проверка завершена: 314 тестов в 13 файлах прошли. Проверены
независимые формулы четырёх форм и острие циклоиды, перевод градусов,
размерности и обязательные поля Rush, сохранение constructionReport
и отсутствие certified error. Программный вход редактора и JSON/OBJ/PLY
round-trip прошли для 66 фиксированных Rush-примеров: JSON сохраняет
definitions/hash, сетка — индексы и координаты с допуском 1e-5 mm.
Это не visual review или continuous surface error certificate.
Полный Rust-прогон: 331 NURBS-тест, 2 doctest и языковые тесты прошли.
WASM: 9823241 bytes, SHA256
`476017f121b8317305b1418aa3d0e5f10dd66a7bd7083a241264bbe6825d8faf`.
Visual/STEP и rounding-inclusive error qualification остаются открытыми.
