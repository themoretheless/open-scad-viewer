# Traits с полями и копирование через with

Короткая функция: `fn name arg: Type ret expression`. Скобок вокруг параметров
нет. Вызов по-прежнему использует скобки: `name(value)`. Тип результата можно
указать через `-> Type`, иначе он выводится из возвращаемого значения.
Совместимая старая форма с `=>` остаётся доступна; у лямбд `=>` сохраняется.
В теле через отступ можно вводить локальные имена и завершать его `ret expression`.
Функция без возвращаемого выражения остаётся функцией без результата.

```text
// @modelgraph-text/1
fn plate width: length, height: length = 4mm ret box(width,12mm,height)
show plate(20mm)
```

## Структурное соответствие

Trait описывает необходимые поля. Запись или именованная структура с такими
полями удовлетворяет ему без `impl`. Дополнительные поля разрешены и сохраняются.
Объявление trait должно предшествовать использующей его функции. Для нескольких
ограничений используется `+`.

```text
// @modelgraph-text/1
trait Sized {
  width: length
  depth: length
  height: length
}
trait Positioned { x: length }
fn solid[T: Sized + Positioned] value: T ret
  box(value.width,value.depth,value.height).move(x:value.x)
p = { width: 10mm, depth: 12mm, height: 4mm, x: 5mm, label: "plate" }
show solid(p)
```

## with сохраняет исходный тип

`base with { field: value }` создаёт копию. Исходная запись неизменна.
Обновлять можно только существующие поля, сохраняя их тип. Поля справа вычисляются
в окружающей области видимости; порядок обновлений не создаёт новых локальных имён.
Вложенные записи обновляются явно; автоматического глубокого слияния нет.

```text
// @modelgraph-text/1
trait Sized { width: length, depth: length, height: length }
struct Plate { width: length, depth: length, height: length, label: str }
fn widen[T: Sized] value: T, extra: length -> T
  width = value.width + extra
  ret value with { width: width }
fn solid value: Plate ret box(value.width,value.depth,value.height)
base = Plate { width: 10mm, depth: 12mm, height: 4mm, label: "plate" }
wide = widen(base,5mm)
assert(base.width == 10mm)
assert(wide.label == "plate")
show solid(wide)
```

```text
// @modelgraph-text/1
part = { size: { width: 10mm, depth: 12mm }, height: 4mm }
wide = part with { size: part.size with { width: 20mm } }
show box(wide.size.width,wide.size.depth,wide.height)
```

## Проверки и дальнейшие возможности

Реализованы поля, ограничения generic-параметров и копирование записей с известными
полями. Проверки выполняются при раскрытии вызова в канонический граф; неизвестный
trait отклоняется уже при объявлении функции. Проверки физических единиц и
зависимых от параметров значений сохраняются в графе и повторяются при его вычислении.
Несовместимые ограничения называют trait и поле. Все ограничения обязательны,
в том числе у явно заданного типа и пустого `Vec<T>`.

Методы, `impl`, реализации по умолчанию и связанные типы описаны в
[справочнике методов](modelgraph-text-trait-methods.md). Наследование traits и
trait-объекты пока не поддерживаются. Traits не являются типами значений или
конструкторами. Поля могут использовать связанные типы `Self.Item`; generic-параметры у самих
traits пока не вводятся. Для generic-функций, переданных в динамический callback коллекции,
вывод типа записи ещё не поддерживается; прямые вызовы работают.
