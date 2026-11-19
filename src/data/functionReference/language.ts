import { text, parameter, scad, graph, type ReferenceEntry } from './entry'

export const language: readonly ReferenceEntry[] = [
  {
    id: 'impl',
    name: 'impl / Self',
    category: 'patterns',
    summary: text(
      'Реализует методы trait и задаёт связанные типы для структуры.',
      'Implements trait methods and assigns associated types for a structure.',
    ),
    keywords: ['метод', 'реализация', 'связанный тип', 'default', 'associated', 'method'],
    variants: {
      rush: graph(
        'impl Render for Plate { type Output = Geometry; fn render self: Self ret expression }',
        [
          parameter(
            'Self.Output',
            'Связанный тип результата; в generic-функции доступен как T.Output.',
            'Associated result type; available as T.Output in a generic function.',
          ),
        ],
        'trait Render {\n  type Output\n  fn render self: Self -> Self.Output\n}\nstruct Plate { width: length }\nimpl Render for Plate {\n  type Output = Geometry\n  fn render self: Self ret box(self.width,12mm,4mm)\n}\nshow Plate {width:20mm}.render()',
        text(
          'Метод с телом в trait имеет реализацию по умолчанию. Явный выбор: Render.render(value).',
          'A method body in the trait provides a default implementation. Explicit selection: Render.render(value).',
        ),
      ),
    },
  },
  {
    id: 'with',
    name: 'with',
    category: 'patterns',
    summary: text(
      'Создаёт копию записи с изменёнными полями, сохраняя исходный тип.',
      'Copies a record with changed fields while preserving its type.',
    ),
    variants: {
      rush: graph(
        'record with { field: value }',
        [
          parameter(
            'field',
            'Существующее поле; новое значение должно сохранять его тип.',
            'An existing field; its replacement must preserve its type.',
          ),
        ],
        'base = { width: 10mm, depth: 12mm, height: 4mm }\nwide = base with { width: 20mm }\nshow box(wide.width,wide.depth,wide.height)',
        text(
          'Исходная запись неизменна. Вложенные записи обновляются явно.',
          'The original record is unchanged. Nested updates are explicit.',
        ),
      ),
    },
  },
  {
    id: 'trait',
    name: 'trait',
    category: 'patterns',
    summary: text(
      'Описывает обязательные поля для generic-параметра функции.',
      'Specifies required fields for a function generic parameter.',
    ),
    variants: {
      rush: graph(
        'trait Sized { width: length } / fn widen[T: Sized] value: T ret expression',
        [
          parameter(
            'T: Sized',
            'Запись или структура должна содержать поля trait подходящих типов.',
            'The record or structure must contain the trait fields with compatible types.',
          ),
        ],
        'trait Sized { width: length, depth: length, height: length }\nfn widen[T: Sized] value: T -> T ret value with { width: value.width + 5mm }\np = widen({ width: 10mm, depth: 12mm, height: 4mm })\nshow box(p.width,p.depth,p.height)',
        text(
          'Несколько ограничений: T: Sized + Positioned. Методы используют self: Self; impl задаёт обязательные методы и связанные типы.',
          'Multiple bounds: T: Sized + Positioned. Methods use self: Self; impl supplies required methods and associated types.',
        ),
      ),
    },
  },
  {
    id: 'function',
    name: 'fn',
    category: 'patterns',
    summary: text(
      'Объявляет функцию с аргументами по умолчанию и коротким результатом.',
      'Declares a function with default arguments and a concise result expression.',
    ),
    keywords: ['функция', 'аргументы', 'умолчанию', 'defaults', 'function', 'return', 'тип'],
    variants: {
      rush: graph(
        'fn name argument: Type, optional: Type = value ret expression',
        [
          parameter(
            '= value',
            'Значение пропущенного аргумента. Может использовать предыдущие аргументы.',
            'Value for an omitted argument. May refer to earlier arguments.',
          ),
          parameter(
            'ret expression',
            'Возвращаемое выражение; тип результата можно не указывать.',
            'Returned expression; the result type can be omitted.',
          ),
        ],
        'fn plate width: length, depth: length = width / 2, height: length = 4mm ret\n  box([width,depth,height])\nshow [plate(20mm),plate(width: 20mm,height: 6mm).move(x:25mm)]',
        text(
          'Типы входных аргументов обязательны. Для явного ограничения результата используйте -> Geometry. Именованные аргументы можно передавать в любом порядке; смешивать их с позиционными нельзя.',
          'Input argument types are required. Use -> Geometry for an explicit result constraint. Named arguments can appear in any order but cannot be mixed with positional arguments.',
        ),
      ),
    },
  },
  {
    id: 'destructure',
    name: '[a, b] = values',
    category: 'lists',
    summary: text(
      'Распаковывает список в несколько именованных значений.',
      'Unpacks a list into several named values.',
    ),
    keywords: ['распаковка', 'список', 'массив', 'destructuring', 'unpack'],
    variants: {
      rush: graph(
        '[width, depth, height] = sizes',
        [
          parameter('names', 'Различные имена для элементов списка.', 'Distinct names for the list items.'),
          parameter(
            'sizes',
            'Список с точно таким же числом элементов.',
            'List with exactly the same number of elements.',
          ),
        ],
        'sizes = [[20mm,12mm,4mm],[30mm,16mm,6mm]]\n[width,depth,height] = sizes[1]\nshow box([width,depth,height])',
        text(
          'Работает также в функциях, foreach и блоках match. Длина вычисляемого списка проверяется при выполнении.',
          'Also works in functions, foreach and match blocks. Dynamic list length is checked during evaluation.',
        ),
      ),
    },
  },
  {
    id: 'assert',
    name: 'assert',
    category: 'checks',
    summary: text(
      'Проверяет условие; в RushGraph также проверяет геометрию выбранной детали.',
      'Checks a condition; RushGraph also supports checks on a selected geometry target.',
    ),
    keywords: ['проверка', 'условие', 'ошибка', 'контракт', 'validate', 'validation', 'guard'],
    variants: {
      openscad: scad(
        'assert(condition, "message");',
        [
          parameter('condition', 'Условие, которое должно быть истинным.', 'Condition that must be true.'),
          parameter('message', 'Текст ошибки при нарушении условия.', 'Error text when the condition fails.'),
        ],
        'size = 12;\nassert(size > 0, "Size must be positive");\ncube([size, size, 4]);',
        text(
          'Можно писать отдельной инструкцией или перед фигурой/блоком, в модуле и цикле. Форма assert как часть выражения пока не поддерживается браузерным движком.',
          'Use as a statement or before geometry/a block, inside a module or loop. Expression-form assert is not yet supported by the browser engine.',
        ),
      ),
      rush: graph(
        'assert(condition).message("text") / assert part.isWatertight()',
        [
          parameter(
            'condition',
            'Логическое условие или цепочка сравнений: size.atLeast(1mm).',
            'A predicate or comparison chain, such as size.atLeast(1mm).',
          ),
          parameter(
            'part',
            'Доступная в текущей области фигура, включая промежуточную или скрытую.',
            'Geometry in the current scope, including an intermediate or hidden part.',
          ),
        ],
        'fn plate height: length -> Geometry\n  assert(height > 0mm).message("Height must be positive")\n  body = box([20mm, 12mm, height])\n  assert body.isWatertight()\n  assert measure(body).height.approximately(height, tolerance: 0.01mm)\n  ret body\nshow plate(4mm)',
        text(
          'Доступен на верхнем уровне, в функциях, foreach и блоках match. Проверяются только выполняемые ветки и итерации. Геометрия проверяется при построении: hasBodies, isWatertight, hasNoDegenerateTriangles и measure(part).width/depth/height. own-nurbs пока не поддерживает эти проверки.',
          'Available at the top level, in functions, foreach and match blocks. Only executed branches and iterations are checked. Built geometry supports hasBodies, isWatertight, hasNoDegenerateTriangles and measure(part).width/depth/height. own-nurbs does not yet support these checks.',
        ),
      ),
    },
  },
  {
    id: 'validate',
    name: 'validate',
    category: 'checks',
    summary: text(
      'Проверяет значения и параметры; геометрию проверяют через assert.',
      'Checks values and parameters; use assert to check geometry.',
    ),
    keywords: ['проверка', 'валидация', 'параметр', 'диапазон', 'assert', 'validation', 'constraint'],
    variants: {
      rush: graph(
        'validate value.between(min, max).message("text")',
        [
          parameter(
            'value',
            'Число, величина с единицами или логическое условие.',
            'Number, quantity with units, or predicate.',
          ),
          parameter(
            'min / max',
            'Включительные границы с совместимыми единицами.',
            'Inclusive bounds with compatible units.',
          ),
        ],
        'param gap = 0.3mm range 0mm..1mm\nvalidate gap.between(0.1mm, 0.6mm).message("Gap must be 0.1–0.6 mm")\nshow box([20mm, 12mm, 4mm + gap])',
        text(
          'Для скалярных проверок поведение совпадает с assert; по умолчанию отличается текст ошибки. Разрешён в тех же блоках, что assert. Также: validate(gap > 0mm).',
          'Scalar checks behave like assert; the default error message differs. Allowed in the same blocks as assert. Also: validate(gap > 0mm).',
        ),
      ),
    },
  },
]
