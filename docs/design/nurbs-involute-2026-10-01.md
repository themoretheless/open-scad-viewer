# NURBS-0011: эвольвента базовой окружности

Статус: integrated. `involute::approximate(center,radius,start,end,budget)`
и JSON `curve_involute` (center/radius/start_radians/end_radians/max_deviation).
Ось Z, плоскость XY. Радиус и бюджет положительны, углы в радианах конечны
и строго возрастают; центр конечен. Отрицательные углы допускаются.

P(θ)=center+r*[cosθ+θsinθ,sinθ−θcosθ,0]. Домен t∈[0,1],
θ=start+(end−start)*t. Касательная по t вычисляется аналитически:
r*θ*(end−start)*[cosθ,sinθ,0]. При θ=0 касательная нулевая;
регулярность в этой точке не заявляется. Это кривая эвольвенты, не полный
профиль зуба: толщина, угол давления, зеркальная сторона, основание,
вершина, впадина и повторение зубьев остаются отдельными операциями.

Кубический Hermite с единичными весами, до 85 spans/256 контролей.
Норма четвёртой производной по θ равна r*sqrt(9+θ²). Conservative ideal
remainder: sqrt(2)*r*sqrt(9+maxAbsAngle²)*h⁴/384, h=(end−start)/spans.
Первый подходящий spans выбирается в рамках бюджета, иначе запрос
отклоняется. Binary64 sin/cos, построение контролей и NURBS evaluation
не включены: continuousBound:false, roundingCertified:false.
Переполнение/непредставимая оценка отклоняются.

Три native-теста прошли: независимая формула в 1001 точке для трёх
диапазонов (включая отрицательные углы и переход через ноль), крайние
касательные, увеличение spans при ужесточении бюджета, отказы radius=0,
обратного диапазона и недостижимого бюджета, JSON/report. Выборка не
является непрерывным сертификатом ошибки. WASM/TS/Rush/editor/export,
visual review и STEP qualification пока ожидаются.

Добавлены TS-адаптер approximateInvoluteNurbsCurve (радианы),
graph/Rush involute_curve (start_degrees/end_degrees с угловыми единицами),
сохранение constructionReport и пример involute-extrusion.r. Rust language
cargo check и vue-tsc прошли. Языковой WASM собран, размерности и
обязательность бюджета через пакет прошли. Геометрический WASM собирается;
точность адаптера, перевод градусов, редактор и экспорт ещё не проверены.

Пакетная проверка завершена: 287 тестов в 10 файлах прошли, включая
независимую формулу, крайние касательные, перевод graph градусов в радианы,
Rush размерности и обязательный бюджет, сохранение constructionReport
и запрет certified error при approximate curve. Программный вход редактора
и JSON/OBJ/PLY round-trip проверены на 62 фиксированных Rush-примерах.
JSON сохраняет definitions/hash; OBJ/PLY — индексы и координаты сетки
с допуском 1e-5 mm. Это не visual review и не continuous error certificate.
Полный Rust-прогон: 325 NURBS-тестов, 2 doctest и языковые тесты прошли.
WASM: 9810133 bytes, SHA256
`8deecc676b2d8e511ed8bb18fe8f3a7c8c39fa8dd4d877dc6782b49c0b92b5a0`.
Visual/STEP и rounding-inclusive error qualification остаются открытыми.
