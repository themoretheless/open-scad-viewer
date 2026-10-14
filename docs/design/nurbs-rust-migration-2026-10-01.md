# Rust first: перенос оставшейся NURBS-логики

По уточнению пользователя основной результат — native Rust library, а не
наличие TypeScript/Rush/editor bindings у каждой фигуры. App integration
не является обязательным критерием готовности native API. Полная цель1200
не достигнута; existing catalog status integrated — исторический app-delivery
статус, не число полностью квалифицированных Rust algorithms.

Current source audit: nurbsCurve.ts/nurbsSurface.ts/nurbsConstructors.ts
основные вычисления уже вызывают Rust. rushGraphNurbsKernel.ts ещё содержал
матричное преобразование control points; оно перенесено в nurbs_core::affine.
Native typed APIs curve/surface/patches принимают [[f64;4];4], сохраняют knots,
degrees,weights/periodic encoding, отвергают projective/nonfinite matrices и
coordinates outside1e6. Singular maps по прежнему допустимы. JSON transport
остаётся optional adapter, TypeScript не вычисляет transformed coordinates.

surface::Evaluation дополнен native typed metadata: curvatures(),domains(),
derivative_status()->DerivativeStatus и derivative_sides()->DerivativeSide.
Эти значения прежде были доступны внешнему caller через codec fields.
Прямой examples/typed_nurbs_families.rs работает без default features, JSON,
WASM или viewer и проверяет7constructor families плюс cylinder curvature/jet.
Full no-default-features275tests/2doctests прошёл до добавления affine.
После affine добавления3focused no-default tests прошли. Typecheck прошёл.
WASM rebuild и364packaged tests в3files прошли: affine operation,88Rush
fixture export/editor иartifact checks. Full current no-default regression278tests/2doctests прошёл.

Не перенесён весь app orchestration: graph execution sequencing, UI metadata,
trim-wire endpoint prevalidation и агрегация display bounds ещё в TypeScript.
STEP/edit adapters требуют отдельного более широкого audit. Эти компоненты
не следует объявлять переведёнными только по проверке core constructor wrappers.
Continuous rounding-inclusive certification остаётся отдельной незавершённой
геометрической задачей; успешная компиляция не является её доказательством.
