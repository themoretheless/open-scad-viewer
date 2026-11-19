import { text, parameter, scad, graph, type ReferenceEntry } from './entry'

export const operations: readonly ReferenceEntry[] = [
  {
    id: 'union',
    name: 'union',
    category: 'booleans',
    summary: text('Объединяет фигуры в общую геометрию.', 'Combines shapes into one geometry result.'),
    variants: {
      openscad: scad(
        'union() { children; }',
        [
          parameter(
            'children',
            'Фигуры одной размерности: только 2D или только 3D.',
            'Shapes of the same dimension: all 2D or all 3D.',
          ),
        ],
        'union() {\n  cube([20, 10, 6]);\n  translate([10, 0, 0]) cylinder(h = 10, r = 6, $fn = 32);\n}',
      ),
      rush: graph(
        'union(part1, part2, ...)',
        [
          parameter(
            'parts',
            'Несколько фигур или коллекция одной размерности.',
            'Multiple shapes or a collection of the same dimension.',
          ),
        ],
        'base = box([20mm, 10mm, 6mm])\npost = cylinder(6mm, 10mm).translate(x: 10mm)\nshow union(base, post)',
        text(
          'show [a, b] сохраняет отдельные объекты; union(a, b) выполняет геометрическое объединение.',
          'show [a, b] keeps objects separate; union(a, b) performs a geometric union.',
        ),
      ),
    },
  },
  {
    id: 'difference',
    name: 'difference / subtract',
    category: 'booleans',
    summary: text('Вырезает отверстия и выемки из исходной фигуры.', 'Cuts holes and recesses out of a base shape.'),
    variants: {
      openscad: scad(
        'difference() { base; cutters; }',
        [
          parameter('base', 'Первая дочерняя фигура — исходное тело.', 'The first child is the base shape.'),
          parameter(
            'cutters',
            'Последующие фигуры вычитаются из первой.',
            'Subsequent children are subtracted from the first.',
          ),
        ],
        'difference() {\n  cube([20, 20, 8]);\n  translate([10, 10, -1]) cylinder(h = 10, r = 4, $fn = 32);\n}',
      ),
      rush: graph(
        'base.subtract(cutter1, cutter2, ...)',
        [
          parameter('base', 'Исходное тело или профиль перед точкой.', 'The base solid or profile before the dot.'),
          parameter('cutters', 'Вычитаемые фигуры той же размерности.', 'Shapes of the same dimension to subtract.'),
        ],
        'base = box([20mm, 20mm, 8mm])\nhole = cylinder(4mm, 10mm).translate([10mm, 10mm, -1mm])\nshow base.subtract(hole)',
      ),
    },
  },
  {
    id: 'intersection',
    name: 'intersection',
    category: 'booleans',
    summary: text('Оставляет только общую часть фигур.', 'Keeps only the overlapping region of shapes.'),
    variants: {
      openscad: scad(
        'intersection() { children; }',
        [
          parameter(
            'children',
            'Пересекающиеся фигуры одной размерности.',
            'Overlapping shapes of the same dimension.',
          ),
        ],
        'intersection() {\n  cube([20, 20, 20], center = true);\n  sphere(r = 13, $fn = 32);\n}',
      ),
      rush: graph(
        'intersection(part1, part2, ...)',
        [
          parameter(
            'parts',
            'Фигуры, общую область которых нужно оставить.',
            'Shapes whose common region should remain.',
          ),
        ],
        'block = box([20mm, 20mm, 20mm]).translate([-10mm, -10mm, -10mm])\nshow intersection(block, sphere(13mm))',
      ),
    },
  },
  {
    id: 'hull',
    name: 'hull',
    category: 'booleans',
    summary: text(
      'Создаёт выпуклую оболочку вокруг нескольких фигур.',
      'Creates the convex hull around several shapes.',
    ),
    variants: {
      openscad: scad(
        'hull() { children; }',
        [
          parameter(
            'children',
            'Фигуры, между которыми будет заполнено пространство.',
            'Shapes whose convex connecting region will be filled.',
          ),
        ],
        'hull() {\n  cylinder(h = 4, r = 5, $fn = 32);\n  translate([25, 0, 0]) cylinder(h = 4, r = 5, $fn = 32);\n}',
      ),
      rush: graph(
        'hull(part1, part2, ...)',
        [parameter('parts', 'Фигуры одной размерности.', 'Shapes of the same dimension.')],
        'left = circle(5mm)\nright = circle(5mm).translate(x: 25mm)\nshow hull(left, right).extrude(4mm)',
      ),
    },
  },
  {
    id: 'extrude',
    name: 'linear_extrude / extrude',
    category: 'profiles',
    summary: text('Выдавливает плоский контур вдоль оси Z.', 'Extrudes a 2D profile along the Z axis.'),
    variants: {
      openscad: scad(
        'linear_extrude(height, center = false, twist = 0) children;',
        [
          parameter('height', 'Положительная высота выдавливания.', 'Positive extrusion height.'),
          parameter('center', 'Центрировать высоту относительно Z = 0.', 'Center the height about Z = 0.'),
          parameter('twist', 'Поворот верхнего сечения в градусах.', 'Rotation of the top section in degrees.'),
        ],
        'linear_extrude(height = 20, twist = 30, slices = 20)\n  square([10, 6], center = true);',
      ),
      rush: graph(
        'profile.extrude(height)',
        [parameter('height', 'Положительная высота от плоскости XY.', 'Positive height from the XY plane.')],
        'show rectangle([20mm, 12mm]).extrude(8mm)',
        text(
          'В компактном Text/1 эта операция принимает только высоту.',
          'In compact Text/1 this operation accepts only the height.',
        ),
      ),
    },
  },
  {
    id: 'revolve',
    name: 'rotate_extrude / revolve',
    category: 'profiles',
    summary: text(
      'Вращает плоский контур вокруг оси Z, создавая тело.',
      'Revolves a 2D profile around the Z axis to create a solid.',
    ),
    variants: {
      openscad: scad(
        'rotate_extrude(angle = 360) children;',
        [parameter('angle', 'Угол вращения в градусах.', 'Revolution angle in degrees.')],
        'rotate_extrude(angle = 360, $fn = 48)\n  translate([15, 0]) circle(r = 4, $fn = 24);',
        text(
          'X профиля задаёт радиус, Y — высоту. Держите профиль по одну сторону оси.',
          'Profile X is the radius and Y is the height. Keep the profile on one side of the axis.',
        ),
      ),
      rush: graph(
        'profile.revolve(angle)',
        [parameter('angle', 'Положительный угол не больше 360deg.', 'Positive angle no greater than 360deg.')],
        'show circle(4mm).translate(x: 15mm).revolve(360deg)',
        text(
          'X профиля задаёт радиус, Y — высоту. Профиль не должен пересекать ось вращения.',
          'Profile X is the radius and Y is the height. The profile must not cross the rotation axis.',
        ),
      ),
    },
  },
  {
    id: 'offset',
    name: 'offset',
    category: 'profiles',
    summary: text('Расширяет или сужает плоский контур.', 'Expands or contracts a 2D profile.'),
    variants: {
      openscad: scad(
        'offset(r = distance) children; / offset(delta = distance) children;',
        [
          parameter('r', 'Отступ со скруглёнными внешними углами.', 'Offset with rounded outer corners.'),
          parameter('delta', 'Отступ с сохранением острых углов.', 'Offset preserving sharp corners.'),
        ],
        'linear_extrude(height = 4)\n  offset(r = 2) square([20, 12]);',
      ),
      rush: graph(
        'profile.offset(distance) / profile.offset(delta: distance)',
        [
          parameter(
            'distance',
            'Положительное значение расширяет контур, отрицательное сужает.',
            'Positive values expand the profile; negative values contract it.',
          ),
          parameter('delta', 'Именованный аргумент выбирает острые углы.', 'The named argument selects sharp corners.'),
        ],
        'show rectangle([20mm, 12mm]).offset(2mm).extrude(4mm)',
      ),
    },
  },
  {
    id: 'repeat',
    name: 'for / repeat',
    category: 'patterns',
    summary: text(
      'Повторяет фигуру с вычисляемым положением или размером.',
      'Repeats geometry with a calculated position or size.',
    ),
    variants: {
      openscad: scad(
        'for (i = [start : step : end]) children;',
        [
          parameter('i', 'Переменная текущей итерации.', 'Current iteration variable.'),
          parameter(
            '[start : step : end]',
            'Диапазон с включённой конечной точкой, если шаг её достигает.',
            'Range including its endpoint when the step reaches it.',
          ),
        ],
        'for (i = [0 : 4])\n  translate([i * 12, 0, 0])\n    cylinder(h = 8, r = 4, $fn = 24);',
      ),
      rush: graph(
        'repeat(count, function) / repeat(count, i => geometry)',
        [
          parameter('count', 'Целое число повторений от 1 до 256.', 'Integer repetition count from 1 to 256.'),
          parameter('i', 'Индекс от 0 до count - 1.', 'Index from 0 through count - 1.'),
        ],
        'fn pin index: int => cylinder(4mm,8mm).move(x:index*12mm)\nshow repeat(5,pin)',
        text(
          'repeat объединяет результаты. Для отдельных объектов используйте список с for или select.',
          'repeat unions its results. Use a list comprehension or select for separate objects.',
        ),
      ),
    },
  },
  {
    id: 'conditional',
    name: '? :',
    category: 'patterns',
    summary: text('Выбирает значение по условию.', 'Chooses a value according to a condition.'),
    variants: {
      openscad: scad(
        'condition ? when_true : when_false',
        [
          parameter('condition', 'Логическое выражение.', 'Boolean expression.'),
          parameter(
            'when_true / when_false',
            'Значения для двух исходов условия.',
            'Values for the two possible outcomes.',
          ),
        ],
        'large = true;\nradius = large ? 12 : 6;\nsphere(r = radius, $fn = 32);',
      ),
      rush: graph(
        'condition ? when_true : when_false',
        [
          parameter(
            'condition',
            '0 означает ложь, ненулевое значение — истину.',
            '0 is false; a nonzero value is true.',
          ),
          parameter(
            'when_true / when_false',
            'Два значения либо две геометрические фигуры.',
            'Two values or two geometry expressions.',
          ),
        ],
        'param rounded = 1 range 0..1\nshow rounded ? sphere(10mm) : box([20mm, 20mm, 20mm])',
      ),
    },
  },
]
