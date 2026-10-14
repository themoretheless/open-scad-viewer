# Проверки самопересечений кривых и поверхностей

Оба API — консервативные проверки с тремя исходами Absent/Present/Unresolved. Absent означает математическое доказательство для всей исходной области, Present — доказанное совпадение различных параметров или собственное пересечение, Unresolved — отсутствие полного доказательства. Неопределённые случаи не считаются отсутствием пересечений. Источники неизменны. Дегенеративное многократное покрытие также считается Present; это не классификация транверсального пересечения.

## Кривая: N166

`curve_self_intersection::inspect(curve,tolerance,max_cells,max_pairs)` принимает валидную 2D/3D positive-weight NURBS, finite positive tolerance и budgets 1..100000. Глобальная строгая монотонность одной координаты по outward rational jets доказывает инъективность непрерывной непериодической кривой. Не более четверти interval budget резервируется на поиск такой координаты. Разрывы базиса не допускают этот вывод.

Для C0 непрерывной цепочки исходных Bezier knot spans проводится полный pair audit. Монотонные, не постоянные координаты control polygons с положительными весами доказывают локальную инъективность каждого span (производная rational Bernstein — положительно взвешенная covariance индекса и монотонных controls). Исходные control hulls дают точное разделение, включая строгие открытые полупространства при единственной общей плоскости. Все endpoint coincidences проверяются отдельно: shared parameter и нормальный closure endpoint исключаются; остальные являются self-contact witness. Неразделённые пары проверяются исходными outward rational enclosures с subdivision; не округлёнными копиями trim. Отдельные локальные успехи без полного покрытия всех пар не дают Absent.

Для прямолинейных planar spans proper chord intersection даёт строгий interior witness; рациональные positive weights изменяют параметризацию, но сохраняют геометрический отрезок. Полная trim-simplicity proof дополнительно поддерживает замкнутые planar degree-one контуры. Криволинейные контакты, не доказанные этим bounded audit, и periodic charts остаются Unresolved; это не универсальный классификатор кратности касаний. Нормальный шов замкнутого контура не является самопересечением. cells и pairs учитывают общую работу; witness_point — округлённая конструкция уже доказанного chord crossing, а не sample-based решение.

4 независимых теста: рациональная пространственная graph curve; closed rational circle с покрытием всех curved span pairs; bow-tie proper intersection в известной точке [0.5,0.5]; замкнутый прямоугольник без self-intersection; insufficient work и invalid input. Обе ветви решения и отказ в ложном сертификате проверены.

## Поверхность: N167

`surface_self_intersection::inspect(surface,max_spans)` принимает исходную tensor positive-weight NURBS и budget 1..100000. Для Absent используется sufficient global injectivity certificate: одна постоянная двумерная проекция F и матрица Y удовлетворяют ||I-Y DF||∞<1 на всём convex parameter rectangle. Derivative hull включает все оригинальные knot rectangles и outward arithmetic. Локально инъективные участки не заменяют глобальный тест.

Present доказывается точной отражательной симметрией CP и весов целой Bezier оси: homogeneous Bernstein identity S(u,v)=S(1-u,v) либо S(u,v)=S(u,1-v). Witness — две различные interior normalized points 1/4 и 3/4; они интерпретируются точным математическим active-domain affine map. Это доказанное двухкратное покрытие, не округлённое совпадение samples. Неподдержанные periodic charts, исчерпание span coverage и отсутствие доказательства одного из исходов сохраняют Unresolved. Для arbitrary shapes строгий достаточный тест может не разрешиться даже при фактической простоте; отсутствие сертификата не утверждает наличие дефекта.

3 интеграционных теста: нелинейная spatial graph surface и её вертикальный поворот; точный fold с различными interior witness и аналитически известными координатами; неполное покрытие, budget=0 и NaN weights. Дополнительно 6 surface_injectivity tests проверяют rational derivatives, несколько spans, обе стороны узла, collapse/fold и общий contraction test.
