# Методы traits, реализации по умолчанию и связанные типы

Метод объявляется через `fn` без скобок вокруг параметров. Первым параметром
идёт `self: Self`; у сигнатуры trait явно указан тип результата. Вызов
`value.method(args)` автоматически передаёт `self`. Метод может содержать
локальные имена и проверки, как обычная функция.

## Обязательный метод и связанный тип

`type Output` объявляет связанный тип. `impl Render for Plate` задаёт его и
реализует обязательные методы. Внутри trait используются `Self` и `Self.Output`,
в generic-функции — `T.Output`. Конкретный тип `Plate` сохраняется через `with`.

```text
// @modelgraph-text/1
trait Render {
  type Output
  fn render self: Self -> Self.Output
}
struct Plate { width: length, depth: length, height: length }
impl Render for Plate {
  type Output = Geometry
  fn render self: Self ret box(self.width,self.depth,self.height)
}
fn draw[T: Render] value: T -> T.Output ret value.render()
p = Plate { width: 10mm, depth: 12mm, height: 4mm }
show draw(p with { width: 20mm })
```

## Реализация по умолчанию

Метод с телом в trait получает реализацию по умолчанию. Он может вызывать
другие методы через `self`; при наличии переопределения вызывается версия из
`impl`. Если у trait только поля и методы с телами, структурно подходящей
записи не нужен отдельный `impl`.

```text
// @modelgraph-text/1
trait Sized {
  width: length
  fn size self: Self -> length ret self.width
  fn solid self: Self -> Geometry
    assert(self.size() > 0mm)
    ret box(self.size(),12mm,4mm)
}
struct Plate { width: length }
impl Sized for Plate {
  fn size self: Self ret self.width + 5mm
}
p = Plate { width: 10mm }
show [p.solid(), {width:8mm}.solid().move(25mm,0,0)]
```

## Связанный тип в аргументах и полях

```text
// @modelgraph-text/1
trait Value {
  type Item
  value: Self.Item
  fn replace self: Self, next: Self.Item -> Self ret self with {value:next}
}
struct Radius { value: length }
impl Value for Radius { type Item = length }
fn change[T: Value] value: T, next: T.Item -> T ret value.replace(next)
p = change(Radius {value:2mm},5mm)
show sphere(p.value)
```

## Явный выбор trait

Внутри метода предпочтение имеет его trait. В generic-функции учитываются
объявленные ограничения. Если остаётся несколько подходящих методов, компилятор
сообщает неоднозначность. Явная форма `Trait.method(receiver, args)` выбирает
нужный контракт; первый аргумент в этой форме всегда позиционный.

```text
// @modelgraph-text/1
trait Small { fn solid self: Self -> Geometry ret box(5,5,5) }
trait Large { fn solid self: Self -> Geometry ret box(10,10,10) }
p = {}
show [Small.solid(p), Large.solid(p).move(20,0,0)]
```

Реализации сохраняют область видимости своего объявления; методы по умолчанию —
область объявления trait. Аргументы могут иметь значения по умолчанию и передаваться
по имени. Имена и типы параметров в `impl` должны соответствовать контракту;
тип результата реализации можно вывести из тела, но он проверяется по сигнатуре trait.

`impl` предназначен для именованных структур, включая конкретные специализации
вроде `Holder<length>`. Все связанные типы и обязательные методы должны быть заданы.
Повторный `impl` того же trait для того же типа, неизвестные члены и несовместимые
сигнатуры отклоняются при объявлении. Связанные типы могут ссылаться друг на друга
через `Self.Name`, независимо от порядка; циклы запрещены.

Проверки значений и результатов выполняются при вызове и сохраняются в каноническом
графе. Раскрытие методов ограничено теми же лимитами, что обычные функции.
Неиспользуемые тела не выполняются. Существующие физические единицы, assertions и
пересчёт параметров сохраняются.

Пока нет generic-методов, blanket `impl<T>`, наследования traits, trait-объектов,
связанных типов с собственными параметрами и значений связанных типов по умолчанию.
Методы этого этапа имеют один результат с явным типом в контракте. Вывод типа
записи для dynamic generic callbacks коллекций остаётся отдельным ограничением.
