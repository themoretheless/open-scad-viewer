# NURBS-0071: параметрический переход круг–круг

Native circle_transition::ruled(&CircleSection,&CircleSection), где section
содержит center,normal,seam,radius. JSON surface_circle_transition использует
start_center/start_normal/start_seam/start_radius и соответствующие end fields.
Радиусы положительные, center/directions finite. Normal нормализуется со
scale guard; seam direction проецируется в плоскость круга, parallel/zero
projection отвергается. Авторские швы задают correspondence; normal определяет
направление обхода. Геометрия не выбирает shortest twist автоматически.

Два рациональных полного круга degree2 с9controls и одинаковыми weights/knots
loft даёт поверхность degreeU2/degreeV1 с18Cartesian controls, U/V=[0,1].
В реальной арифметике S(u,v)=(1-v)A(u)+vB(u). У обоих кругов одинаковая
rational angular parameterization; u не является линейным углом/arc length.
Шов сохраняется repeated endpoint controls, periodic_u:false. Совпадающие
control profiles отвергаются как вырожденный переход. Произвольное расположение
и ориентация остальных сечений могут дать singularity/self-intersection.

Native tests используют независимую rational quadratic quarter-circle formula
на1001U samples и5V stations, с переменным radius, translated centers и
orthogonal end planes. Дополнительно проверяются compact degrees/control count,
shared seam и invalid normals/seams/radii/center/coincident profiles.
Полный native прогон369tests/2doctests прошёл.

Это ruled G0 transition, не G1/G2 blend/fillet, caps или sewn solid. Continuous
rounding-inclusive error/regularity/self-intersection и STEP не доказаны.
TypeScript circleTransitionNurbsSurface и строгий graph circle_transition_surface
подключены; Rush требует все8section fields. Centers/radii length,
normal/seam directions scalar. Typecheck, language WASM, native syntax fixture
и packaged dimension test прошли. Geometry WASM packaging и полный444tests/26files прогон прошли.
86Rush fixtures сохраняют JSON document hash/rational definitions,
OBJ/PLY display mesh coordinates с допуском1e-5mm и точные triangle indices.
Программный editor entrypoint нового примера прошёл; actual visual review впереди.
Packaged geometry9895564bytes, SHA256
b44b18990d9195340d8ef55a1013c7162814111513369541f2138c2540891685.
