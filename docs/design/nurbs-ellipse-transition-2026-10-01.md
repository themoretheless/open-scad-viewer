# NURBS-0073: переход эллипс–эллипс

Native ellipse_transition::ruled(&EllipseSection,&EllipseSection). Section:
center,axis_u,axis_v; оба вектора осей имеют размерность length и задают
center+axis_u*cos(theta)+axis_v*sin(theta). Оси могут быть skew; независимость
и численная обусловленность проверяются существующим ellipse_arc admission.
Coincident control profiles отвергаются. Порядок/знаки осей определяют
angular correspondence без автоматического выбора twist или seam alignment.

Два rational degree2 profiles с одинаковыми weights дают G0 ruled поверхность
S(u,v)=(1-v)A(u)+vB(u), U/V=[0,1],9x2controls, degreeU2/degreeV1.
Rational angular parameter не линейный угол/arc length. Шов exact repeated
controls, periodic_u:false. Бинарные округления не сертифицированы;
self-intersections, interior regularity, G1/G2 blend и sewn solid не доказаны.

Native independent tests: rational quarter equations на1001U*5V, unequal
principal radii, rotated/skew axes, implicit first ellipse boundary, compact
control count, seam; nonfinite/parallel/zero axes и duplicate-profile refusals.
Full native371tests/2doctests прошёл.
JSON surface_ellipse_transition использует start/end center,axis_u,axis_v fields.
TypeScript ellipseTransitionNurbsSurface, Rush ellipse_transition_surface
с6обязательными vector fields. All fields length; typecheck прошёл.
Оба WASM build, native language fixture и packaged dimension test прошли.
Полный450tests/27files прогон прошёл, включая87Rush fixtures: JSON hashes/
rational definitions сохраняются, OBJ/PLY coordinates tolerance1e-5mm,
triangle indices точные. Программный editor entrypoint нового примера прошёл.
Packaged geometry9898203bytes, SHA256
806cc70afa9b9abc8d30ff8a508ae9af9f9e3b4d5d9392496794b4da134194cc.
Actual visual review, continuous rounding-inclusive и STEP ещё открыты.
