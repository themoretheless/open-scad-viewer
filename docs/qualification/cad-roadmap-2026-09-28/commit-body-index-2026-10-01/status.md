# Индекс тел при проверке коммита

prepareCommit ранее искал кандидата линейным bodies.find для каждого связанного экземпляра; источники также находились линейно. Построены индексы previous/next по ID; сравнения экземпляров, источников и locked objects сохранены. Существующие UI-тесты недопустимого изменения linked geometry и заблокированного экземпляра запущены вместе с typecheck. Полная задержка сцены пока не измерена; оптимизация поиска не закрывает историю/rendering.

7 selected UI tests passed (266 unrelated cases excluded explicitly); typecheck passed. Full CAD regression and browser benchmark await new pending WASM, live session68608.

1000-instance Chromium run passed source-edit/Undo, compact request, exact final export and post-orbit picking. Three source edits median 2275.406 ms / p95 4285.981 ms; max RAF gap 417.1 ms. Orbit RAF 42.734, automation cadence, not physical presentation FPS. Sampled JS heap 655,294,540 and aggregate RSS 2,648,080,384 bytes (not instantaneous peaks; shared RSS may double-count). Scene screenshot viewed. Full CAD 802 tests pass. These results do not establish causal speedup or meet whole-scene latency targets; source edit still exceeds one second. Fixture hash included in report.
