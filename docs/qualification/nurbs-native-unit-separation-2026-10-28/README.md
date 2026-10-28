# Разделение native и wire unit-проверок · 28.10.2026

Минимальная библиотечная сборка и публичные native integration paths были квалифицированы отдельно. Полный unit-test target без codec впервые проверен командой cargo test --manifest-path crates/Cargo.toml -p brep-core --offline --locked --no-default-features --lib --no-run: исходно 571 ошибка тестового кода. Это другая область проверки, не регрессия ранее прошедшей library-сборки.

В тестах SourcePoint, Fragment и рациональных root-параметров native requalify и типизированная исходная идентичность отделены от wire round-trip. В минимальном варианте сохраняются проверки исходных селекторов, подменённых кэшей, изменённых уравнений, порядка концов, рациональных кандидатов и отказов по бюджету. Default-вариант дополнительно сохраняет прежние JSON/binary assertions.

Общие test helpers SourceRegion и Restriction проверяют native replay, параметры, исходные фрагменты, preview/display enclosures, непрерывность, нулевые/избыточные бюджеты и независимые world-evaluation witnesses. Их прежний wire helper выполняется дополнительно при codec. Сравнение direct/mapped replay использует типизированный pristine original, а wire-результат сравнивается отдельно при codec. Самостоятельный diagnostic_wire тест, проверяющий исключительно JSON-формат отчёта, ограничен feature codec. Вычислительные модули и смешанные native-тесты целиком не выключались; новых ignore или поднятых лимитов нет.

Результат: **492 ошибки вместо 571** в полном минимальном unit-test target; он пока не собирается и его успешный запуск не заявляется. Наибольшие группы: source_support_shell 228, source_shell_incidence 67, source_projective_use 25, source_shared_edge/tests/mod 22. Полный список primary diagnostics сохранён в matrix.json и журнале. Далее нужны отдельные native/wire части этих смешанных тестов с сохранением математических проверок.

На окончательных исходниках default четыре native-библиотеки: **4208 успешных, 0 отказов, 6 прежних пропусков**; публичный native_source_without_codec с --no-default-features: **2/2**. Карта API 239/200 актуальна, архитектурных превышений 70, общий архитектурный gate остаётся открытым. Хэши журналов — в matrix.json.

Этот этап не завершает полный план. Геометрические направления Boolean/Solid, центрального G2-угла, offset, упрощения и продуктовой WASM/UI/STEP-квалификации сохраняются.
