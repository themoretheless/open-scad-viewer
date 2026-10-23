# Происхождение сегментов подготовленного профиля

Добавлена последовательность curveSources, согласованная с выходными сегментами профиля после переориентации. Исходные сегменты сохраняют номер входной цепочки, номер сегмента и направление; явные соединители связывают конец текущей цепочки со следующей. При отказе проверки контура возвращается происхождение обхода для будущей локализации ошибки.

15 native profile preparation tests pass, including clockwise/counterclockwise rational arcs and explicit connectors. Initial typecheck passed. Added worker response validation and real postMessage parity test; awaiting current WASM build (live session 70117). Latest typecheck session 81883. No claim of general NURBS region support: analytic planar trim remains the next dependency.

Worker provenance validation now derives source segment counts from the actual request and rejects out-of-range chain/segment/connector references and malformed direction/profile fields. Four transport tests and latest typecheck passed. Native release compilation completed in 2m14s; optimization remains live on session 70117. Real worker parity awaits packaged new kernel.

Packaged WASM 9,576,180 bytes; 54 focused tests pass across five files, including actual profile provenance postMessage parity. Browser rational arc preparation with provenance capture passed cancel/Undo/repeat/JSON reload and extrusion/STEP download. Vite and dist gate passed: 7,164,247 assets + 11,632,300 raw WASM, feature budget increment measured 1,770 asset bytes. Full CAD session32696 remains live.
