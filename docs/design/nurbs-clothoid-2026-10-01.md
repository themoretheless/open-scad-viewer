# NURBS-0014: клотоида

Статус integrated. clothoid::approximate(center,length,start_curvature,
end_curvature,phase_degrees,budget). JSON curve_clothoid.
Length L>0, endpoint curvatures signed inverse model lengths,
phase degrees. theta(t)=phase+L*k0*t+L*(k1-k0)*t^2/2;
P(t)=center+L*integral_0^t[cos(theta),sin(theta),0]du, t=[0,1].
Start point center, unit tangent phase, linear arc-length curvature.
Signed/reversed curvature и limits straight/circle поддерживаются.
Это authored trajectory, не solver заданных endpoints/радиусов сопряжения.

Нормализованные коэффициенты a=L*k0,b=L*(k1-k0), w=max(abs(a),abs(a+b)).
Composite Simpson uniform even intervals выбирается из2..4096.
Четвёртая производная integrand components ограничивается
M=w^4+6*w^2*abs(b)+3*b^2. Ideal Euclidean quadrature estimate:
qe=sqrt(2)*L*M/(180*n^4). Каждый partial integral от0..t имеет
не больший remainder. Выбирается qe<=budget/2.

Cubic Hermite использует numerical integral points и analytic tangent
L*[cos(theta),sin(theta),0]. Ideal fit estimate:
he=sqrt(2)*L*hypot(w^3,3*w*abs(b))/(384*spans^4), spans1..85.
Endpoint position Hermite basis weights nonnegative and sum1, поэтому
quadrature contribution<=qe, не умножается на число spans. Total qe+he
обязан помещаться в authored budget; отказ, без silent relaxation.

Report сохраняет quadratureIntervals, quadratureErrorEstimate,
hermiteErrorEstimate и total realArithmeticErrorEstimate. Binary64
trig/accumulation/control/evaluation rounding не включены, обе
continuousBound/roundingCertified false; method clothoid-Hermite-composite-Simpson.
До256 curve controls; экструзия плотной кривой будет multipatch.

Проверки используют независимый convergent complex exponential power
series интеграла exp(i*s^2/2), включая зеркальную кривую, shifted Fresnel
для ненулевой initial curvature и rotation. Проверены endpoint arc-length
jets, straight и constant-curvature circle limits и budget refusals.
WASM/TS/Rush/editor/export, inverse-length dimensions, visual/STEP
и continuous rounding-inclusive квалификация пока не завершены.

Три focused tests прошли, полный native-прогон352tests и2doctests
прошёл. WASM/TS/Rush остаются следующим шагом, статусnative не повышен.

Добавлены TS approximateClothoidNurbsCurve, strict graph/Rush clothoid_curve,
checked-in JSON schema/prompt. Contextual length uses LENGTH, endpoint
curvatures inverse LENGTH[-1,0], phase ANGLE. Bare numerics use canonical
model units; explicit expressions2/1mm сохраняют physical dimensions.
vue-tsc, native language tests и packaged Rush inverse-length test прошли.
Language WASM собран; geometry/editor/export через новый пакет ожидаются.

Packaged geometry/editor entrypoint/export завершены:382tests/21files
прошли, independent Fresnel series, inverse-length units/refusals,
both estimates/total through multipatch, JSON/OBJ/PLY76fixedexamples.
Display mesh regression positions с1e-5mm tolerance, indices точно;
JSON сохраняет hash/definitions. Actual visual review не проведён.
Geometry WASM9862017bytes, SHA-256
30a14a92533583972e581857ccbb4768a0463239edc0f0085b9c2b0de6cd38c3.
Текущий языковой прогон и vue-tsc прошли. Native352tests/2doctests
доказаны предыдущим прогоном этой implementation. Continuous rounding,
visual/STEP остаются unqualified; integrated не означает qualified.
