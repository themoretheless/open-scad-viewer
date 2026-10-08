# NURBS-0049: винтовая поверхность профиля

Статус integrated. screw_surface::approximate(profile,origin,axis,height,
turns,phase_degrees,budget)->Approximation{patches,spans,budget,estimate}.
JSON surface_screw. Axis finite nonzero direction, origin model lengths,
height signed axial distance, turns signed revolutions (nonzero), phase
angular degrees. Height may be zero. Profile3D rational, unit circle law
is approximate Hermite; no fitted profile replacement. Angle and height
both linear in normalized V; U retains original profile parameterization.

Max radial distance R of positive-weight profile control hull chooses
common helix fit. Each profile control decomposes into axis-parallel,
radial and tangent parts, then combines polynomial cos/sin Hermite
controls with axial translation. Tensor product weights retain profile
rationality. In real arithmetic, radial distance at every profile point
is<=R, so common angular approximation error is<=the R-radius helix
estimate. This excludes binary64 normalization, projection, trig,
knot insertion, controls and evaluation rounding. Both certificate flags
false; method profile-screw-helix-Hermite. Report not a solid/topology cert.

Small curve axes<=32controls retain one span collection. Dense curves
split at active knots, preserving U and V subdomains; up to2048 patches.
Dense periodic profile needs explicit clamped encoding. All-axis profiles
refuse (degenerate rotational contract); controls/degrees retain existing
Curve/Surface budgets. No rounding-inclusive error or sewing certification.

Focused tests cover independent rational profile rotation, signed turns
and pitch, original domains, dense profile+motion splitting both axes,
weights, translated origin, huge axis scales, zero-height quarter-turn
and oblique-axis analytic endpoint. First test initially assumed every
half-turn needed multiple patches; corrected to require that for the
full-turn tight-budget case while checking all geometry for both cases.

WASM/TS/Rush/editor/export, actual visual review and STEP are pending.

Полный native-прогон355tests и2doctests прошёл после всех правок,
включая dense rational profile и oblique axis. Native JSON dispatch
добавлен; packaged WASM/TS/Rush/export ещё не доказаны, статусnative.

Добавлены TS approximateScrewNurbsSurfaces, strict graph/Rush screw_surface
как piped modifier, checked-in schema/prompt, mandatory motion fields.
Патчи и faceIds сохраняются; construction report передаётся в общий
build report. Пример rational weights[1,2], full turn и1e-4mm budget.
vue-tsc и языковые native tests прошли; language WASM собран.
Geometry WASM сейчас оптимизируется, packaged/export proof ожидается.

Packaged проверки завершены:389tests/22files прошли, независимая
рациональная screw equation для positive/negative turns/height,
Rush units/refusals, construction report и JSON/OBJ/PLY77fixedexamples.
Display mesh regression coordinates1e-5mm, indices точно; JSON preserves
hash/rational definitions. Программный editor entrypoint, не visual review.
Geometry WASM9869834bytes, SHA-256
ad003a6d608fe31b5914be87368fab8d13acf1ac3aede49481044b226b8147b7.
Текущие native языковые тесты и vue-tsc прошли.355nativeNURBS tests и
2doctests доказаны предыдущим прогоном этой implementation. Остались
continuous rounding-inclusive error,visual/STEP; статус не qualified.
