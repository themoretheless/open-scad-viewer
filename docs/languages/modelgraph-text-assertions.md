# Проверки в ModelGraph Text

`assert` можно писать отдельным оператором на верхнем уровне, внутри функций,
в блоке `foreach` и в блоке ветки `match`. Внутри блока соблюдайте его отступ;
проверки пишутся до завершающего `ret` или `yield`. Переменная должна быть уже
объявлена в доступной области видимости.

`validate` поддерживает проверки значений. `assert` поддерживает те же проверки
значений и дополнительно проверки геометрии.

## Предикаты и сообщения

Можно использовать сравнение или логическое выражение. Скобки необязательны;
перед `.message(...)` составное выражение нужно заключить в скобки. Как в
условных выражениях, безразмерный ноль означает ложь, ненулевое значение — истину.

```text
// @modelgraph-text/1
param width = 20mm range 0mm..50mm
assert(width > 0mm).message("Ширина должна быть положительной")
validate width.atMost(40mm).message("Ширина должна быть не больше 40 мм")
assert width > 0mm && width <= 40mm
show box([width,10mm,5mm])
```

Для чисел также доступны `greaterThan`, `atLeast`, `lessThan`, `atMost`,
`equalTo`, `between` и `approximately`. Сравниваемые величины должны иметь
совместимые единицы. `between` включает обе границы. Например,
`assert width.approximately(20mm, tolerance: 0.1mm)`.

## Функции с фигурными скобками

Проверки используют аргументы и ранее объявленные локальные значения.

```text
// @modelgraph-text/1
make = fn height: length -> Geometry {
  assert height.greaterThan(0mm)
  doubled = height * 2
  validate(doubled <= 20mm).message("Удвоенная высота превышает 20 мм")
  ret box([5mm,5mm,doubled])
}
show make(4mm)
```

## Функции с отступами и функции без результата

У функции без результата нет `->`; её можно вызвать отдельной строкой.
Она заканчивается при уменьшении отступа или явным `ret` без значения.

```text
// @modelgraph-text/1
fn requirePositive value: length
  assert(value > 0mm).message("Размер должен быть положительным")

fn make height: length -> Geometry
  requirePositive(height)
  body = box([5mm,5mm,height])
  assert body.hasBodies(1).isWatertight()
  ret body

show make(4mm)
```

Само объявление функции не запускает её проверки. Проверки вызванной функции
сохраняются, даже если её результат не используется в `show`. Если вызов находится в
невыбранной ветке `condition ? a : b` или `match`, его проверки не выполняются.
То же относится к вызову функции внутри callback, например
`repeat(3, i => make((i + 1) * 1mm))`.

## Циклы `foreach`

Проверки выполняются для текущего элемента в порядке операторов. После
`continue` оставшиеся операторы этой итерации пропускаются. После `break`
пропускаются оставшиеся операторы и все следующие элементы. Проверка перед
`break` относится и к элементу, на котором цикл заканчивается.

```text
// @modelgraph-text/1
parts = foreach height in [1mm,0mm,2mm,99mm,-1mm]
  continue if height == 0mm
  break if height == 99mm
  assert(height > 0mm).message("Высота детали должна быть положительной")
  local = box([1mm,1mm,height])
  assert measure(local).height.approximately(height)
  yield local.translate([height*3,0,0])
show parts
```

Этот пример строит две детали. Значения `0mm`, `99mm` и `-1mm` не доходят до
проверки высоты. Проверки внутри функций, вызванных из `select`, `repeat` или
`foreach`, используют значения соответствующего элемента.

## Ветки `match`

Блок ветки содержит проверки, локальные присваивания и завершающий `ret`.
Проверки других веток не выполняются.

```text
// @modelgraph-text/1
param kind = 1 range 0..1
body = match kind
  1 =>
    height = 4mm
    assert height.atLeast(1mm)
    local = box([5mm,5mm,height])
    assert local.isWatertight()
    ret local
  _ =>
    assert false.message("Для этого варианта деталь ещё не определена")
    ret box([1mm,1mm,1mm])
show body
```

При `kind = 1` пример работает. При `kind = 0` появляется указанное сообщение.

## Промежуточная и скрытая геометрия

Проверка относится именно к указанной геометрии, даже если затем её преобразуют
или выводят другую модель. Необязательно помещать проверку после `show`.

```text
// @modelgraph-text/1
part = box([2mm,3mm,4mm])
assert part.hasBodies(1).isWatertight().hasNoDegenerateTriangles()
assert measure(part).height.approximately(4mm, tolerance: 0.01mm)
show part.scale([1,1,2])
```

Проверяется высота `part` в 4 мм; показанная модель имеет высоту 8 мм.
Можно проверить и локальную геометрию функции, возвращающей число:

```text
// @modelgraph-text/1
fn doubledHeight height: length -> length
  sample = box([2mm,3mm,height])
  assert sample.isWatertight()
  assert measure(sample).height.approximately(height)
  ret height * 2

show box([1mm,1mm,doubledHeight(4mm)])
```

`measure(...).width`, `.depth` и `.height` измеряют габариты вдоль мировых осей
X, Y и Z. Это размеры ограничивающего параллелепипеда в миллиметрах.
`hasBodies(n)` считает связные компоненты треугольной поверхности;
`isWatertight()` проверяет её замкнутость и согласованную ориентацию.

Проверки значений выполняются при вычислении соответствующего участка модели.
Геометрические проверки выполняются после построения их цели. Компиляция в
ModelGraph сама по себе сохраняет и подготавливает эти проверки, но не строит
треугольную поверхность и не подтверждает результат измерения. Команда
«Собрать» выполняет проверки построенной геометрии; нарушение или невозможность
измерения считается ошибкой.

Геометрические проверки пока недоступны для backend `own-nurbs`, например для
`brep_box(...).brep_tessellate(...)` и операций NURBS.
