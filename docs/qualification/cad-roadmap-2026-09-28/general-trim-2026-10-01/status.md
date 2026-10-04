# Общие NURBS в проверке простоты контура

Расширена достаточная проверка trim_simplicity на многопролётные рациональные B-spline. Строгий общий знак одной координатной производной во всех исходных узловых пролётах доказывает injectivity. Используются интервальные jets по оригинальным коэффициентам, с ограниченным подразделением; исходная кривая не разлагается в приближённую замену. Число проверенных ячеек входит в общий бюджет. Исчерпание бюджета или отсутствие достаточного признака оставляет результат unproven.

Пять тестов простоты и четыре теста региона проходят, включая rational multispan + hole, обратную ориентацию, возвращающийся сегмент, малый бюджет, неизменность исходной геометрии. Полная native NURBS regression session59646 запущена. Общая подготовка профиля/offset/extrusion по-прежнему требует подключения и квалификации: planar_trim analytic carrier не заменён. Весь план P0–P3 остаётся открытым.

Full native NURBS regression passed: 223 tests, no failures/skips, 34.25 seconds. WASM build launched for runtime regression; no public artifact claim yet.

Runtime tests prepared in solidDistance.test.ts and mainSolidWorkerRealBoundary.test.ts: replace one cube UV coedge with equivalent degree-2 multispan B-spline, require trimValid on direct WASM and real postMessage, require volume-validity-unproven on trimCells=1 and preserve original request. Tests intentionally await current kernel session68608. Typecheck passed before latest worker test. General retained-profile storage still calls analytic planar_trim validation/area; native extrusion and Boolean admission likewise need extension, so this sufficient trim evidence is a prerequisite, not a complete general-profile path.

Current WASM packaged 9,578,146 bytes. Direct/runtime and real postMessage checks pass: 27 targeted tests; full roadmap 802 tests / 67 files pass. Face-domain native regression passed. Embedding-native process91261 still live. Vite and dist gate pass: 7,165,388 asset bytes + 11,634,266 raw WASM. No claim of general retained-profile/extrusion support.
