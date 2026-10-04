# NURBS-0046: геликоид

Статус integrated. helicoid::approximate(center,inner_radius,outer_radius,height,
turns,phase_degrees,budget). JSON surface_helicoid, graph/Rush helicoid_surface.
Ось Z, U/V в [0,1]. P(u,v)=center+[r(u)cos(theta(v)),r(u)sin(theta(v)),height*v],
r(u)=inner+(outer-inner)*u, theta(v)=phase+2*pi*turns*v.
Center, радиусы, height и budget — длины, turns — signed dimensionless,
phase — degrees. 0<=inner<outer, height!=0, turns!=0.

Общий helix Hermite-конструктор вычисляет внешний радиус при нулевом центре.
Внутренняя строка контролей масштабирует XY той же спирали; линейная
интерполяция U сохраняет радиальные образующие и общий угловой параметр.
Обе строки используют одинаковые единичные веса и knots. Z не масштабируется.
Real-arithmetic estimate внешней спирали ограничивает остальные радиусы.
Binary64 округление контролей, trig, масштабирования, перевода и оценки
не включено: continuousBound:false, roundingCertified:false;
method radial-ruled-helix-Hermite. Это approximate поверхность, не точная
рациональная форма transcendental спирали и не solid/topology certificate.

Одна поверхность ограничена32 контролами по V (до10 cubic spans).
Плотные запросы отклоняются без ослабления допуска; multipatch впереди.
Высота обязана быть ненулевой: вырожденный плоский сектор — другой контракт.
Полный оборот не замыкает Z, periodic_v=false.

Два native-теста прошли: независимая формула в201 V stations и пяти
радиусах для inner=0, положительного вращения и annular отрицательного
вращения/height. Проверены недопустимые радиусы, нулевые height/turns и
отказ слишком точного запроса. TS/Rush-адаптеры, dimension rules, пример
и регрессии добавлены; packaged geometry/language builds запущены.
Visual/editor/export и rounding-inclusive error ещё не квалифицированы.

Завершён packaged прогон: 355 тестов в18 файлах прошли, включая независимые
координаты, Rush dimensions, сохранение uncertified construction report и
JSON/OBJ/PLY для72 фиксированных примеров. Display mesh coordinates имеют
допуск1e-5 mm, индексы точны; JSON сохраняет hash и rational definitions.
Geometry WASM 9836527 bytes,
SHA-256 bfb6ec8d3a949ed381f4701f9910004c849ff51b253e2d69347140b4b9e7c90f.
Language WASM собран после обновления checked-in JSON-schema (первая
проверка dimensions обнаружила пропущенный экспорт схемы; исправлено).
vue-tsc прошёл; полный native-прогон341 NURBS tests,2 doctests и языковые
тесты прошли, отдельная native frontend fixture тоже прошла.
Editor entrypoint проверен программно; actual visual review, STEP и
continuous rounding-inclusive qualification остаются незавершёнными.
