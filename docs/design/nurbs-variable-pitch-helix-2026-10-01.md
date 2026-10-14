# NURBS-0023: винтовая кривая с переменным шагом

Статус: integrated. `helix::approximate_variable_pitch(center,radius,height,
turns,start_pitch,end_pitch,phase_degrees,max_deviation)` и JSON
`curve_variable_pitch_helix` реализованы. WASM/TS/Rush, программный вход
редактора и JSON/OBJ/PLY проверены; visual/STEP и error qualification ожидаются.

Радиус постоянен, ось Z. Шаг — подписанное осевое расстояние на подписанный
оборот, то есть z′(0)=turns*start_pitch, z′(1)=turns*end_pitch по t∈[0,1].
Высота задаёт z(1)−z(0); кубический Hermite-закон согласует эти четыре
условия. При произвольных условиях движение внутри может менять направление.
Это не произвольная функция шага и не ограничение монотонности.

XY использует общий helix-генератор до 85 spans/256 контролей. Кубический Z
представлен через blossom полинома в той же единично-весовой B-spline базе,
без дополнительной выборочной аппроксимации. Его четвёртая производная нулевая,
поэтому ideal Hermite remainder остаётся оценкой XY. Binary64 вычисления
коэффициентов, контролей, тригонометрии и NURBS не сертифицированы:
continuousBound:false, roundingCertified:false. Переполнение отклоняется.

Семь целевых helix-тестов прошли: новый случай проверен в 1001 точке для
положительных и отрицательных оборотов, с независимым Hermite-полиномом Z,
производными на обоих концах, JSON/report и отказами NaN/overflow.
Выборочная проверка не заменяет непрерывную оценку с учётом округления.

Дополнительная проверка `--no-default-features` не прошла: текущий
параллельный перенос сериализации оставляет `check` под codec, обращения
к value_codec/transport и другим сериализационным функциям без feature-gates.
Это ограничение всей текущей библиотеки, не подтверждение standalone native
сборки. Обычная default-feature проверка семи helix-тестов прошла.

Добавлены TypeScript-адаптер, строгая graph-schema и исполнение с
сохранением constructionReport, Rush lowering, runtime размерности шага
как длины и языковой selector. Пример variable-pitch-helix-extrusion.r
добавлен в экспортные regression fixtures; обязательность полей проверяется
отдельным frontend-тестом. vue-tsc и cargo check трёх языковых crate прошли.
Пакет геометрического WASM сейчас собирается; статус native сохраняется
до проверки адаптера, редактора и экспорта через новый пакет.

Пакетная проверка завершена: 280 тестов в 9 файлах прошли, включая
новый variable-pitch адаптер, независимые формулы и крайние производные,
размерности Rush, сохранение constructionReport и запрет certified error
при наличии ideal approximation. 61 фиксированный Rush-пример проверен
через программный вход редактора и JSON/OBJ/PLY round-trip; позиции сетки
сравниваются с допуском 1e-5 mm, индексы и JSON definitions/hash сохраняются.
Это не визуальная проверка и не continuous surface error certificate.
Полный Rust-прогон: 322 NURBS-теста и 2 doctest, языковые тесты также прошли.
vue-tsc и diff-check прошли. WASM: 9804835 bytes,
SHA256 `8d42b8a76f6f4e74fa0da4c027180fb0657ffd7da49a16b866a019c4e7d75f28`.
