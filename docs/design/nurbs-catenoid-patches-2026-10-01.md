# Расширение NURBS-0047: плотный катеноид набором патчей

Native API catenoid::approximate_patches(center,scale,start_z,end_z,budget),
JSON surface_catenoid_patches возвращает patches и approximation report.
Новый ID не создаётся: расширяется прежняя фигура NURBS-0047.
Старый approximate возвращает одну surface и сохраняет свой32-control cap.

Общий radial_profile создаёт ровно прежний catenary fit; для плотного
профиля деcompose использует knot insertion с исходными U-subdomains.
Каждый cubic span вращается полным rational circle вокруг Z. Ни budget,
ни spans, ни original curve fit не меняются. Профиль ограничен85spans/256
controls, набор до85patches; V=[0,4], periodic_v=false. Small profiles
с<=32controls возвращают один patch. Контролы кругового шва совпадают.
Это набор retained surfaces, не sewn B-rep или solid.

Estimate прежнего profile сохраняется, включая false continuousBound/
roundingCertified; binary64 knot-insertion/revolution не сертифицированы.
Method catenary-profile-Hermite-rational-revolution остаётся общим.

Три focused native catenoid tests прошли. Новый тест:scale1,z[-5,5],
budget1e-4mm требует больше10 cubic spans; прежняя single surface
отказывает, новый набор сохраняет requested fit. В каждом span проверены
5U и5V stations против независимого radial cosh equation, локальная
высота, исходные domains, соседние boundaries с1e-10mm regression
precision и точные control seams круга. Budget1e-10mm всё ещё превышает
85spans и отклоняется; без tolerance relaxation.

WASM/TS/Rush/editor/export нового patch-set entrypoint пока не завершены.
Существующий single-surface contract остаётся integrated, эта дополнительная
задача явно записана в remainingQualification каталога. Visual/STEP и
continuous rounding-inclusive qualification также остаются незавершёнными.

Полный native-прогон после общего profile refactor:356tests и2doctests
прошли; single-surface регрессии также покрыты. Packaged integration
нового entrypoint остаётся следующим шагом.

TS approximateCatenoidNurbsPatches, strict graph/Rush catenoid_patches,
checked-in schema/prompt, contextual LENGTH scale/start_z/end_z добавлены.
Construction report сохраняется отдельно от patch set, faceIds retained.
Пример catenoid-patches.r scale1,z[-5,5],budget1e-4mm с tessellation12.
vue-tsc прошёл, language WASM собран; packaged geometry/export pending.

Packaged integration завершена:395tests/22files прошли, независимая
radial equation, retained domains, adjacent boundaries, круговые seam
controls, Rush dimensions/report и JSON/OBJ/PLY78fixedexamples.
Display mesh coordinates с1e-5mm regression tolerance, indices точно;
JSON сохраняет document hash и rational definitions. Программный editor
entrypoint проверен; actual visual review не проведён. Ранний combined
Rush/build тест вызван до готовности нового geometry package и отказал
на старом пакете; повтор после терминальной упаковки прошёл полностью.
Geometry WASM9872913bytes, SHA-256
23ce135a2b0209f2b5c3c5018a644940a7d2bc6062b9b96a3065b42bf13513bf.
Native языковые тесты и vue-tsc прошли;356nativeNURBS tests и2doctests
доказаны предыдущим прогоном этой implementation. Дополнительная dense
integration снята изremaining каталога; rounding-inclusive,visual/STEP
остаются незавершёнными. Новый ID не добавлен,71figures integrated.
