# Native source boundary без обязательного кодека · 28.10.2026

Минимальная библиотечная конфигурация brep-core теперь собирается без value-codec. Wire definition/restore, сериализация диагностик и сохранённая original-метаинформация отделены feature-флагом codec; типизированные отчёты, геометрические ворота, native replay, подготовка обмена и generic topology projection сохранены. В normal dependency tree минимальной конфигурации value-codec отсутствует. Сняты старые codec-флаги с вычислительных linear_canal_body и to_capped_source_shell.

## Вычисления и идентичность

Обратный shear работает с типизированным Original/SplitRoot/SplitParameter-рецептом области. Для каждой границы сохраняются точная проверка композиции с inverse-поверхностью, свежий допуск корней, прежние счётчики и пределы. Cut-рецепты не получают нового разрешения. В default unit-тестах результат, причина и work/root/mapping-счётчики положительных native replay сравниваются с прежним JSON-путём, оставленным только тестовым oracle.

Проверка принадлежности Prepared конкретному Body больше не сравнивает JSON. Типизированный снимок сохраняет исходные рецепты областей (включая спецификацию допусков), оригиналы рёбер и их отображений, адреса uses, адреса/координаты полюсов и параметры inverse shear. Кэши и результаты квалификации не являются идентичностью. Независимо построенный Body с теми же буквальными оригиналами принимается; сдвинутый Body отвергается. Полное покрытие paired-contact также сравнивает типизированные исходные фрагменты/рёбра.

## Проверки на окончательных исходниках

- cargo check --manifest-path crates/Cargo.toml -p brep-core --offline --locked --no-default-features --lib: успешно, 0 ошибок вместо прежних 54.
- Та же обычная library-сборка с default features: успешно.
- Четыре default native-библиотеки: 4208 прошли, 0 отказов, 6 прежних пропусков (brep-core 2004/5, nurbs-core 1996/1, cad-predicates 100/0, nurbs-intersect 108/0).
- cargo test --manifest-path crates/Cargo.toml -p brep-core --offline --locked --no-default-features --test native_source_without_codec: 2/2.
- Тот же integration target с default codec: 2/2, дополнительно сверяется прежняя original-метаинформация.
- Архитектурный аудит: 70 прежних превышений; API-карта 239/200 актуальна. Лимиты, pins и допуски не повышались.

Публичный integration target строит/допускает замкнутый куб, проверяет region/affine/projective/carrier replay, endpoints/trims exchange, отказ для другого Body и native topology projection. Второй тест допускает замкнутый capped canal и сверяет интервал объёма с независимой формулой frustum + spherical caps. Wire restore не используется в минимальном target.

Полный минимальный unit-test target с wire-содержащими unit-модулями этим протоколом не квалифицирован: проверены минимальная библиотека и конкретные публичные integration paths. Это также не новая квалификация WASM, UI или STEP runtime-артефакта.

Общие Boolean/Solid, центральный G2-угол, непрерывный offset, упрощение и полная продуктовая интеграция остаются в плане. Сам план не завершён. Хэши журналов приведены в matrix.json.
