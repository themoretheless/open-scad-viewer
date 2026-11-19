import { text, parameter, scad, graph, type ReferenceEntry } from './entry'

export const shapes: readonly ReferenceEntry[] = [
  {
    id: 'translate',
    name: 'translate',
    category: 'transforms',
    summary: text('Перемещает фигуру по осям X, Y и Z.', 'Moves geometry along the X, Y and Z axes.'),
    keywords: ['перемещение', 'сдвиг', 'смещение', 'move', 'translation', 'position'],
    variants: {
      openscad: scad(
        'translate([x, y, z]) children;',
        [parameter('[x, y, z]', 'Смещение по каждой оси.', 'Displacement along each axis.')],
        'translate([20, 0, 5])\n  cube([10, 10, 10]);',
      ),
      rush: graph(
        'geometry.move(x, y, z) / geometry.move([x, y, z])',
        [
          parameter(
            '[x, y, z]',
            'Расстояния; можно указать mm, cm, m или in.',
            'Distances; mm, cm, m and in units are supported.',
          ),
        ],
        'show box([10mm, 10mm, 10mm]).move([20mm, 0, 5mm])',
        text(
          'Также: geometry.move(x: 20mm, z: 5mm). translate остаётся совместимым именем. Неуказанные оси имеют смещение 0. Можно перемещать коллекцию: [a,b].move(z:10mm); объекты останутся отдельными.',
          'Also: geometry.move(x: 20mm, z: 5mm). translate remains a compatible alias. Omitted axes default to 0. Collections work too: [a,b].move(z:10mm) keeps objects separate.',
        ),
      ),
    },
  },
  {
    id: 'rotate',
    name: 'rotate',
    category: 'transforms',
    summary: text('Поворачивает фигуру вокруг начала координат.', 'Rotates geometry around the origin.'),
    keywords: ['поворот', 'вращение', 'rotation', 'turn', 'angle'],
    variants: {
      openscad: scad(
        'rotate([x, y, z]) children;',
        [
          parameter(
            '[x, y, z]',
            'Углы вокруг осей в градусах; порядок X, затем Y, затем Z.',
            'Angles in degrees; applied around X, then Y, then Z.',
          ),
        ],
        'rotate([0, 0, 30])\n  cube([20, 8, 5]);',
      ),
      rush: graph(
        'geometry.rotate([x, y, z])',
        [parameter('[x, y, z]', 'Углы; поддерживаются deg и rad.', 'Angles; deg and rad units are supported.')],
        'show box([20mm, 8mm, 5mm]).rotate(z: 30deg)',
        text(
          'Можно передать вектор или именованные x, y, z; неуказанные углы равны 0.',
          'Use a vector or named x, y, z arguments; omitted angles default to 0.',
        ),
      ),
    },
  },
  {
    id: 'scale',
    name: 'scale',
    category: 'transforms',
    summary: text('Меняет размер фигуры по каждой оси.', 'Scales geometry independently along each axis.'),
    keywords: ['масштаб', 'масштабирование', 'увеличение', 'уменьшение', 'scaling', 'resize'],
    variants: {
      openscad: scad(
        'scale([x, y, z]) children;',
        [
          parameter(
            '[x, y, z]',
            'Коэффициенты масштаба; 1 сохраняет размер.',
            'Scale factors; 1 keeps the original size.',
          ),
        ],
        'scale([2, 1, 0.5])\n  sphere(r = 10, $fn = 32);',
      ),
      rush: graph(
        'geometry.scale([x, y, z])',
        [parameter('[x, y, z]', 'Безразмерные коэффициенты масштаба.', 'Dimensionless scale factors.')],
        'show sphere(10mm).scale([2, 1, 0.5])',
        text(
          'Именованные оси также поддерживаются: scale(x: 2). Неуказанные коэффициенты равны 1.',
          'Named axes also work: scale(x: 2). Omitted factors default to 1.',
        ),
      ),
    },
  },
  {
    id: 'mirror',
    name: 'mirror',
    category: 'transforms',
    summary: text(
      'Отражает фигуру относительно плоскости через начало координат.',
      'Reflects geometry across a plane through the origin.',
    ),
    keywords: ['отражение', 'зеркало', 'симметрия', 'reflection', 'mirroring', 'symmetry'],
    variants: {
      openscad: scad(
        'mirror([x, y, z]) children;',
        [
          parameter(
            '[x, y, z]',
            'Ненулевой вектор нормали к плоскости отражения.',
            'A nonzero vector normal to the reflection plane.',
          ),
        ],
        'mirror([1, 0, 0])\n  translate([5, 0, 0])\n    cube([12, 8, 5]);',
      ),
      rush: graph(
        'geometry.mirror([x, y, z])',
        [
          parameter(
            '[x, y, z]',
            'Нормаль; [1, 0, 0] отражает относительно плоскости YZ.',
            'Plane normal; [1, 0, 0] reflects across the YZ plane.',
          ),
        ],
        'show box([12mm, 8mm, 5mm]).translate(x: 5mm).mirror([1, 0, 0])',
      ),
    },
  },
  {
    id: 'box',
    name: 'cube / box',
    category: 'primitives',
    summary: text('Создаёт куб или прямоугольный параллелепипед.', 'Creates a cube or rectangular box.'),
    variants: {
      openscad: scad(
        'cube(size = [x, y, z], center = false);',
        [
          parameter('size', 'Размеры по осям или одно число для куба.', 'Axis dimensions, or one number for a cube.'),
          parameter(
            'center',
            'true помещает центр фигуры в начало координат.',
            'true centers the shape at the origin.',
          ),
        ],
        'cube([20, 12, 8], center = true);',
      ),
      rush: graph(
        'box(x, y, z) / box([x, y, z])',
        [parameter('[x, y, z]', 'Три положительных размера.', 'Three positive dimensions.')],
        'show box([20mm, 12mm, 8mm])',
        text(
          'Начальный угол находится в [0, 0, 0]. Для центрирования добавьте translate с половинами размеров со знаком минус.',
          'Starts at [0, 0, 0]. Center it with a translation by negative half-dimensions.',
        ),
      ),
    },
  },
  {
    id: 'sphere',
    name: 'sphere',
    category: 'primitives',
    summary: text('Создаёт сферу с центром в начале координат.', 'Creates a sphere centered at the origin.'),
    variants: {
      openscad: scad(
        'sphere(r = radius, $fn = segments);',
        [
          parameter(
            'r',
            'Положительный радиус; d задаёт диаметр вместо r.',
            'Positive radius; use d instead of r for a diameter.',
          ),
          parameter(
            '$fn',
            'Число сегментов аппроксимации окружности.',
            'Number of segments used to approximate a circle.',
          ),
        ],
        'sphere(r = 10, $fn = 40);',
      ),
      rush: graph(
        'sphere(radius)',
        [parameter('radius', 'Положительный радиус.', 'Positive radius.')],
        'segments 40\nshow sphere(10mm)',
        text(
          'segments задаёт общую детализацию кривых: целое число от 12 до 128.',
          'segments sets shared curve resolution: an integer from 12 to 128.',
        ),
      ),
    },
  },
  {
    id: 'cylinder',
    name: 'cylinder',
    category: 'primitives',
    summary: text('Создаёт цилиндр вдоль оси Z.', 'Creates a cylinder along the Z axis.'),
    variants: {
      openscad: scad(
        'cylinder(h = height, r = radius, center = false);',
        [
          parameter('h', 'Высота.', 'Height.'),
          parameter('r', 'Радиус; r1 и r2 позволяют задать конус.', 'Radius; r1 and r2 can define a cone.'),
          parameter('center', 'Центрировать высоту относительно Z = 0.', 'Center the height about Z = 0.'),
        ],
        'cylinder(h = 20, r = 8, $fn = 40);',
      ),
      rush: graph(
        'cylinder(radius, height)',
        [
          parameter('radius', 'Положительный радиус.', 'Positive radius.'),
          parameter('height', 'Положительная высота от Z = 0.', 'Positive height starting at Z = 0.'),
        ],
        'show cylinder(radius: 8mm, height: 20mm)',
      ),
    },
  },
  {
    id: 'circle',
    name: 'circle',
    category: 'profiles',
    summary: text('Создаёт круглый плоский контур для выдавливания.', 'Creates a circular 2D profile for extrusion.'),
    variants: {
      openscad: scad(
        'circle(r = radius);',
        [parameter('r', 'Радиус; центр находится в [0, 0].', 'Radius; the center is at [0, 0].')],
        'linear_extrude(height = 4)\n  circle(r = 12, $fn = 40);',
      ),
      rush: graph(
        'circle(radius)',
        [
          parameter(
            'radius',
            'Положительный радиус профиля в плоскости XY.',
            'Positive radius of a profile in the XY plane.',
          ),
        ],
        'show circle(12mm).extrude(4mm)',
        text(
          'Перед показом превратите 2D-профиль в тело через extrude или revolve.',
          'Turn a 2D profile into a solid with extrude or revolve before showing it.',
        ),
      ),
    },
  },
  {
    id: 'rectangle',
    name: 'square / rectangle',
    category: 'profiles',
    summary: text('Создаёт прямоугольный плоский контур.', 'Creates a rectangular 2D profile.'),
    variants: {
      openscad: scad(
        'square(size = [x, y], center = false);',
        [
          parameter(
            'size',
            'Ширина и глубина или одно число для квадрата.',
            'Width and depth, or one number for a square.',
          ),
          parameter('center', 'Центрировать контур в начале координат.', 'Center the profile at the origin.'),
        ],
        'linear_extrude(height = 4)\n  square([20, 12], center = true);',
      ),
      rush: graph(
        'rectangle([x, y])',
        [
          parameter(
            '[x, y]',
            'Два положительных размера от угла [0, 0].',
            'Two positive dimensions starting at [0, 0].',
          ),
        ],
        'show rectangle([20mm, 12mm]).extrude(4mm)',
      ),
    },
  },
  {
    id: 'polygon',
    name: 'polygon',
    category: 'profiles',
    summary: text('Строит плоский контур по упорядоченным вершинам.', 'Builds a 2D profile from ordered vertices.'),
    variants: {
      openscad: scad(
        'polygon(points = [[x, y], ...]);',
        [
          parameter(
            'points',
            'Вершины границы; последний отрезок замыкается автоматически.',
            'Boundary vertices; the final edge closes automatically.',
          ),
        ],
        'linear_extrude(height = 5)\n  polygon(points = [[0, 0], [20, 0], [10, 15]]);',
      ),
      rush: graph(
        'polygon([[x, y], ...])',
        [
          parameter(
            'points',
            'От 3 до 256 вершин простого контура без самопересечений.',
            '3 to 256 vertices of a simple, non-self-intersecting profile.',
          ),
        ],
        'show polygon([[0, 0], [20mm, 0], [10mm, 15mm]]).extrude(5mm)',
      ),
    },
  },
]
