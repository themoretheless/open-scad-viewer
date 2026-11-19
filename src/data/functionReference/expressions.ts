import { text, parameter, scad, graph, type ReferenceEntry } from './entry'

export const expressions: readonly ReferenceEntry[] = [
  {
    id: 'power',
    name: 'pow / **',
    category: 'math',
    summary: text('Возводит число в степень.', 'Raises a number to a power.'),
    variants: {
      openscad: scad(
        'pow(base, exponent)',
        [
          parameter('base', 'Основание степени.', 'Base value.'),
          parameter('exponent', 'Показатель степени.', 'Exponent.'),
        ],
        'height = pow(3, 2);\ncube([12, 12, height]);',
      ),
      rush: graph(
        'base ** exponent',
        [
          parameter('base', 'Основание степени.', 'Base value.'),
          parameter('exponent', 'Безразмерный показатель степени.', 'Dimensionless exponent.'),
        ],
        'height = (3 ** 2) * 1mm\nshow box([12mm, 12mm, height])',
      ),
    },
  },
  {
    id: 'min',
    name: 'min',
    category: 'math',
    summary: text('Возвращает наименьшее значение.', 'Returns the smallest value.'),
    variants: {
      openscad: scad(
        'min(a, b, ...) / min(values)',
        [parameter('values', 'Числа для сравнения.', 'Numbers to compare.')],
        'height = min(12, 8, 15);\ncube([20, 12, height]);',
      ),
      rush: graph(
        'values.min()',
        [
          parameter(
            'values',
            'Непустой список чисел с совместимыми единицами.',
            'A nonempty list of numbers with compatible units.',
          ),
        ],
        'height = [12mm, 8mm, 15mm].min()\nshow box([20mm, 12mm, height])',
      ),
    },
  },
  {
    id: 'max',
    name: 'max',
    category: 'math',
    summary: text('Возвращает наибольшее значение.', 'Returns the largest value.'),
    variants: {
      openscad: scad(
        'max(a, b, ...) / max(values)',
        [parameter('values', 'Числа для сравнения.', 'Numbers to compare.')],
        'height = max(12, 8, 15);\ncube([20, 12, height]);',
      ),
      rush: graph(
        'values.max()',
        [
          parameter(
            'values',
            'Непустой список чисел с совместимыми единицами.',
            'A nonempty list of numbers with compatible units.',
          ),
        ],
        'height = [12mm, 8mm, 15mm].max()\nshow box([20mm, 12mm, height])',
      ),
    },
  },
  {
    id: 'sin',
    name: 'sin',
    category: 'math',
    summary: text('Вычисляет синус угла в градусах.', 'Calculates the sine of an angle in degrees.'),
    variants: {
      openscad: scad(
        'sin(angle)',
        [parameter('angle', 'Угол в градусах.', 'Angle in degrees.')],
        'angle = 30;\ntranslate([0, 20 * sin(angle), 0])\n  sphere(r = 4, $fn = 24);',
      ),
    },
  },
  {
    id: 'cos',
    name: 'cos',
    category: 'math',
    summary: text('Вычисляет косинус угла в градусах.', 'Calculates the cosine of an angle in degrees.'),
    variants: {
      openscad: scad(
        'cos(angle)',
        [parameter('angle', 'Угол в градусах.', 'Angle in degrees.')],
        'angle = 60;\ntranslate([20 * cos(angle), 0, 0])\n  sphere(r = 4, $fn = 24);',
      ),
    },
  },
  {
    id: 'abs',
    name: 'abs',
    category: 'math',
    summary: text('Возвращает модуль числа без знака.', 'Returns the absolute value of a number.'),
    variants: {
      openscad: scad(
        'abs(value)',
        [parameter('value', 'Исходное число.', 'Input number.')],
        'height = abs(-8);\ncube([12, 12, height]);',
      ),
    },
  },
  {
    id: 'sqrt',
    name: 'sqrt',
    category: 'math',
    summary: text('Вычисляет квадратный корень.', 'Calculates a square root.'),
    variants: {
      openscad: scad(
        'sqrt(value)',
        [parameter('value', 'Неотрицательное число.', 'A nonnegative number.')],
        'radius = sqrt(100);\nsphere(r = radius, $fn = 32);',
      ),
    },
  },
  {
    id: 'floor',
    name: 'floor',
    category: 'math',
    summary: text('Округляет число вниз до целого.', 'Rounds a number down to an integer.'),
    variants: {
      openscad: scad(
        'floor(value)',
        [parameter('value', 'Число для округления.', 'Number to round.')],
        'count = floor(4.8);\nfor (i = [0 : count - 1])\n  translate([i * 10, 0, 0]) cube([6, 6, 6]);',
      ),
    },
  },
  {
    id: 'ceil',
    name: 'ceil',
    category: 'math',
    summary: text('Округляет число вверх до целого.', 'Rounds a number up to an integer.'),
    variants: {
      openscad: scad(
        'ceil(value)',
        [parameter('value', 'Число для округления.', 'Number to round.')],
        'height = ceil(7.2);\ncube([12, 12, height]);',
      ),
    },
  },
  {
    id: 'round',
    name: 'round',
    category: 'math',
    summary: text('Округляет число до ближайшего целого.', 'Rounds a number to the nearest integer.'),
    variants: {
      openscad: scad(
        'round(value)',
        [parameter('value', 'Число для округления.', 'Number to round.')],
        'height = round(7.8);\ncube([12, 12, height]);',
      ),
    },
  },
  {
    id: 'length',
    name: 'len / length',
    category: 'lists',
    summary: text('Возвращает количество элементов списка.', 'Returns the number of items in a list.'),
    variants: {
      openscad: scad(
        'len(values)',
        [parameter('values', 'Список или строка.', 'A list or string.')],
        'sizes = [4, 6, 8];\ncube([len(sizes) * 5, 10, 4]);',
      ),
      rush: graph(
        'length(values) / values.length()',
        [parameter('values', 'Список, число элементов которого нужно получить.', 'List whose item count is needed.')],
        'sizes = [4mm, 6mm, 8mm]\nshow box([length(sizes) * 5mm, 10mm, 4mm])',
      ),
    },
  },
  {
    id: 'concat',
    name: 'concat',
    category: 'lists',
    summary: text('Соединяет списки, сохраняя порядок элементов.', 'Joins lists while preserving item order.'),
    variants: {
      openscad: scad(
        'concat(list1, list2, ...)',
        [parameter('lists', 'Списки, элементы которых нужно соединить.', 'Lists whose items should be joined.')],
        'positions = concat([0, 12], [24, 36]);\nfor (x = positions)\n  translate([x, 0, 0]) cube([8, 8, 8]);',
      ),
      rush: graph(
        'values.concat(other)',
        [parameter('other', 'Список, добавляемый в конец исходного.', 'List appended to the original list.')],
        'positions = [0mm, 12mm].concat([24mm, 36mm])\nshow positions.select(x => box([8mm, 8mm, 8mm]).translate(x: x))',
      ),
    },
  },
  {
    id: 'at',
    name: '[] / at',
    category: 'lists',
    summary: text('Получает элемент списка по индексу.', 'Gets a list item by its index.'),
    variants: {
      openscad: scad(
        'values[index]',
        [parameter('index', 'Индекс первого элемента равен 0.', 'The first item has index 0.')],
        'heights = [4, 8, 12];\ncube([12, 12, heights[1]]);',
      ),
      rush: graph(
        'values[index] / at(values, index)',
        [
          parameter('values', 'Исходный список.', 'Input list.'),
          parameter(
            'index',
            'Целый индекс от 0 до length(values) - 1.',
            'Integer index from 0 through length(values) - 1.',
          ),
        ],
        'heights = [4mm, 8mm, 12mm]\nshow box([12mm, 12mm, heights[1]])',
      ),
    },
  },
  {
    id: 'range',
    name: 'range / ..',
    category: 'lists',
    summary: text('Создаёт равномерную последовательность значений.', 'Creates an evenly spaced sequence of values.'),
    variants: {
      openscad: scad(
        '[start : step : end]',
        [
          parameter('start / end', 'Начальная и конечная точки.', 'Start and end values.'),
          parameter(
            'step',
            'Шаг; конечная точка входит в диапазон, если шаг её достигает.',
            'Step; the endpoint is included when reached.',
          ),
        ],
        'for (x = [0 : 12 : 36])\n  translate([x, 0, 0]) sphere(r = 4, $fn = 24);',
      ),
      rush: graph(
        'start..end by step / start..<end count n',
        [
          parameter('.. / ..<', 'Включить или исключить конечную точку.', 'Include or exclude the endpoint.'),
          parameter(
            'by / count',
            'Задать шаг или количество элементов; одновременно нельзя.',
            'Set a step or item count, but not both.',
          ),
        ],
        'positions = 0mm..36mm by 12mm\nshow [for x in positions => sphere(4mm).translate(x: x)]',
        text(
          'Для диапазонов с единицами нужно явно указать by или count. Не более 256 элементов.',
          'Ranges with units require explicit by or count. At most 256 items.',
        ),
      ),
    },
  },
  {
    id: 'select',
    name: 'select / map',
    category: 'lists',
    summary: text(
      'Преобразует каждый элемент списка в значение или фигуру.',
      'Transforms each list item into a value or shape.',
    ),
    variants: {
      rush: graph(
        'values.select(function) / values.select(x => expression)',
        [
          parameter(
            'x',
            'Текущий элемент; второй аргумент лямбды может быть индексом.',
            'Current item; an optional second lambda argument is the index.',
          ),
          parameter('expression', 'Результат преобразования элемента.', 'Result of transforming one item.'),
        ],
        'fn column height: length, index: int =>\n  box([8mm,8mm,height]).move(x:index*12mm)\nshow [5mm,10mm,15mm].select(column)',
        text(
          'Можно передать имя функции. После select, возвращающего геометрию, доступны move, rotate, scale и mirror; фильтруйте и сортируйте заранее.',
          'A function name can be passed directly. Geometry results support move, rotate, scale and mirror; filter and sort beforehand.',
        ),
      ),
    },
  },
  {
    id: 'where',
    name: 'where / filter',
    category: 'lists',
    summary: text('Оставляет элементы списка, удовлетворяющие условию.', 'Keeps list items that satisfy a condition.'),
    variants: {
      rush: graph(
        'values.where(x => condition)',
        [
          parameter(
            'condition',
            'Условие: ненулевой результат сохраняет элемент.',
            'Predicate: a nonzero result keeps the item.',
          ),
        ],
        'indices = (0..<6).where(i => i % 2 == 0)\nshow indices.select(i => cylinder(4mm, 8mm).translate(x: i * 12mm))',
      ),
    },
  },
  {
    id: 'enumerate',
    name: 'enumerate',
    category: 'lists',
    summary: text('Добавляет к каждому элементу списка его индекс.', 'Pairs each list item with its index.'),
    variants: {
      rush: graph(
        'enumerate(values) / values.enumerate()',
        [
          parameter(
            'values',
            'Список; результат состоит из пар [индекс, значение].',
            'Input list; produces [index, value] pairs.',
          ),
        ],
        'heights = [5mm, 10mm, 15mm]\nshow [for (i, h) in enumerate(heights) => box([8mm, 8mm, h]).translate(x: i * 12mm)]',
      ),
    },
  },
  {
    id: 'zip',
    name: 'zip',
    category: 'lists',
    summary: text(
      'Сопоставляет элементы нескольких списков по позиции.',
      'Pairs items from several lists by position.',
    ),
    variants: {
      rush: graph(
        'zip(list1, list2, ...)',
        [parameter('lists', 'От 2 до 8 списков одинаковой длины.', '2 to 8 lists of equal length.')],
        'positions = [0mm, 15mm, 30mm]\nheights = [5mm, 10mm, 15mm]\nshow [for (x, h) in zip(positions, heights) => box([8mm, 8mm, h]).translate(x: x)]',
      ),
    },
  },
  {
    id: 'take',
    name: 'take',
    category: 'lists',
    summary: text('Берёт первые элементы списка.', 'Takes the first items from a list.'),
    variants: {
      rush: graph(
        'values.take(count)',
        [parameter('count', 'Неотрицательное целое число элементов.', 'Nonnegative integer number of items.')],
        'positions = [0mm, 12mm, 24mm, 36mm].take(3)\nshow positions.select(x => sphere(4mm).translate(x: x))',
      ),
    },
  },
  {
    id: 'orderBy',
    name: 'orderBy',
    category: 'lists',
    summary: text(
      'Сортирует список по возрастанию вычисленного ключа.',
      'Sorts a list by an ascending calculated key.',
    ),
    variants: {
      rush: graph(
        'values.orderBy(x => key)',
        [parameter('key', 'Ключ сортировки для каждого элемента.', 'Sort key for each item.')],
        'heights = [15mm, 5mm, 10mm].orderBy(h => h)\nshow heights.select((h, i) => box([8mm, 8mm, h]).translate(x: i * 12mm))',
        text(
          'orderByDescending сортирует по убыванию; thenBy добавляет следующий ключ.',
          'orderByDescending sorts descending; thenBy adds a secondary key.',
        ),
      ),
    },
  },
  {
    id: 'sum',
    name: 'sum',
    category: 'math',
    summary: text('Складывает числовые элементы списка.', 'Adds the numeric items of a list.'),
    variants: {
      rush: graph(
        'values.sum()',
        [parameter('values', 'Числовой список с совместимыми единицами.', 'Numeric list with compatible units.')],
        'height = [2mm, 3mm, 5mm].sum()\nshow box([12mm, 12mm, height])',
        text('Для пустого списка результат — безразмерный 0.', 'An empty list returns dimensionless 0.'),
      ),
    },
  },
]
