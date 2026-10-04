# NURBS-0072: переход круг–прямоугольник

Native circle_rectangle_transition::ruled(circle,rectangle) возвращает4surfaces.
CircleSection задаёт center,normal,seam,radius; RectangleSection center и
orthogonal half-edge vectors axis_u/axis_v. Half-edges finite, norm>1e-12,
normalized dot<=1e-12. Corner order (+u,+v),(-u,+v),(-u,-v),(+u,-v).
Circle quarter0 начинается на authored seam и соответствует первому corner.
Нет автоматического phase/twist/seam matching.

Full circle decompose сохраняет4U domains [i/4,(i+1)/4]. Rational degree2
circle weights W повышаются доdegree3. Rectangle linear edge homogeneous
coordinates умножаются на W через Bernstein product, circle H повышаются
доdegree3. Общие weights в обоих V rows сохраняют Cartesian interpolation:
S(u,v)=(1-v)A(u)+vB(t), t=4u-i. V=[0,1], degreeU3/degreeV1,4x2controls
perpatch (32Cartesian controls total). No fitting or tessellation construction.
Boundary circle remains rational; rectangle edges linear. Adjacent patches
share endpoint profiles in real arithmetic, including final cyclic seam.

Independent native test проверяет quarter rational equation, authored rectangle
edges и interior linear interpolation на201samples/patch и5V stations; cyclic
seams, compact controls, invalid skew/zero/nonfinite frames. Full native
прогон373tests/2doctests прошёл. JSON surface_circle_rectangle_transition возвращает
все4surface definitions, не sewn solid. Binary64 trig/decomposition/degree
operations и products не имеют rounding-inclusive certificate.

TypeScript circleRectangleTransitionNurbsPatches, graph circle_rectangle_transition
и Rush constructor подключены, all7fields required. Centers/radius/half-edges
length; circle normal/seam scalar. Typecheck, native syntax fixture и language
WASM прошли. Geometry packaging и полный456tests/28files прогон прошли.
88Rush fixtures сохраняют JSON hashes/rational definitions, OBJ/PLY coordinates
с допуском1e-5mm и exact triangle indices. Новый программный editor entrypoint
прошёл. Geometry9904116bytes, SHA256
badfab5ba43b0c62464bd81947c173ae0f827d837a202dc89a5fd3d6fe576f93.
Actual visual, continuous rounding-inclusive error и STEP ещё впереди. G1/G2 blends, caps и regularity/self-intersection не доказаны.
