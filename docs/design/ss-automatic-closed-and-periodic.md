# Автоматические SS-пересечения: замкнутые ветви и periodic/poles

3 ноября 2026. Собственные Rust-библиотеки; это ограниченные доказанные семейства, а не завершение общих NURBS-пересечений.

## Общий API

`nurbs_core::ss_intersection::construct_qualified_ss_branches(&sources, limits)` возвращает собственника исходных определений с `reverify()` и `covers_entire_surfaces()`.

Варианты результата:

- `ClosedOrDegenerateGraph`: ранее поддержанные точные quadratic/rational graph-семейства.
- `RegularOpenArcs`: полный original-root atlas, component census и физические открытые ветви.
- `RegularClosedLoops`: полный mixed-axis census, component transitions, C1/rank, отсутствие контактов с границей параметрического произведения и глобальная инъективность обеих опор.
- `IsolatedGraphContact`: изолированное касание, найденное из исходных коэффициентов, с независимой точной проверкой всего графового зазора.
- `PeriodicPlaneSection`: полное сечение исходной periodic homothetic поверхности конечной координатной плоскостью, включая collapsed pole.

`QualifiedSSBuildLimits.closed` и `.periodic` задают отдельные конечные allowances. Исчерпание уже выполненной quadratic-проверки не маскируется fallback. Доказанный even-power gap порядка выше двух заранее исключает пару quadratic height graphs и выбирает собственное точное семейство без такой попытки.

## Несколько скрытых замкнутых ветвей

Численный Newton используется только для предложений. Полнота принадлежит повторяемому покрытию исходного параметрического произведения. Mixed charts могут иметь разные свободные координаты; charts не склеиваются по расстоянию между world points.

Для непрерывных polynomial B-spline graphs поддержана точная двумерная редукция:

- один глобальный affine XY frame проверяется через исходные Greville-коэффициенты обеих опор;
- положительные постоянные весовые gauges каждой опоры могут различаться;
- каждый общий knot-прямоугольник получается точным rational blossom restriction исходного Cox-базиса;
- все замкнутые прямоугольники входят в один forest; work и cell allowance общие, replay восстанавливает весь исходный forest;
- degree 1..8, не более 128 controls по оси и 64 общих knot-прямоугольников; normalized common knots должны точно представляться в binary64. Другие случаи остаются на общем 4D-пути, без автоматического допуска;
- покрытие каждого листа основано на точном исключении, сертифицированном chart или точной derivative/root-image оценке.

Перекрывающиеся charts уплотняются по целым сертифицированным параметрическим root enclosures. Более сильная compaction проверяет четыре замкнутых free-подынтервала свежими исходными операторами. Обе процедуры освобождают места для предложений; итоговый независимый census всё равно обязателен.

`max_charts` остаётся не более 64. `proposal_rectangles` общий для начальных предложений, repair и compaction, без сброса. Максимальное допустимое `max_repairs` расширено с 8 до 64; default остаётся 4. Каждый repair запускает новый полный census в своём явно заданном coverage allowance, как и раньше. Квалификация двух отдельных овалов использует caller `max_repairs=32`, 64 charts и прежние 100000 proposal rectangles; это не performance benchmark.

## Изолированные касания

`discover_isolated_graph_contact` выводит центр, чётные степени 2/4/6/8 и коэффициенты из исходного Bernstein gap. Затем `qualify_isolated_graph_contact` проверяет полное точное тождество

`z_second - z_first = c0*(u-a)^even + c1*(v-b)^even`.

Оба ненулевых коэффициента имеют одинаковый знак. Совпадающий глобальный affine XY frame инъективен; поэтому нулевое множество ровно одна точка. Проверяются смещённые центры, разные natural domains, gauges и перестановка операндов. Смешанные остатки, ненулевой зазор и повреждённый XY frame отвергаются. Текущая спецификация требует точно представимых binary64 center/coefficient; произвольные algebraic centers, касательные линии и сети этим семейством не закрыты.

## Periodic и полюса

`qualify_periodic_plane_ss` использует независимую исходную проверку seam identity, Jordan-профиля, homothetic quotient и regular interior. Флаги periodic не снимаются; исходные controls/weights/knots сохраняются.

Поддержана одна periodic ось и degree-1 transverse homothety с положительным постоянным отношением row gauges, а также конечный affine rectangle в координатной плоскости. Весь точный level contour должен строго помещаться в rectangle; частичное clipping отказывается.

Результаты различаются:

- `ClosedContour`: замкнутый embedded профиль на точном transverse level;
- `CollapsedPoleContact`: ровно одна исходная world-точка; нулевое ребро не объявляется замкнутым контуром;
- `Disjoint`: полный отказ контакта по исходному height interval.

`transverse_parameter`, `level_control_points` и `level_weights` сохраняются как точные rational values. Например, natural UV = 2/3 не округляется до f64. Эти коэффициенты используют неизменённый original periodic basis. Конвертация результата в существующее f64 curve storage требует отдельной downstream-квалификации.

## Граница результата и следующий обязательный объём

`RegularClosedLoops.trims()` — процедурные исходные SS-границы на обеих опорах. Это ещё не сохранённые BRep loops и не полное удаление/сохранение материала. Новые native APIs не получили bridge/WASM/editor-адаптер; прежняя runtime-квалификация других операций на них не переносится.

Открыты: произвольные рациональные/multispan SS-ветви и singular networks, общий periodic quotient, произвольные полюса, частичное finite-face clipping, согласованные world/pcurve traces через nonlinear conditioning, полное автоматическое BRep trimming, material/orientation/healing и общий Boolean замкнутых тел. Ни один из этих пунктов не закрывается поддержанными семействами выше.

Результаты проверок: [receipt](../qualification/ss-closed-periodic-2026-11-03/results.json). Общий остаток: [Plasticity parity](plasticity-remaining-2026-11-02.md).
