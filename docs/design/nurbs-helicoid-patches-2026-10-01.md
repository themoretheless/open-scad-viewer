# Плотные геликоиды: сохранённые патчи

helicoid::approximate_patches использует тот же angular_profile и идеальную
Hermite remainder estimate, что approximate. Single surface сохраняет прежний
лимит32controls; новый результат при превышении лимита decompose existing knots
и строит radial ruled surface для каждого span. До85patches, U=[0,1],
V сохраняет исходные subdomains; height и theta linear in normalized V.
Ни нового fit, ни изменения budget нет. Для малого профиля возвращается один patch.

Независимый native test проверяет axis/annular варианты, signed two turns/height,
angular equation/radial interpolation, exact V domains, neighboring boundaries,
малый single patch и отказ сверх85span budget. Full native363tests/2doctests прошёл.
JSON surface_helicoid_patches; TypeScript approximateHelicoidNurbsPatches;
Rush helicoid_patches с теми же обязательными аргументами, что helicoid_surface.
Display через nurbs_patches_tessellate; example segments8 сохраняет mesh budget.
TypeScript typecheck, native language fixture и оба WASM build прошли.
Общий packaged прогон425tests/24files прошёл;83Rush fixtures включают
JSON rational-definition/hash preservation, OBJ/PLY display mesh tolerance1e-5mm
и exact indices, программный editor entrypoint. Geometry WASM9888718bytes,
SHA256 f8c34945d6d2bf533aecdcd7a6de303168a98dbb648bb5b661b6dc462400bd6f.

continuousBound:false, roundingCertified:false. Идеальный error estimate
не включает trig/knot-insertion/binary64 roundoff. Патчи не sewn solid,
visual editor review/STEP/continuous rounding-inclusive остаются открытыми.
Нового catalog figure ID не добавляется: это расширение NURBS-0046.
