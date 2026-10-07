# NURBS-0012: архимедова спираль

Статус integrated. archimedean_spiral::approximate(center,start_radius,end_radius,
start_degrees,end_degrees,budget). JSON curve_archimedean_spiral с теми же
полями и max_deviation вместо budget. Углы в градусах, конечны и строго
возрастают; endpoint radii неотрицательны, хотя бы один положителен.

P(t)=center+[r(t)*cosθ(t),r(t)*sinθ(t),0], r(t)=(1−t)*r0+t*r1,
θ(t)=start+(end−start)*t. Ось Z, плоскость XY; endpoint radius может быть
нулевым. Радиус линейный по углу, сжатие допускается. Равные радиусы дают
круговой предельный случай, без отдельного каталожного ID.

Обёртка использует общий conical-helix Hermite-генератор с нулевой высотой;
алгоритм не дублируется. До 85 spans/256 контролей. Ideal remainder включает
производную радиуса: hypot по XY координатам оценки
(maxRadius*step^4+4*abs(r1−r0)*step^3/spans)/384,
step=abs(2π*(end−start)/360)/spans. Binary64 перевод углов, sin/cos,
контролы и evaluation не включены: continuousBound:false,
roundingCertified:false. Непредставимые оценки и недостижимый бюджет
отклоняются; regularity/самопересечения/замыкание/топология не сертифицированы.

Два native-теста прошли: независимые формулы в 1001 точке для растущей,
сжимающейся и начинающейся в центре спирали на двух оборотах; крайние
касательные; отказы обратного диапазона, отрицательных/обоих нулевых
радиусов и недостижимого бюджета. WASM/TS/Rush/editor/export, visual/STEP
и rounding-inclusive error qualification остаются впереди.
Существующий NURBS-0012 используется; параметры и варианты не добавляют ID.

Добавлены TS-адаптер и graph/Rush archimedean_spiral_curve, строгая
schema, runtime длины радиусов и угловые границы, constructionReport
и пример archimedean-spiral-extrusion.r. vue-tsc и cargo check языковых
crate прошли; language WASM собран. Geometry WASM собирается,
пакетные geometry/editor/export проверки ожидаются. Общий diff-check
обнаружил trailing whitespace в параллельно изменяемых brep-core файлах;
узкий diff-check файлов текущего конструктора используется отдельно.

Пакетная проверка завершена: 334 теста в 15 файлах прошли. Проверены
независимые формулы растущей/сжимающейся/начинающейся в центре спирали,
передача градусов, размерности и обязательные поля Rush, сохранение
constructionReport и отсутствие certified error. Программный вход
редактора и JSON/OBJ/PLY round-trip прошли для 69 фиксированных примеров:
JSON сохраняет definitions/hash, сетка — индексы и координаты с допуском
1e-5 mm. Это не visual review или continuous error certificate.
Полный Rust-прогон: 335 NURBS-тестов, 2 doctest и языковые тесты прошли.
WASM: 9823255 bytes, SHA256
`db2f609869810005ad2468068c16130684b5360640e7745e59ea7116b45c04e2`.
Visual/STEP и rounding-inclusive error qualification остаются открытыми.
