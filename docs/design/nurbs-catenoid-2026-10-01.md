# NURBS-0047: катеноид

Статус integrated. catenoid::approximate(center,scale,start_z,end_z,budget).
JSON surface_catenoid: center/scale/start_z/end_z/max_deviation.
Ось Z, центр — центр окружности горловины. Профиль r(z)=scale*cosh(z/scale),
где z локален относительно center.z, scale>0, границы z возрастают.
Все координаты, scale и budget — длины. Это аналитическая форма;
условия минимальной поверхности и физическая задача плёнки не решаются.

Общий catenary-конструктор даёт cubic Hermite профиль. Его контролы
переставляются из [z,height,0] в [radius,0,z] с radius=height+scale,
затем существующий rational revolve строит полный круг. Профиль approximate,
круговые сечения рациональны без выборочного фиттинга. Положительные
радиальные контролы обязательны; max32 контроля на ось поверхности
ограничивает профиль максимум десятью кубическими spans. Более плотный
запрос отклоняется, а не снижает точность; multipatch пока отдельная работа.

Ideal profile remainder сохранён. В точной арифметике вращение сохраняет
его евклидову величину; binary64 вычисления профиля, контролей и вращения
не включены: continuousBound:false, roundingCertified:false.
Method catenary-profile-Hermite-rational-revolution. V-домен [0,4],
параметризация рациональных четвертей круга, periodic_v=false. Последние
контролы каждой строки равны первым. Это не B-rep/solid/topology certificate.

Два native-теста прошли: независимое уравнение радиуса в 201 сечении
и пяти V-положениях, локальная высота, совпадение контролей кругового шва,
отказ scale=0 и запроса, превышающего бюджет одной поверхности.
WASM/TS/Rush/editor/export, visual/STEP и rounding-inclusive error ожидаются.

Добавлены TS-адаптер и graph/Rush catenoid_surface, schema,
контекстные LENGTH-размерности scale/start_z/end_z, constructionReport
и пример catenoid-surface.r. vue-tsc и language cargo check прошли;
language WASM собран. Проверки размерностей и обязательных полей Rush
через пакет и native frontend прошли. Geometry build завершился с
ошибками geometry-bridge во время параллельных изменений; subsequent
wasm32 cargo check geometry-bridge прошёл. Запущена новая сборка после
терминального отказа предыдущей. Geometry/editor/export пока не проверены.

Повторная сборка geometry WASM завершилась успешно: 9830852 байта,
SHA-256 a244c0b54b5a8767e7e66e3e9877bdfdcda033e5c76c26dc78d28fc4b97f6e5e.
Пакетный прогон: 348 тестов, 17 файлов; катеноид проверен по независимой
формуле радиуса, отказам, размерностям и отчёту приближения. Graph/editor
entrypoint и JSON/OBJ/PLY прошли для 71 фиксированного примера Rush;
координаты display mesh проверены с допуском 1e-5 mm, индексы точно.
Это программная проверка entrypoint, не визуальный просмотр редактора.
Полный native-прогон: 339 тестов nurbs-core, два doctests и языковые
тесты modelgraph-runtime/modelgraph-text прошли. vue-tsc прошёл.
Continuous rounding-inclusive error, visual review и STEP остаются
не квалифицированы; конструкция не отмечается qualified.

Dense profiles теперь поддержаны отдельным catenoid::approximate_patches
и graph/Rush catenoid_patches; packaged proof395tests/22files включает
78fixedexamples JSON/OBJ/PLY. Single-surface32-control limit сохраняется.
См. nurbs-catenoid-patches-2026-10-01.md.
