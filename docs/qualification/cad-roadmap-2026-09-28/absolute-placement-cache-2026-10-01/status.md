# Абсолютное размещение экземпляра через worker-кэш

Маршрут instance-place теперь передаёт общий SolidInstanceBatchCache в resolveSolidInstances, как создание и относительное перемещение экземпляров. Проверены совпадение с вычислением без кэша, hit повторного размещения, сохранение вращения при изменении абсолютной позиции, последующая авторитетная JSON-проверка, неизменность исходных данных и независимость результатов.

21 тест в двух файлах прошёл. Улучшение полной задержки сцены не измерено; новый benchmark и браузерная регрессия остаются обязательными перед заявлением об ускорении. Typecheck запущен; WASM build другого этапа остаётся live session 70117.

Benchmark completed after normalizing fixture through the authoritative document parser. Initial raw-fixture run failed the JSON normalization comparison; no accepted timings from that run. Five alternating cached/uncached repetitions on identical target placement: 100 instances median 18.445 ms cached / 88.306 ms uncached; 1000 instances 275.051 / 921.038 ms. Cache retained 20,208,620 bytes for 1000. Every output matches uncached computation and subsequent JSON normalization; source unchanged. Scope excludes worker transfer, history, display and UI, so these figures do not close whole-scene latency acceptance. Typecheck passed.

Actual postMessage full/compact scene parity now includes instance-place and passed (1 selected test; 16 unrelated tests excluded by explicit name filter). Browser harness adds --absolute-placement, preserving complete matrix/source/Undo assertions; execution awaits frontend build with the pending packaged kernel. Binaryen optimizer confirmed live via process inspection at 99.7 percent CPU; unrelated second build is in root checkout, not this isolated checkout. No restart or process termination.

Mouse browser absolute placement passed complete matrix/source/Undo checks plus late preview cancellation and Apply without recomputation. Keyboard scenario launched; full CAD regression remains live session32696.
