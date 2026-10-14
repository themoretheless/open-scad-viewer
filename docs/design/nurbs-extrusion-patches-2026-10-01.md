# Экструзия плотной кривой набором патчей

Native API extrusion_patches::extrude(curve,vector)->Vec<Surface>,
JSON surface_extrude_patches. Операция расширяет существующую экструзию,
не добавляет новый ID фигуры и не меняет max_deviation исходного fit.

Для curves<=32 controls сохраняется одна существующая extrude surface.
Плотная nonperiodic 3D curve раскладывается по активным knot spans;
каждый сегмент trim строится knot insertion, без fitting или sampling.
Далее каждый retained сегмент экструдируется прежним rational loft.
До256 spans, degree<=25 из исходной Curve. Каждый patch сохраняет
исходный U-subdomain и V=[0,1]. Dense periodic curve отклоняется;
нужна явная clamped representation. Патчи не сшиваются в B-rep solid.
Real arithmetic representation сохраняется, binary64 knot-insertion,
translation и evaluation rounding не сертифицированы.

Три focused native-теста прошли: full(2,3) knot с1e-4mm fit budget,
плотная rational polyline с varying weights и nonlinear knot spacing,
небольшой rational профиль и отказы для2D/нулевого вектора.
Плотный knot отказывает у прежнего single-surface extrude, а новая
операция сохраняет оригинальную кривую на всех spans; проверены
параметры, sample correspondence, независимая knot formula и соседние
границы с1e-11mm numerical tolerance. Это regression samples, не
непрерывная rounding-inclusive qualification или topology certificate.

WASM/TS/Rush/editor/export интеграция ещё не завершена. После неё
нужно заменить coarse full-knot fixture на tight multipatch example,
проверить сохранение construction report и ограничения tessellation.

Интеграция завершена: TS extrudeNurbsCurvePatches, graph/Rush
surface_extrude_patches и checked-in JSON schema/prompt, retained faceIds.
Полный knot fixture заменён с coarse0.3mm на1e-4mm,
.nurbs_patches_tessellate(segments:8). Construction report исходной
кривой сохраняет budget1e-4 и оба false certificate flags. Tessellation
проверяет общий budget20_000triangles, не делает silent coarsening.
368 тестов в19 файлах прошли: native packaged correspondence каждого
патча, домены, construction report, JSON/OBJ/PLY74fixedexamples.
347 native NURBS tests,2 doctests и языковые тесты прошли; vue-tsc прошёл.
Geometry WASM9850576 bytes, SHA-256
9d4965e8fc11ebb72fb46aa58a63bb3463d9c5fef96ea152369758516c15094b.
Editor entrypoint проверен программно; visual review и STEP ещё не
квалифицированы. Samples не доказывают continuous rounding-inclusive
error и не превращают набор патчей в сшитый B-rep solid.
