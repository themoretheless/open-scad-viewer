# Native/wire разделение проверок Body · 28.10.2026

Смешанные тесты capped canal и source_shell_incidence используют native Body/Shell replay для вычислительных проверок. Сохранены независимый cap/frustum volume oracle, signed volume, orientation, all-pair count, vertex owners, преобразование в Model, сравнение generic topology/pcurves и STEP-проверки. Отказы при volume-cells=1 и shell-exact-work=1 проверяются native-путём. Cached-volume/orientation revalidation выполняется в общем варианте теста канала.

При codec дополнительно остаются прежние serialized-payload проверки: подделанные success/volume/orientation flags, смена/удаление inverseShear, JSON/binary round-trip, wire metadata, fixture-file hooks и сравнение исходного/восстановленного Model. Native/wire replay дополнительно сравнивается по incidence work/root/driver/reason. Типизированная исходная идентичность используется для refinement originals, coedges/pcurves и restriction carriers; Prepared provenance проверяется через native belongs_to.

Производственные геометрические алгоритмы в этом этапе не менялись. Вычислительные модули или смешанные native-тесты целиком не выключались, новых ignore нет; wire-only операции отделены feature codec.

На окончательных исходниках default четыре native-библиотеки: **4208 прошли, 0 отказов, 6 прежних пропусков** (brep-core 2004/5, nurbs-core 1996/1, cad-predicates 100/0, nurbs-intersect 108/0). Полный минимальный unit-test target всё ещё не собирается: **412 ошибок вместо 456**. В primary diagnostics больше нет linear_canal_body или source_shell_incidence, но это не заменяет успешный запуск всего минимального target. Архитектурных превышений 70, API-карта 239/200 актуальна. Хэши и остаток — в matrix.json.

## Следующий геометрический рубеж

Основной приоритет 1 остаётся получением настоящего Boolean-тела из квалифицированных SS-разбиений. QualifiedClosedGraphSides доказывает сторону целой области относительно противоположной опоры, но не принадлежность объёму. Нужен отдельный точный guard исходного graph body: roof/floor/footprint, вертикальные границы, ориентация и полная связь с реальной топологией, затем source-owned sewing выбранных участков и независимая проверка результата.

Существующий canonical_graph_frame_cell в nurbs_ss_g6/nurbs_boolean_imprint_certificate.rs использует tolerance/near_point и ограниченный класс графовой формы. Его распознавание нельзя считать новым точным доказательством объёма. Новая проверка должна опираться на исходные коэффициенты/положительные веса и полные границы, с ограниченным бюджетом и без близостной склейки. Native source-shell/body admission теперь доступен для повторной проверки поддерживаемых исходных опор, но новые SS contact/body роли требуют собственного доказательства.

Остаток минимальных unit-тестов сохраняется отдельным открытым gate. Общие Boolean/Solid, центральный G2-угол, continuous offset, упрощение и полная WASM/UI/STEP-квалификация не завершены. Полный план остаётся открытым.
