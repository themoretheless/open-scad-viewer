# Native replay материальной оболочки · 28.10.2026

Добавлен Shell::requalify для affine/direct/mapped и Shell<ProjectiveEdge>::requalify. Replay повторно допускает все исходные материальные области, заново допускает записанные фрагменты обоих uses и требует буквального совпадения с фактическими границами восстановленных областей. Далее исходные world-кривые, raw mapping-рецепты, root/candidate/plane inputs, адреса и полюса передаются в существующий native incidence assembler.

Кэшированные faces, vertex IDs, направления и диапазоны не дают разрешения. Пропущенные uses, несовпадающие исходные границы и недостающие полюса отвергаются. Положительные fresh regions сами по себе не обходят повторную проверку общих рёбер и владения вершинами. Топология не создаётся из координатной близости.

## Бюджеты и границы

Exact-work и driver-work общие для всей сборки пар; каждый вызов edge replay с новым полным бюджетом не используется. Region budgets остаются независимыми по face/step, mapping budget записанных фрагментов — по use, как в старом wire restore. Сохранены пределы: 2..4096 регионов, 524288 affine-пар или 32768 projective-пар, 1048576 affine-полюсов или 65536 projective-полюсов, exact 1..100000000, driver <=100000. Неконечные projective plane/parameter/fraction proposals отвергаются до сборки. Бюджеты и допуски не повышались.

Возвращается свежий Report incidence, а не разрешение общего Boolean/Solid. Embedding, контакты, ориентация материала и объём остаются самостоятельными воротами.

## Проверки на окончательных исходниках

- Обычная и --no-default-features library-сборки прошли.
- Default четыре native-библиотеки: **4208 прошли, 0 отказов, 6 прежних пропусков**.
- Публичный native_source_without_codec: **2/2**, включая affine/projective whole-shell replay и отказ при общем exact-work=1; куб, exchange provenance, topology projection и независимый capped-canal volume oracle сохранены.
- Default helper сверяет native/wire shell replay по work/root/driver-счётчикам, причине, uncertain_pair, исходной definition, vertex IDs, uses, boundary display/network и vertex links. Дополнительно проверяется игнорирование подменённых cached faces/vertices, отказ при пропущенном use и полюсе.
- Projective tetrahedron использует native replay; с codec дополнительно сверяется старый JSON/binary путь. Общие topology, embedding, all-pair refusal, signed-volume oracle, native model conversion и STEP round-trip assertions сохранены. Wire-only forged-payload проверки остаются при codec.
- Архитектурных превышений 70; карта API 239/200 актуальна. Общий архитектурный gate остаётся открытым.

Полный минимальный unit-test target пока не собирается: **468 ошибок вместо 492**. Выключение смешанных native-тестов целиком не использовалось. Остаток и хэши журналов приведены в matrix.json.

Общие SS/Boolean/Solid, центральный G2-угол, непрерывный offset, упрощение и полная WASM/UI/STEP-квалификация остаются в плане. Новый native API не считается поставленным в браузерный артефакт. Полный план не завершён.
