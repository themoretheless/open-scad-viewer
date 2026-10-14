# NURBS-0026/0027: торические траектории

Статус integrated. Dense multipatch интегрирован; полная квалификация ещё не завершена.
toroidal_spiral::approximate(center,R,r,major_turns,minor_turns,
major_phase_degrees,minor_phase_degrees,budget) строит траекторию
P(t)=center+[(R+r*cos(B))*cos(A),(R+r*cos(B))*sin(A),r*sin(B)],
A=major_phase+2*pi*major_turns*t, B=minor_phase+2*pi*minor_turns*t.
R>r>0: ring torus. Signed fractional turns поддерживаются, оба ненулевые.
Phase в degrees, остальные геометрические величины/допуск в model lengths.

approximate_knot вместо turns принимает положительные взаимно простые
целые p,q в2..32. Это параметризация одного замкнутого торического узла;
links и unknots не проходят этот контракт. Заключительные point/tangent
samples переиспользуют начальные, endpoint controls совпадают точно.
Кривая всё ещё clamped с periodic=false; это не wrapping или доказательство
отсутствия самопересечений приближения/класса топологии.

Общий cubic Hermite builder. Разложение XY в сумму гармоник даёт
Mxy=R*abs(a)^4+r/2*(abs(a+b)^4+abs(a-b)^4), Mz=r*abs(b)^4,
a=2*pi*major_turns, b=2*pi*minor_turns. Estimate для n spans:
sqrt(2*Mxy^2+Mz^2)/(384*n^4). Вычисляется через масштабированные
частоты и hypot, без предварительного возведения больших частот в степень.
Не более85 cubic spans/256 controls. Непредставимые оценки и слишком
точные запросы отклоняются. Estimate не включает binary64 rounding,
continuousBound:false, roundingCertified:false. Метод отчёта общий
uniform-angle-cubic-Hermite-fourth-derivative-estimate.

Два native-теста прошли: signed fractional и integer turns по независимой
формуле в1001 точке, endpoint dP/dt, замкнутые контролы узла(2,3),
отказы для(2,4), horn torus и недостижимого допуска. JSON operations:
curve_toroidal_spiral и curve_torus_knot. JSON dispatch добавлен;
packaged geometry, frontend/editor/export, visual/STEP и непрерывная
rounding-inclusive квалификация впереди.

Полный native-прогон после JSON dispatch: 343 теста nurbs-core и два
doctests прошли. Каталог:66 integrated,2 native,45 planned; всего113
конкретных контрактов,1087 ещё не распределены до цели1200.

Добавлены TS wrappers approximateToroidalSpiralNurbsCurve/approximateTorusKnotNurbsCurve,
graph/Rush toroidal_spiral_curve/torus_knot_curve, checked-in schema/prompt,
угловые major/minor_phase_degrees и length/scalar dimension checks.
Language WASM завершён; native frontend fixture, vue-tsc и packaged Rush
units test прошли. Geometry WASM собирается; координаты/graph/export
через этот пакет пока не проверены.

Пример toroidal-spiral-extrusion.r — короткая signed-fractional траектория
с явным1e-4mm бюджетом. Полный torus-knot-extrusion.r — явно помеченный
coarse display с0.3mm бюджетом: существующая одиночная extrude surface
имеет32 profile controls, тогда как точная knot curve допускает256.
Сам конструктор не ослабляет бюджет; dense curve fitting тестируется
при1e-4mm. Multipatch extrusion для точного полного узла ещё требуется.

Packaged geometry завершена:9847788 bytes,
SHA-256 e30458947280098dae603b0481da574540b912fd791fe9e7c37388e384abbd5e.
367 тестов в19 файлах прошли: независимые координаты tight-budget curves,
closure/refusals, Rush units, explicit approximation reports, JSON/OBJ/PLY
для74 fixed examples. Mesh positions проверены с1e-5mm tolerance,
triangle indices точно; JSON сохраняет hash и rational definitions.
Это программный graph/editor entrypoint, не actual visual review.
N26/N27 отмечены integrated, не qualified; dense multipatch extrusion
полного узла сохраняется как явная незавершённая задача.

Native multipatch extrusion теперь реализована в extrusion_patches.rs;
три focused tests прошли, включая полный узел при1e-4mm. Интеграция
через WASM/TS/Rush/editor/export пока впереди; catalog remaining не снят.

Dense multipatch extrusion подключена к WASM/TS/Rush/editor entrypoint
и экспортам. Current full-knot example задаёт1e-4mm, прежний coarse0.3mm
пример заменён.368tests/19files прошли, включая все74fixedexamples
JSON/OBJ/PLY.347nativeNURBS tests и2doctests прошли. Remaining dense
extrusion снят изN27; rounding-inclusive,visual/STEP сохраняются.
