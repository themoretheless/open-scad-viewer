# Loft: автоматическое согласование, G1/G2 и solid

Пользователь запросил все три расширения после первой версии guided loft.

## Автоматическое согласование

`loft_alignment::interpolate(sections,parameters,guides,budget)` использует bounded curve/curve intersection reports. На каждую пару section/guide требуется ровно одно изолированное пересечение и отсутствие unresolved boxes. Перекрытия, несколько решений, немонотонные пересечения, дубли U-станций и несогласованные U-станции отвергаются.

Направляющие разворачиваются по порядку сечений, сортируются по U и обрезаются между крайними сечениями. Piecewise affine remapping согласует пересечения с authored V-станциями, сохраняя геометрическое место в вещественной арифметике; он может вводить C0-узлы. Whole-curve outward bounds учитывают округление remapping, масштабирования весов и конечного сохранения направляющей. Ошибка сечений также проверяется на полной кривой, а не только в пересечениях. Условия весов остаются совместимыми; произвольные несовместимые рациональные масштабы по разным пересечениям не исправляются.

2..11 sections, 1..9 guides, до 32 контролей по каждой оси итоговой поверхности. Интерпретация budget: модельные длины для сохранения geometry; предварительный U station gate также использует это число, а окончательное принятие требует geometry bounds. Нет поиска произвольной U-перепараметризации сечений. Reports относятся к нормализованным определениям и retained guide portions.

Rust: `loft_alignment::interpolate`; JSON: `surface_auto_guided_loft`; TS: `autoGuidedLoftNurbsCurves`; Rush: `auto_guided_loft_surface`. Пример `examples/rush/auto-guided-loft.r`.

## G1/G2

`loft_continuity::match_ends` согласует один или оба конца нормализованного cubic-or-higher loft с существующими поверхностями. Используется положительный authored normal scale и order 1/2. Scaled homogeneous C1/C2 условия сильнее G1/G2; accepted reports требуют регулярности seam, tangential smoothness и непрерывной оценки всех независимых jet errors.

Согласование basis имеет отдельную whole-surface preparation gate. Три простых, различных V-узла с каждого затронутого конца локализуют изменения до ближайшего внутреннего сечения; они не вводят искусственных C0-изломов в cubic V. Все сечения и направляющие проходят whole-curve retention gate. Несовместимые guide tangents/curvatures вызывают отказ. Оба seam reports повторно вычисляются на конечной поверхности без её изменения, чтобы второй match не оставлял устаревший сертификат первого.

Rust: `loft_continuity::match_ends`; JSON: `surface_loft_match_ends`; TS: `matchNurbsLoftEnds`; Rush: `loft_match_surface`. Пример `examples/rush/g2-loft-surface.r`. Поддерживается reverse seam orientation. Исчерпание 32-control budget вызывает отказ; запрос не превращается в незаявленный approximate fit.

## Замкнутый solid

`brep_core::natural_section_loft` строит полные cubic rational side patches через все authored sections и две плоские крышки. Кривые декомпозируются в согласованные Bezier spans; остаётся контракт одинаковых span degrees/weights и порядка outer/holes. `capped_loft_surfaces(start,end,sides)` принимает готовые нормализованные loft-патчи, включая guided/matched результаты, соответствующие этим граничным spans.

Крышки проходят interval trim-region audit; Builder разделяет общие edges/coedges, model.validate проверяет closed manifold incidence и соответствие pcurves/carriers. Не соответствующие крышкам или соседям side patches отвергаются. Результат имеет body и closed shell. Это не универсальный сертификат глобального embedding для произвольного self-intersecting loft.

JSON bridge: `brep_nurbs_natural_section_loft`, `brep_nurbs_capped_loft_surfaces`; TS: `createNaturalBrepSectionLoft`, `createCappedBrepLoftSurfaces`; Rush: `brep_natural_loft`, `brep_capped_loft`. Пример `examples/rush/natural-loft-solid.r`.

## Проверенные результаты

- Полный native `nurbs-core` прогон: 413 tests passed, включая automatic orientation/order/nonuniform station mapping и отказы.
- Native simultaneous two-end G2 и конфликт направляющей прошли; отдельный тест подтверждает nonzero curvature, dP/dV/d²P/dV² и отсутствие artificial internal C0 knots.
- 11 rational-section BRep tests прошли, включая новую capped natural поверхность, supplied patches, refusal broken boundary, сохранение отверстий и STEP /9 round-trip.
- После последней публикации geometry/language WASM: 37 tests в 8 файлах loft/WASM/Rush + 23 tests в 3 файлах Solid bridge/WASM identity/Brotli прошли. `vue-tsc --noEmit` и scoped `git diff --check` прошли.
- Независимый OpenCascade импорт `natural-hollow-solid.step`: BRepCheck valid, 1 solid, 10 faces (8 cubic curved + 2 cap faces). Midsection radius/height error <=8.89e-16. Adaptive volume integration 687.5599921858749 сравнен с независимой аналитической формулой 687.5599921856518: relative error 3.25e-13.
- Просмотрены screenshots трёх реальных Rush-сборок в редакторе через CPU Mesh fallback; browser pageerror отсутствуют. Native BRep display vertices теперь точно объединяются при переносе в editable Mesh: solid-пример показывает 1116 vertices / 2228 triangles и «замкнут». Авторитетный BRep сохраняется при переносе в Solid, это подтверждено отдельным integration test. GPU qualification не заявлена.

Артефакты: `docs/qualification/advanced-loft-2026-10-02/`; независимая проверка: `scripts/verify-advanced-loft-solid.py`; browser workflow: `scripts/check-advanced-loft-browser.mjs`.

## Независимая публикация от origin/main

Ветка `codex/loft-library` переносит только loft и необходимые базовые конструкторы на API опубликованного main; исходное рабочее дерево с параллельными CAD-изменениями сохранено. Предыдущая квалификация выше относится к исходному рабочему дереву. Проверки этой ветки: полный набор nurbs-core 273 теста, BRep/STEP 11 тестов, modelgraph-text 6 и modelgraph-runtime 54 теста.

Автосогласование теперь разделяет пространственный `budget` (мм) и безразмерный `parameter_tolerance` для станций U (по умолчанию 1e-8, допустимо (0,1]). Rust API: `interpolate_with_parameter_tolerance`; прежний `interpolate` сохраняет сигнатуру и применяет значение по умолчанию. TS использует необязательный пятый аргумент; Rush — именованное поле. Регрессионная проверка подтверждает отказ при разных U-станциях даже с budget=10 мм и отказ от размерного значения parameter_tolerance.

TypeScript/WASM: 42 теста в пяти файлах прошли на release-сборке этой ветки. Rush routing добавлен в language bridge и распознавание исходников редактора.
