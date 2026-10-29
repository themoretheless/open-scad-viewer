# Native replay исходных фрагментов и рёбер · 28.10.2026

Добавлены SourcePoint::requalify, Fragment::requalify, carrier Point::requalify и SharedEdge/ProjectiveEdge::requalify(_with_uses). Исходные уравнения, селекторы, Direct/Mapped-рецепты и рациональные параметрические отображения повторно проходят исходные математические ворота. Кэшированные корни, UV/world-оценки, диапазоны, направления и прежние результаты доказательства не дают разрешения.

Fragment делит mapping-бюджет между двумя crossing-концами; carrier-концы получают непересекающиеся зарезервированные доли mapping/exact/driver/span-бюджетов по прежнему контракту restore. Перед рекурсией запрещены вложенные carrier-родители. На каждом incident use повторно проверяются исходные корни, затем общая композиция и оба конца. SharedEdge/ProjectiveEdge сохраняют единый exact-work и driver-бюджет и прежние пределы. Optional projective-предложения с неконечными числами отвергаются.

ExchangeCarrier::replay_refined для affine и projective рёбер теперь вызывает native replay напрямую вместо definition → изменения JSON → restore. При root refinement подставляются новые исходные uses; world, рецепты отображения, плоскости и кандидаты остаются исходными и заново проверяются. Сериализованная original-метаинформация Prepared ещё остаётся; перенос не означает завершённую сборку без кодека.

Окончательные проверки: **4208 native-тестов прошли, 0 отказов, 6 пропусков** (brep-core 2004/5, nurbs-core 1996/1, cad-predicates 100/0, nurbs-intersect 108/0); обычная dev-сборка прошла. Тесты сравнивают native и прежний wire restore по результатам, причинам отказа и work/root/driver-счётчикам для direct, mapped и projective с рациональными root-кандидатами. Проверены подменённые cached maps, изменённая world-кривая, исчерпание exact-work, свежие source-point bounds, реальные двух-carrier фрагменты и carrier root transport. Существующие native обмен/STEP и body-тесты входят в полный прогон; это не новый WASM/UI runtime-artifact.

Минимальная сборка без кодека пока не проходит: **54 ошибки вместо 55**, включая 53 обращения к value_codec и импорт linear_canal_body в restore-адаптере. Следующий этап — отделить оставшуюся wire-метаинформацию и restore-адаптеры от native типов, сохраняя математические вычисления доступными. Архитектурных превышений 70, лимиты не менялись; API-карта 239/200 актуальна.

Общие Boolean/Solid, центральный G2-угол, offset, упрощение и полный WASM/UI/STEP остаются открытыми. Полный план не завершён. Хэши журналов приведены в matrix.json.
