# NURBS-0020: пространственная кривая Лиссажу

Статус integrated. lissajous::approximate(center,amplitudes,frequencies,phases,budget).
JSON curve_lissajous: center/amplitudes/frequencies/phases_degrees/max_deviation.
Все векторы имеют три компоненты. Центр и амплитуды — длины, амплитуды
неотрицательны. Частоты — подписанные обороты на нормализованном t∈[0,1],
фазы — градусы; допустимы дробные и нулевые частоты, но хотя бы одна ось
должна меняться (amplitude>0 и frequency≠0). Константная кривая отклоняется.

P_d(t)=center_d+amplitude_d*sin(phase_d+2π*frequency_d*t).
Касательные вычисляются аналитически. До 85 кубических Hermite spans,
256 контролей; общий ideal remainder равен hypot по трём координатам
amplitude_d*(abs(2π*frequency_d)/spans)^4/384. Это оценка интерполяции
в точной арифметике; binary64 sin/cos, контролы и evaluation не включены:
continuousBound:false, roundingCertified:false. Непредставимая оценка,
переполнение и недостижимый бюджет отклоняются.

Два native-теста прошли: независимые формулы в 1001 точке для трёх наборов
частот (отрицательные, дробные и нулевые компоненты), крайние производные,
ужесточение бюджета, отказы при отрицательной амплитуде, константной кривой
и чрезмерных частотах. Целые частоты не означают binary64 точного замыкания:
periodic=false; автоматическое snap/шов/топология не заявляются.
WASM/TS/Rush/editor/export, visual/STEP и rounding-inclusive квалификация
ещё ожидаются. Предназначение — пространственные траектории и колебательные
профили; дальнейший sweep требует собственных проверок регулярности.

Добавлены TS-адаптер, graph/Rush lissajous_curve, schema, runtime векторные
размерности (amplitudes LENGTH, frequencies SCALAR, phases_degrees ANGLE),
constructionReport и пример lissajous-extrusion.r. vue-tsc и language
cargo check прошли. Языковой WASM собран. Geometry WASM ещё собирается;
геометрия TS/graph, программный editor entrypoint и экспорт ожидаются.
Пример использует короткий участок с частотами [0.125,0.25,0.375], чтобы
при сохранённом бюджете 1e-4 mm не превысить 32 контроля single-surface
экструзии. Более длинные кривые могут требовать разделения на поверхности.

Пакетная проверка завершена: 301 тест в 12 файлах прошёл. Проверены
независимые формулы и крайние производные трёхосевых траекторий,
размерности векторов и обязательные поля Rush, графические фазы в градусах
и сохранение несертифицированного constructionReport. Программный вход
редактора и JSON/OBJ/PLY round-trip прошли для 64 фиксированных примеров:
JSON сохраняет definitions/hash, сетка — индексы и координаты с допуском
1e-5 mm. Это не visual review или continuous error certificate.
Полный Rust-прогон: 329 NURBS-тестов, 2 doctest и языковые тесты прошли.
WASM: 9816225 bytes, SHA256
`e0c35b002a0d5573a55842124b726a3d0caffe62b3b7ef06cc4558a90fb33f93`.
Visual/STEP и rounding-inclusive error qualification остаются открытыми.
