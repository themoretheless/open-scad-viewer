# Native повторный допуск Body · 28.10.2026

Добавлен Body::requalify для affine/mapped и projective source Bodies. API принимает заимствованные типизированные Limits и заново выполняет material-shell replay, embedding/all-pair contact admission и signed-volume qualification. При остановке раннего этапа последующие не выполняются. Для affine Body с inverse shear используются только исходные axes/coefficient; inverse chart, регионы и контакты доказываются заново. Кэшированные signed-volume/volume/orientation и прежние результаты incidence/embedding не дают разрешения.

Бюджеты не увеличены и не переиспользуются для покупки недоказанного результата. Сохраняются прежние независимые scopes старого restore: region budgets на face/step, общий exact/driver budget оболочки и отдельные явно заданные embedding/volume limits. Копируются только исходные скалярные настройки, а не сертификаты. Неизменяемый входной Body сохраняется.

## Проверки

- Обычная и минимальная library-сборки прошли.
- Полный default прогон четырёх native-библиотек: **4208 прошли, 0 отказов, 6 прежних пропусков**.
- После снятия избыточного codec-guard с native cache-проверки повторены все **13 тестов изменённого incidence-модуля**, 0 отказов. Default-ветвь guard уже была активна в полном прогоне; native assertion теперь доступен и минимальному test target.
- Публичный target native_source_without_codec: **2/2**. Он повторно допускает affine/projective куб и capped canal, проверяет остановку при shell-work=1, неполном all-pair budget и volume-cells=1. Канал сверяется с независимым объёмом frustum + spherical caps; fresh volume и topology сравниваются с оригиналом.
- Default unit-проверки подменяют cached signed bounds, volume и orientation affine/inverse и projective Body; fresh admission восстанавливает исходные вычисленные величины.
- Inverse-shear fixture сравнивает native Body с прежним restore по исходной definition, объёму, ориентации, vertex IDs и incidence work/root/driver/reason. Rational-fraction shell fixture использует native replay, а с codec дополнительно сравнивает wire counters и определения. Prepared provenance проверяется типизированно; wire metadata assertions сохранены при codec.
- Архитектурных превышений 70, карта API 239/200 актуальна. Лимиты и pins не повышались.

Полный минимальный unit-test target пока не собирается: **456 ошибок вместо 468**. Он не объявляется квалифицированным; основной остаток — смешанные wire/native тесты source_support_shell и других модулей. Перечень primary diagnostics и хэши журналов — в matrix.json.

API повторно допускает уже поддерживаемые source-body классы. Это не новая геометрическая поддержка общего Boolean/Solid, центрального G2-угла, непрерывного offset или упрощения, и не поставка в WASM/UI. Все оставшиеся требования пяти направлений сохраняются. Полный план не завершён.
