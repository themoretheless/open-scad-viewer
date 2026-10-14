# NURBS-0025: сферическая спираль

Статус integrated. spherical_spiral::approximate(center,radius,longitude_turns,
latitude_turns,longitude_phase_degrees,latitude_phase_degrees,budget).
P(t)=center+r*[cos(B)cos(A),cos(B)sin(A),sin(B)], t=[0,1],
A=longitude_phase+2*pi*longitude_turns*t,
B=latitude_phase+2*pi*latitude_turns*t. Radius>0, turns ненулевые signed
fractional, phases degrees, center/radius/budget model lengths. Pole
crossings разрешены. Это не equal-area spacing или geodesic solver.

XY product-to-sum гармоники имеют частоты a+b и a-b, Z частоту b.
Mxy=r/2*(abs(a+b)^4+abs(a-b)^4), Mz=r*abs(b)^4;
ideal estimate sqrt(2*Mxy^2+Mz^2)/(384*n^4). Вычисления через
масштабированные частоты/hypot избегают промежуточного переполнения.
До85 cubic Hermite spans/256 controls. Непредставимые оценки и слишком
точные запросы отклоняются без ослабления authored budget.
Binary64 trig, controls и evaluation не включены: continuousBound:false,
roundingCertified:false; периодичность/closure не заявляются.
Приближение не удовлетворяет точному sphere constraint между узлами.

Два native-теста прошли: независимая формула и radial equation в1001
точке для полюсов, signed/fractional turns, endpoint analytic jets,
refusals нулевых радиуса/rates и недостижимого бюджета.
JSON curve_spherical_spiral, TS approximateSphericalSpiralNurbsCurve,
graph/Rush spherical_spiral_curve добавлены. Полная траектория с1e-4mm
использует surface_extrude_patches, не ослабляет допуск для single-surface
бюджета. Packaged geometry/language builds и tests ещё выполняются.
Visual/STEP и continuous rounding-inclusive qualification впереди.

Packaged проверки завершены:375tests/20files прошли, независимые
координаты/radial equation, Rush units/refusals, construction report
и JSON/OBJ/PLY75fixedexamples. Display mesh coordinates с1e-5mm
regression tolerance, triangle indices точно; JSON retains hash/definitions.
Программный editor entrypoint проверен, actual visual review не проведён.
349nativeNURBS tests,2doctests и языковые тесты прошли; vue-tsc прошёл.
Geometry WASM9853126 bytes, SHA-256
c68c273773d6768c4a0ce34a8be9ecc9db4551908a6de820e56c22b36e09983e.
Continuous rounding-inclusive error, visual/STEP остаются незавершёнными;
N25 integrated, не qualified. Каталог69integrated/44planned,
113specified/1087unallocated до1200.
