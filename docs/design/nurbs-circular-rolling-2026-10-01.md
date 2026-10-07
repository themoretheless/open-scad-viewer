# NURBS-0016 / NURBS-0017: эпициклоида и гипоциклоида

Статус integrated. circular_rolling::approximate_epicycloid и
approximate_hypocycloid(center,fixed_radius,rolling_radius,start,end,budget).
JSON curve_epicycloid/curve_hypocycloid: center/fixed_radius/rolling_radius/
start_radians/end_radians/max_deviation. Углы возрастают и задаются радианами;
радиусы положительны. Для внутреннего качения fixed_radius>rolling_radius.

Обозначения R=fixed_radius, r=rolling_radius. Снаружи A=R+r, q=A/r,
P=center+[A*cosθ−r*cos(qθ),A*sinθ−r*sin(qθ),0]. Внутри A=R−r,
P=center+[A*cosθ+r*cos(qθ),A*sinθ−r*sin(qθ),0]. Центр означает центр
фиксированной окружности, а не стартовую точку. Ось Z, плоскость XY.
Касательные по нормализованному t аналитические. Частоты q могут быть
нецелыми; замыкание в binary64 и periodic encoding не заявляются.

До 85 кубических Hermite spans/256 контролей. Conservative ideal remainder
sqrt(2)*(A*h⁴+r*(q*h)⁴)/384, h=(end−start)/spans. Binary64 отношение
радиусов, тригонометрия, контролы и evaluation не включены в сертификат:
continuousBound:false, roundingCertified:false. Недостижимый бюджет,
переполнение и непредставимая оценка отклоняются. Регулярность острий,
пересечения, замыкание, профиль зуба и solid topology не сертифицированы.

Два native-теста прошли: независимые формулы в 1001 точке для обоих видов,
нулевые касательные при θ=0, крайние производные, отказы невозможного
внутреннего радиуса, нулевого радиуса и чрезмерной частоты/бюджета.
WASM/TS/Rush/editor/export, visual/STEP и rounding-inclusive error ожидаются.
Два существующих каталожных ID сохранены; размеры/отношения не добавляют ID.

Добавлены TS-адаптеры и graph/Rush epicycloid_curve/hypocycloid_curve,
строгая schema, runtime LENGTH для fixed_radius/rolling_radius, угловые
границы, constructionReport и два extrusion-примера. vue-tsc и cargo check
языковых crate прошли; language WASM собран. Geometry WASM собирается,
пакетные geometry/editor/export проверки ожидаются; статус native сохранён.

Пакетная проверка завершена: 327 тестов в 14 файлах прошли. Проверены
независимые формулы обоих видов качения, острия, перевод градусов,
размерности и обязательные поля Rush, несертифицированные constructionReport.
Программный вход редактора и JSON/OBJ/PLY round-trip прошли для 68
фиксированных Rush-примеров: JSON сохраняет definitions/hash, сетка —
индексы и координаты с допуском 1e-5 mm. Это не visual review или
continuous surface error certificate. Полный Rust-прогон: 333 NURBS-теста,
2 doctest и языковые тесты прошли. WASM: 9821338 bytes, SHA256
`623ff72dea1ea9c078acc64d7a2191ebebef5dccca67f7eadb1a28f1ede60d3b`.
Visual/STEP и rounding-inclusive error qualification остаются открытыми.
