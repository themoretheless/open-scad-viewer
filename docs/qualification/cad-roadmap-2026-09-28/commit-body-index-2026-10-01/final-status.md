# Индекс тел при проверке коммита

Повторные линейные поиски тела/источника заменены индексами previous/next по ID. Полные сравнения геометрии экземпляров, источников и locked objects сохранены.

7 целевых UI-тестов и полный CAD-прогон 802 теста прошли, как и typecheck/build/dist. Контрольная CPU Chromium сцена: 1000 linked instances, три source-edit/Undo, compact worker requests, окончательный JSON и выбор после orbit проверены. Fixture SHA-256 включён в measurements.json; финальный снимок просмотрен.

Source edit median 2275.406 ms, p95 4285.981 ms; max RAF gap 417.1 ms. Orbit RAF cadence 42.734 (automation, не physical FPS). Sampled heap 655 294 540 bytes, aggregate RSS 2 648 080 384 bytes; это не гарантированные пики, shared pages могут учитываться повторно. Замер не доказывает причинного ускорения и не закрывает целевые задержки больших сцен.
