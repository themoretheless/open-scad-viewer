export type ReferenceLanguage = 'openscad' | 'modelgraph'
export type ReferenceLocale = 'ru' | 'en'
export type ReferenceCategory = 'transforms' | 'primitives' | 'booleans' | 'profiles' | 'patterns' | 'checks' | 'math' | 'lists'
export type ReferenceText = Readonly<Record<ReferenceLocale, string>>

export interface ReferenceVariant {
  readonly signature: string
  readonly parameters: readonly { readonly name: string; readonly description: ReferenceText }[]
  /** A complete scene, including the routing header for ModelGraph Text. */
  readonly example: string
  readonly notes?: ReferenceText
}

export interface ReferenceEntry {
  readonly id: string
  readonly name: string
  readonly category: ReferenceCategory
  readonly summary: ReferenceText
  readonly keywords?: readonly string[]
  readonly variants: Readonly<Partial<Record<ReferenceLanguage, ReferenceVariant>>>
}

const text = (ru: string, en: string): ReferenceText => ({ ru, en })
const parameter = (name: string, ru: string, en: string) => ({ name, description: text(ru, en) })
const scad = (signature: string, parameters: ReferenceVariant['parameters'], example: string, notes?: ReferenceText): ReferenceVariant => ({ signature, parameters, example, ...(notes ? { notes } : {}) })
const graph = (signature: string, parameters: ReferenceVariant['parameters'], example: string, notes?: ReferenceText): ReferenceVariant => scad(signature, parameters, `// @modelgraph-text/1\n${example}`, notes)

export const REFERENCE_CATEGORIES: readonly { readonly id: ReferenceCategory; readonly label: ReferenceText }[] = [
  { id: 'transforms', label: text('Преобразования', 'Transforms') },
  { id: 'primitives', label: text('Объёмные фигуры', 'Solid primitives') },
  { id: 'booleans', label: text('Объединение и вычитание', 'Boolean operations') },
  { id: 'profiles', label: text('Контуры и выдавливание', 'Profiles and extrusion') },
  { id: 'patterns', label: text('Повторение и условия', 'Patterns and conditions') },
  { id: 'checks', label: text('Проверки', 'Checks') },
  { id: 'math', label: text('Математика', 'Math') },
  { id: 'lists', label: text('Списки', 'Lists') },
]

/** Browser-supported operations. Text/1 and OpenSCAD intentionally have distinct signatures. */
export const FUNCTION_REFERENCE: readonly ReferenceEntry[] = [
  {
    id: 'impl', name: 'impl / Self', category: 'patterns',
    summary: text('Реализует методы trait и задаёт связанные типы для структуры.', 'Implements trait methods and assigns associated types for a structure.'),
    keywords: ['метод', 'реализация', 'связанный тип', 'default', 'associated', 'method'],
    variants: { modelgraph: graph('impl Render for Plate { type Output = Geometry; fn render self: Self ret expression }', [parameter('Self.Output', 'Связанный тип результата; в generic-функции доступен как T.Output.', 'Associated result type; available as T.Output in a generic function.')], 'trait Render {\n  type Output\n  fn render self: Self -> Self.Output\n}\nstruct Plate { width: length }\nimpl Render for Plate {\n  type Output = Geometry\n  fn render self: Self ret box(self.width,12mm,4mm)\n}\nshow Plate {width:20mm}.render()', text('Метод с телом в trait имеет реализацию по умолчанию. Явный выбор: Render.render(value).', 'A method body in the trait provides a default implementation. Explicit selection: Render.render(value).')) },
  },
  {
    id: 'with', name: 'with', category: 'patterns',
    summary: text('Создаёт копию записи с изменёнными полями, сохраняя исходный тип.', 'Copies a record with changed fields while preserving its type.'),
    variants: { modelgraph: graph('record with { field: value }', [parameter('field', 'Существующее поле; новое значение должно сохранять его тип.', 'An existing field; its replacement must preserve its type.')], 'base = { width: 10mm, depth: 12mm, height: 4mm }\nwide = base with { width: 20mm }\nshow box(wide.width,wide.depth,wide.height)', text('Исходная запись неизменна. Вложенные записи обновляются явно.', 'The original record is unchanged. Nested updates are explicit.')) },
  },
  {
    id: 'trait', name: 'trait', category: 'patterns',
    summary: text('Описывает обязательные поля для generic-параметра функции.', 'Specifies required fields for a function generic parameter.'),
    variants: { modelgraph: graph('trait Sized { width: length } / fn widen[T: Sized] value: T ret expression', [parameter('T: Sized', 'Запись или структура должна содержать поля trait подходящих типов.', 'The record or structure must contain the trait fields with compatible types.')], 'trait Sized { width: length, depth: length, height: length }\nfn widen[T: Sized] value: T -> T ret value with { width: value.width + 5mm }\np = widen({ width: 10mm, depth: 12mm, height: 4mm })\nshow box(p.width,p.depth,p.height)', text('Несколько ограничений: T: Sized + Positioned. Методы используют self: Self; impl задаёт обязательные методы и связанные типы.', 'Multiple bounds: T: Sized + Positioned. Methods use self: Self; impl supplies required methods and associated types.')) },
  },
  {
    id: 'function', name: 'fn', category: 'patterns',
    summary: text('Объявляет функцию с аргументами по умолчанию и коротким результатом.', 'Declares a function with default arguments and a concise result expression.'),
    keywords: ['функция', 'аргументы', 'умолчанию', 'defaults', 'function', 'return', 'тип'],
    variants: {
      modelgraph: graph('fn name argument: Type, optional: Type = value ret expression', [parameter('= value', 'Значение пропущенного аргумента. Может использовать предыдущие аргументы.', 'Value for an omitted argument. May refer to earlier arguments.'), parameter('ret expression', 'Возвращаемое выражение; тип результата можно не указывать.', 'Returned expression; the result type can be omitted.')], 'fn plate width: length, depth: length = width / 2, height: length = 4mm ret\n  box([width,depth,height])\nshow [plate(20mm),plate(width: 20mm,height: 6mm).move(x:25mm)]', text('Типы входных аргументов обязательны. Для явного ограничения результата используйте -> Geometry. Именованные аргументы можно передавать в любом порядке; смешивать их с позиционными нельзя.', 'Input argument types are required. Use -> Geometry for an explicit result constraint. Named arguments can appear in any order but cannot be mixed with positional arguments.')),
    },
  },
  {
    id: 'destructure', name: '[a, b] = values', category: 'lists',
    summary: text('Распаковывает список в несколько именованных значений.', 'Unpacks a list into several named values.'),
    keywords: ['распаковка', 'список', 'массив', 'destructuring', 'unpack'],
    variants: {
      modelgraph: graph('[width, depth, height] = sizes', [parameter('names', 'Различные имена для элементов списка.', 'Distinct names for the list items.'), parameter('sizes', 'Список с точно таким же числом элементов.', 'List with exactly the same number of elements.')], 'sizes = [[20mm,12mm,4mm],[30mm,16mm,6mm]]\n[width,depth,height] = sizes[1]\nshow box([width,depth,height])', text('Работает также в функциях, foreach и блоках match. Длина вычисляемого списка проверяется при выполнении.', 'Also works in functions, foreach and match blocks. Dynamic list length is checked during evaluation.')),
    },
  },
  {
    id: 'assert', name: 'assert', category: 'checks',
    summary: text('Проверяет условие; в ModelGraph также проверяет геометрию выбранной детали.', 'Checks a condition; ModelGraph also supports checks on a selected geometry target.'),
    keywords: ['проверка', 'условие', 'ошибка', 'контракт', 'validate', 'validation', 'guard'],
    variants: {
      openscad: scad('assert(condition, "message");', [parameter('condition', 'Условие, которое должно быть истинным.', 'Condition that must be true.'), parameter('message', 'Текст ошибки при нарушении условия.', 'Error text when the condition fails.')], 'size = 12;\nassert(size > 0, "Size must be positive");\ncube([size, size, 4]);', text('Можно писать отдельной инструкцией или перед фигурой/блоком, в модуле и цикле. Форма assert как часть выражения пока не поддерживается браузерным движком.', 'Use as a statement or before geometry/a block, inside a module or loop. Expression-form assert is not yet supported by the browser engine.')),
      modelgraph: graph('assert(condition).message("text") / assert part.isWatertight()', [parameter('condition', 'Логическое условие или цепочка сравнений: size.atLeast(1mm).', 'A predicate or comparison chain, such as size.atLeast(1mm).'), parameter('part', 'Доступная в текущей области фигура, включая промежуточную или скрытую.', 'Geometry in the current scope, including an intermediate or hidden part.')], 'fn plate height: length -> Geometry\n  assert(height > 0mm).message("Height must be positive")\n  body = box([20mm, 12mm, height])\n  assert body.isWatertight()\n  assert measure(body).height.approximately(height, tolerance: 0.01mm)\n  ret body\nshow plate(4mm)', text('Доступен на верхнем уровне, в функциях, foreach и блоках match. Проверяются только выполняемые ветки и итерации. Геометрия проверяется при построении: hasBodies, isWatertight, hasNoDegenerateTriangles и measure(part).width/depth/height. own-nurbs пока не поддерживает эти проверки.', 'Available at the top level, in functions, foreach and match blocks. Only executed branches and iterations are checked. Built geometry supports hasBodies, isWatertight, hasNoDegenerateTriangles and measure(part).width/depth/height. own-nurbs does not yet support these checks.')),
    },
  },
  {
    id: 'validate', name: 'validate', category: 'checks',
    summary: text('Проверяет значения и параметры; геометрию проверяют через assert.', 'Checks values and parameters; use assert to check geometry.'),
    keywords: ['проверка', 'валидация', 'параметр', 'диапазон', 'assert', 'validation', 'constraint'],
    variants: {
      modelgraph: graph('validate value.between(min, max).message("text")', [parameter('value', 'Число, величина с единицами или логическое условие.', 'Number, quantity with units, or predicate.'), parameter('min / max', 'Включительные границы с совместимыми единицами.', 'Inclusive bounds with compatible units.')], 'param gap = 0.3mm range 0mm..1mm\nvalidate gap.between(0.1mm, 0.6mm).message("Gap must be 0.1–0.6 mm")\nshow box([20mm, 12mm, 4mm + gap])', text('Для скалярных проверок поведение совпадает с assert; по умолчанию отличается текст ошибки. Разрешён в тех же блоках, что assert. Также: validate(gap > 0mm).', 'Scalar checks behave like assert; the default error message differs. Allowed in the same blocks as assert. Also: validate(gap > 0mm).')),
    },
  },
  {
    id: 'translate', name: 'translate', category: 'transforms',
    summary: text('Перемещает фигуру по осям X, Y и Z.', 'Moves geometry along the X, Y and Z axes.'),
    keywords: ['перемещение', 'сдвиг', 'смещение', 'move', 'translation', 'position'],
    variants: {
      openscad: scad('translate([x, y, z]) children;', [parameter('[x, y, z]', 'Смещение по каждой оси.', 'Displacement along each axis.')], 'translate([20, 0, 5])\n  cube([10, 10, 10]);'),
      modelgraph: graph('geometry.move(x, y, z) / geometry.move([x, y, z])', [parameter('[x, y, z]', 'Расстояния; можно указать mm, cm, m или in.', 'Distances; mm, cm, m and in units are supported.')], 'show box([10mm, 10mm, 10mm]).move([20mm, 0, 5mm])', text('Также: geometry.move(x: 20mm, z: 5mm). translate остаётся совместимым именем. Неуказанные оси имеют смещение 0. Можно перемещать коллекцию: [a,b].move(z:10mm); объекты останутся отдельными.', 'Also: geometry.move(x: 20mm, z: 5mm). translate remains a compatible alias. Omitted axes default to 0. Collections work too: [a,b].move(z:10mm) keeps objects separate.')),
    },
  },
  {
    id: 'rotate', name: 'rotate', category: 'transforms',
    summary: text('Поворачивает фигуру вокруг начала координат.', 'Rotates geometry around the origin.'),
    keywords: ['поворот', 'вращение', 'rotation', 'turn', 'angle'],
    variants: {
      openscad: scad('rotate([x, y, z]) children;', [parameter('[x, y, z]', 'Углы вокруг осей в градусах; порядок X, затем Y, затем Z.', 'Angles in degrees; applied around X, then Y, then Z.')], 'rotate([0, 0, 30])\n  cube([20, 8, 5]);'),
      modelgraph: graph('geometry.rotate([x, y, z])', [parameter('[x, y, z]', 'Углы; поддерживаются deg и rad.', 'Angles; deg and rad units are supported.')], 'show box([20mm, 8mm, 5mm]).rotate(z: 30deg)', text('Можно передать вектор или именованные x, y, z; неуказанные углы равны 0.', 'Use a vector or named x, y, z arguments; omitted angles default to 0.')),
    },
  },
  {
    id: 'scale', name: 'scale', category: 'transforms',
    summary: text('Меняет размер фигуры по каждой оси.', 'Scales geometry independently along each axis.'),
    keywords: ['масштаб', 'масштабирование', 'увеличение', 'уменьшение', 'scaling', 'resize'],
    variants: {
      openscad: scad('scale([x, y, z]) children;', [parameter('[x, y, z]', 'Коэффициенты масштаба; 1 сохраняет размер.', 'Scale factors; 1 keeps the original size.')], 'scale([2, 1, 0.5])\n  sphere(r = 10, $fn = 32);'),
      modelgraph: graph('geometry.scale([x, y, z])', [parameter('[x, y, z]', 'Безразмерные коэффициенты масштаба.', 'Dimensionless scale factors.')], 'show sphere(10mm).scale([2, 1, 0.5])', text('Именованные оси также поддерживаются: scale(x: 2). Неуказанные коэффициенты равны 1.', 'Named axes also work: scale(x: 2). Omitted factors default to 1.')),
    },
  },
  {
    id: 'mirror', name: 'mirror', category: 'transforms',
    summary: text('Отражает фигуру относительно плоскости через начало координат.', 'Reflects geometry across a plane through the origin.'),
    keywords: ['отражение', 'зеркало', 'симметрия', 'reflection', 'mirroring', 'symmetry'],
    variants: {
      openscad: scad('mirror([x, y, z]) children;', [parameter('[x, y, z]', 'Ненулевой вектор нормали к плоскости отражения.', 'A nonzero vector normal to the reflection plane.')], 'mirror([1, 0, 0])\n  translate([5, 0, 0])\n    cube([12, 8, 5]);'),
      modelgraph: graph('geometry.mirror([x, y, z])', [parameter('[x, y, z]', 'Нормаль; [1, 0, 0] отражает относительно плоскости YZ.', 'Plane normal; [1, 0, 0] reflects across the YZ plane.')], 'show box([12mm, 8mm, 5mm]).translate(x: 5mm).mirror([1, 0, 0])'),
    },
  },
  {
    id: 'box', name: 'cube / box', category: 'primitives',
    summary: text('Создаёт куб или прямоугольный параллелепипед.', 'Creates a cube or rectangular box.'),
    variants: {
      openscad: scad('cube(size = [x, y, z], center = false);', [parameter('size', 'Размеры по осям или одно число для куба.', 'Axis dimensions, or one number for a cube.'), parameter('center', 'true помещает центр фигуры в начало координат.', 'true centers the shape at the origin.')], 'cube([20, 12, 8], center = true);'),
      modelgraph: graph('box(x, y, z) / box([x, y, z])', [parameter('[x, y, z]', 'Три положительных размера.', 'Three positive dimensions.')], 'show box([20mm, 12mm, 8mm])', text('Начальный угол находится в [0, 0, 0]. Для центрирования добавьте translate с половинами размеров со знаком минус.', 'Starts at [0, 0, 0]. Center it with a translation by negative half-dimensions.')),
    },
  },
  {
    id: 'sphere', name: 'sphere', category: 'primitives',
    summary: text('Создаёт сферу с центром в начале координат.', 'Creates a sphere centered at the origin.'),
    variants: {
      openscad: scad('sphere(r = radius, $fn = segments);', [parameter('r', 'Положительный радиус; d задаёт диаметр вместо r.', 'Positive radius; use d instead of r for a diameter.'), parameter('$fn', 'Число сегментов аппроксимации окружности.', 'Number of segments used to approximate a circle.')], 'sphere(r = 10, $fn = 40);'),
      modelgraph: graph('sphere(radius)', [parameter('radius', 'Положительный радиус.', 'Positive radius.')], 'segments 40\nshow sphere(10mm)', text('segments задаёт общую детализацию кривых: целое число от 12 до 128.', 'segments sets shared curve resolution: an integer from 12 to 128.')),
    },
  },
  {
    id: 'cylinder', name: 'cylinder', category: 'primitives',
    summary: text('Создаёт цилиндр вдоль оси Z.', 'Creates a cylinder along the Z axis.'),
    variants: {
      openscad: scad('cylinder(h = height, r = radius, center = false);', [parameter('h', 'Высота.', 'Height.'), parameter('r', 'Радиус; r1 и r2 позволяют задать конус.', 'Radius; r1 and r2 can define a cone.'), parameter('center', 'Центрировать высоту относительно Z = 0.', 'Center the height about Z = 0.')], 'cylinder(h = 20, r = 8, $fn = 40);'),
      modelgraph: graph('cylinder(radius, height)', [parameter('radius', 'Положительный радиус.', 'Positive radius.'), parameter('height', 'Положительная высота от Z = 0.', 'Positive height starting at Z = 0.')], 'show cylinder(radius: 8mm, height: 20mm)'),
    },
  },
  {
    id: 'circle', name: 'circle', category: 'profiles',
    summary: text('Создаёт круглый плоский контур для выдавливания.', 'Creates a circular 2D profile for extrusion.'),
    variants: {
      openscad: scad('circle(r = radius);', [parameter('r', 'Радиус; центр находится в [0, 0].', 'Radius; the center is at [0, 0].')], 'linear_extrude(height = 4)\n  circle(r = 12, $fn = 40);'),
      modelgraph: graph('circle(radius)', [parameter('radius', 'Положительный радиус профиля в плоскости XY.', 'Positive radius of a profile in the XY plane.')], 'show circle(12mm).extrude(4mm)', text('Перед показом превратите 2D-профиль в тело через extrude или revolve.', 'Turn a 2D profile into a solid with extrude or revolve before showing it.')),
    },
  },
  {
    id: 'rectangle', name: 'square / rectangle', category: 'profiles',
    summary: text('Создаёт прямоугольный плоский контур.', 'Creates a rectangular 2D profile.'),
    variants: {
      openscad: scad('square(size = [x, y], center = false);', [parameter('size', 'Ширина и глубина или одно число для квадрата.', 'Width and depth, or one number for a square.'), parameter('center', 'Центрировать контур в начале координат.', 'Center the profile at the origin.')], 'linear_extrude(height = 4)\n  square([20, 12], center = true);'),
      modelgraph: graph('rectangle([x, y])', [parameter('[x, y]', 'Два положительных размера от угла [0, 0].', 'Two positive dimensions starting at [0, 0].')], 'show rectangle([20mm, 12mm]).extrude(4mm)'),
    },
  },
  {
    id: 'polygon', name: 'polygon', category: 'profiles',
    summary: text('Строит плоский контур по упорядоченным вершинам.', 'Builds a 2D profile from ordered vertices.'),
    variants: {
      openscad: scad('polygon(points = [[x, y], ...]);', [parameter('points', 'Вершины границы; последний отрезок замыкается автоматически.', 'Boundary vertices; the final edge closes automatically.')], 'linear_extrude(height = 5)\n  polygon(points = [[0, 0], [20, 0], [10, 15]]);'),
      modelgraph: graph('polygon([[x, y], ...])', [parameter('points', 'От 3 до 256 вершин простого контура без самопересечений.', '3 to 256 vertices of a simple, non-self-intersecting profile.')], 'show polygon([[0, 0], [20mm, 0], [10mm, 15mm]]).extrude(5mm)'),
    },
  },
  {
    id: 'union', name: 'union', category: 'booleans',
    summary: text('Объединяет фигуры в общую геометрию.', 'Combines shapes into one geometry result.'),
    variants: {
      openscad: scad('union() { children; }', [parameter('children', 'Фигуры одной размерности: только 2D или только 3D.', 'Shapes of the same dimension: all 2D or all 3D.')], 'union() {\n  cube([20, 10, 6]);\n  translate([10, 0, 0]) cylinder(h = 10, r = 6, $fn = 32);\n}'),
      modelgraph: graph('union(part1, part2, ...)', [parameter('parts', 'Несколько фигур или коллекция одной размерности.', 'Multiple shapes or a collection of the same dimension.')], 'base = box([20mm, 10mm, 6mm])\npost = cylinder(6mm, 10mm).translate(x: 10mm)\nshow union(base, post)', text('show [a, b] сохраняет отдельные объекты; union(a, b) выполняет геометрическое объединение.', 'show [a, b] keeps objects separate; union(a, b) performs a geometric union.')),
    },
  },
  {
    id: 'difference', name: 'difference / subtract', category: 'booleans',
    summary: text('Вырезает отверстия и выемки из исходной фигуры.', 'Cuts holes and recesses out of a base shape.'),
    variants: {
      openscad: scad('difference() { base; cutters; }', [parameter('base', 'Первая дочерняя фигура — исходное тело.', 'The first child is the base shape.'), parameter('cutters', 'Последующие фигуры вычитаются из первой.', 'Subsequent children are subtracted from the first.')], 'difference() {\n  cube([20, 20, 8]);\n  translate([10, 10, -1]) cylinder(h = 10, r = 4, $fn = 32);\n}'),
      modelgraph: graph('base.subtract(cutter1, cutter2, ...)', [parameter('base', 'Исходное тело или профиль перед точкой.', 'The base solid or profile before the dot.'), parameter('cutters', 'Вычитаемые фигуры той же размерности.', 'Shapes of the same dimension to subtract.')], 'base = box([20mm, 20mm, 8mm])\nhole = cylinder(4mm, 10mm).translate([10mm, 10mm, -1mm])\nshow base.subtract(hole)'),
    },
  },
  {
    id: 'intersection', name: 'intersection', category: 'booleans',
    summary: text('Оставляет только общую часть фигур.', 'Keeps only the overlapping region of shapes.'),
    variants: {
      openscad: scad('intersection() { children; }', [parameter('children', 'Пересекающиеся фигуры одной размерности.', 'Overlapping shapes of the same dimension.')], 'intersection() {\n  cube([20, 20, 20], center = true);\n  sphere(r = 13, $fn = 32);\n}'),
      modelgraph: graph('intersection(part1, part2, ...)', [parameter('parts', 'Фигуры, общую область которых нужно оставить.', 'Shapes whose common region should remain.')], 'block = box([20mm, 20mm, 20mm]).translate([-10mm, -10mm, -10mm])\nshow intersection(block, sphere(13mm))'),
    },
  },
  {
    id: 'hull', name: 'hull', category: 'booleans',
    summary: text('Создаёт выпуклую оболочку вокруг нескольких фигур.', 'Creates the convex hull around several shapes.'),
    variants: {
      openscad: scad('hull() { children; }', [parameter('children', 'Фигуры, между которыми будет заполнено пространство.', 'Shapes whose convex connecting region will be filled.')], 'hull() {\n  cylinder(h = 4, r = 5, $fn = 32);\n  translate([25, 0, 0]) cylinder(h = 4, r = 5, $fn = 32);\n}'),
      modelgraph: graph('hull(part1, part2, ...)', [parameter('parts', 'Фигуры одной размерности.', 'Shapes of the same dimension.')], 'left = circle(5mm)\nright = circle(5mm).translate(x: 25mm)\nshow hull(left, right).extrude(4mm)'),
    },
  },
  {
    id: 'extrude', name: 'linear_extrude / extrude', category: 'profiles',
    summary: text('Выдавливает плоский контур вдоль оси Z.', 'Extrudes a 2D profile along the Z axis.'),
    variants: {
      openscad: scad('linear_extrude(height, center = false, twist = 0) children;', [parameter('height', 'Положительная высота выдавливания.', 'Positive extrusion height.'), parameter('center', 'Центрировать высоту относительно Z = 0.', 'Center the height about Z = 0.'), parameter('twist', 'Поворот верхнего сечения в градусах.', 'Rotation of the top section in degrees.')], 'linear_extrude(height = 20, twist = 30, slices = 20)\n  square([10, 6], center = true);'),
      modelgraph: graph('profile.extrude(height)', [parameter('height', 'Положительная высота от плоскости XY.', 'Positive height from the XY plane.')], 'show rectangle([20mm, 12mm]).extrude(8mm)', text('В компактном Text/1 эта операция принимает только высоту.', 'In compact Text/1 this operation accepts only the height.')),
    },
  },
  {
    id: 'revolve', name: 'rotate_extrude / revolve', category: 'profiles',
    summary: text('Вращает плоский контур вокруг оси Z, создавая тело.', 'Revolves a 2D profile around the Z axis to create a solid.'),
    variants: {
      openscad: scad('rotate_extrude(angle = 360) children;', [parameter('angle', 'Угол вращения в градусах.', 'Revolution angle in degrees.')], 'rotate_extrude(angle = 360, $fn = 48)\n  translate([15, 0]) circle(r = 4, $fn = 24);', text('X профиля задаёт радиус, Y — высоту. Держите профиль по одну сторону оси.', 'Profile X is the radius and Y is the height. Keep the profile on one side of the axis.')),
      modelgraph: graph('profile.revolve(angle)', [parameter('angle', 'Положительный угол не больше 360deg.', 'Positive angle no greater than 360deg.')], 'show circle(4mm).translate(x: 15mm).revolve(360deg)', text('X профиля задаёт радиус, Y — высоту. Профиль не должен пересекать ось вращения.', 'Profile X is the radius and Y is the height. The profile must not cross the rotation axis.')),
    },
  },
  {
    id: 'offset', name: 'offset', category: 'profiles',
    summary: text('Расширяет или сужает плоский контур.', 'Expands or contracts a 2D profile.'),
    variants: {
      openscad: scad('offset(r = distance) children; / offset(delta = distance) children;', [parameter('r', 'Отступ со скруглёнными внешними углами.', 'Offset with rounded outer corners.'), parameter('delta', 'Отступ с сохранением острых углов.', 'Offset preserving sharp corners.')], 'linear_extrude(height = 4)\n  offset(r = 2) square([20, 12]);'),
      modelgraph: graph('profile.offset(distance) / profile.offset(delta: distance)', [parameter('distance', 'Положительное значение расширяет контур, отрицательное сужает.', 'Positive values expand the profile; negative values contract it.'), parameter('delta', 'Именованный аргумент выбирает острые углы.', 'The named argument selects sharp corners.')], 'show rectangle([20mm, 12mm]).offset(2mm).extrude(4mm)'),
    },
  },
  {
    id: 'repeat', name: 'for / repeat', category: 'patterns',
    summary: text('Повторяет фигуру с вычисляемым положением или размером.', 'Repeats geometry with a calculated position or size.'),
    variants: {
      openscad: scad('for (i = [start : step : end]) children;', [parameter('i', 'Переменная текущей итерации.', 'Current iteration variable.'), parameter('[start : step : end]', 'Диапазон с включённой конечной точкой, если шаг её достигает.', 'Range including its endpoint when the step reaches it.')], 'for (i = [0 : 4])\n  translate([i * 12, 0, 0])\n    cylinder(h = 8, r = 4, $fn = 24);'),
      modelgraph: graph('repeat(count, function) / repeat(count, i => geometry)', [parameter('count', 'Целое число повторений от 1 до 256.', 'Integer repetition count from 1 to 256.'), parameter('i', 'Индекс от 0 до count - 1.', 'Index from 0 through count - 1.')], 'fn pin index: int => cylinder(4mm,8mm).move(x:index*12mm)\nshow repeat(5,pin)', text('repeat объединяет результаты. Для отдельных объектов используйте список с for или select.', 'repeat unions its results. Use a list comprehension or select for separate objects.')),
    },
  },
  {
    id: 'conditional', name: '? :', category: 'patterns',
    summary: text('Выбирает значение по условию.', 'Chooses a value according to a condition.'),
    variants: {
      openscad: scad('condition ? when_true : when_false', [parameter('condition', 'Логическое выражение.', 'Boolean expression.'), parameter('when_true / when_false', 'Значения для двух исходов условия.', 'Values for the two possible outcomes.')], 'large = true;\nradius = large ? 12 : 6;\nsphere(r = radius, $fn = 32);'),
      modelgraph: graph('condition ? when_true : when_false', [parameter('condition', '0 означает ложь, ненулевое значение — истину.', '0 is false; a nonzero value is true.'), parameter('when_true / when_false', 'Два значения либо две геометрические фигуры.', 'Two values or two geometry expressions.')], 'param rounded = 1 range 0..1\nshow rounded ? sphere(10mm) : box([20mm, 20mm, 20mm])'),
    },
  },
  {
    id: 'power', name: 'pow / **', category: 'math',
    summary: text('Возводит число в степень.', 'Raises a number to a power.'),
    variants: {
      openscad: scad('pow(base, exponent)', [parameter('base', 'Основание степени.', 'Base value.'), parameter('exponent', 'Показатель степени.', 'Exponent.')], 'height = pow(3, 2);\ncube([12, 12, height]);'),
      modelgraph: graph('base ** exponent', [parameter('base', 'Основание степени.', 'Base value.'), parameter('exponent', 'Безразмерный показатель степени.', 'Dimensionless exponent.')], 'height = (3 ** 2) * 1mm\nshow box([12mm, 12mm, height])'),
    },
  },
  {
    id: 'min', name: 'min', category: 'math',
    summary: text('Возвращает наименьшее значение.', 'Returns the smallest value.'),
    variants: {
      openscad: scad('min(a, b, ...) / min(values)', [parameter('values', 'Числа для сравнения.', 'Numbers to compare.')], 'height = min(12, 8, 15);\ncube([20, 12, height]);'),
      modelgraph: graph('values.min()', [parameter('values', 'Непустой список чисел с совместимыми единицами.', 'A nonempty list of numbers with compatible units.')], 'height = [12mm, 8mm, 15mm].min()\nshow box([20mm, 12mm, height])'),
    },
  },
  {
    id: 'max', name: 'max', category: 'math',
    summary: text('Возвращает наибольшее значение.', 'Returns the largest value.'),
    variants: {
      openscad: scad('max(a, b, ...) / max(values)', [parameter('values', 'Числа для сравнения.', 'Numbers to compare.')], 'height = max(12, 8, 15);\ncube([20, 12, height]);'),
      modelgraph: graph('values.max()', [parameter('values', 'Непустой список чисел с совместимыми единицами.', 'A nonempty list of numbers with compatible units.')], 'height = [12mm, 8mm, 15mm].max()\nshow box([20mm, 12mm, height])'),
    },
  },
  {
    id: 'sin', name: 'sin', category: 'math',
    summary: text('Вычисляет синус угла в градусах.', 'Calculates the sine of an angle in degrees.'),
    variants: { openscad: scad('sin(angle)', [parameter('angle', 'Угол в градусах.', 'Angle in degrees.')], 'angle = 30;\ntranslate([0, 20 * sin(angle), 0])\n  sphere(r = 4, $fn = 24);') },
  },
  {
    id: 'cos', name: 'cos', category: 'math',
    summary: text('Вычисляет косинус угла в градусах.', 'Calculates the cosine of an angle in degrees.'),
    variants: { openscad: scad('cos(angle)', [parameter('angle', 'Угол в градусах.', 'Angle in degrees.')], 'angle = 60;\ntranslate([20 * cos(angle), 0, 0])\n  sphere(r = 4, $fn = 24);') },
  },
  {
    id: 'abs', name: 'abs', category: 'math',
    summary: text('Возвращает модуль числа без знака.', 'Returns the absolute value of a number.'),
    variants: { openscad: scad('abs(value)', [parameter('value', 'Исходное число.', 'Input number.')], 'height = abs(-8);\ncube([12, 12, height]);') },
  },
  {
    id: 'sqrt', name: 'sqrt', category: 'math',
    summary: text('Вычисляет квадратный корень.', 'Calculates a square root.'),
    variants: { openscad: scad('sqrt(value)', [parameter('value', 'Неотрицательное число.', 'A nonnegative number.')], 'radius = sqrt(100);\nsphere(r = radius, $fn = 32);') },
  },
  {
    id: 'floor', name: 'floor', category: 'math',
    summary: text('Округляет число вниз до целого.', 'Rounds a number down to an integer.'),
    variants: { openscad: scad('floor(value)', [parameter('value', 'Число для округления.', 'Number to round.')], 'count = floor(4.8);\nfor (i = [0 : count - 1])\n  translate([i * 10, 0, 0]) cube([6, 6, 6]);') },
  },
  {
    id: 'ceil', name: 'ceil', category: 'math',
    summary: text('Округляет число вверх до целого.', 'Rounds a number up to an integer.'),
    variants: { openscad: scad('ceil(value)', [parameter('value', 'Число для округления.', 'Number to round.')], 'height = ceil(7.2);\ncube([12, 12, height]);') },
  },
  {
    id: 'round', name: 'round', category: 'math',
    summary: text('Округляет число до ближайшего целого.', 'Rounds a number to the nearest integer.'),
    variants: { openscad: scad('round(value)', [parameter('value', 'Число для округления.', 'Number to round.')], 'height = round(7.8);\ncube([12, 12, height]);') },
  },
  {
    id: 'length', name: 'len / length', category: 'lists',
    summary: text('Возвращает количество элементов списка.', 'Returns the number of items in a list.'),
    variants: {
      openscad: scad('len(values)', [parameter('values', 'Список или строка.', 'A list or string.')], 'sizes = [4, 6, 8];\ncube([len(sizes) * 5, 10, 4]);'),
      modelgraph: graph('length(values) / values.length()', [parameter('values', 'Список, число элементов которого нужно получить.', 'List whose item count is needed.')], 'sizes = [4mm, 6mm, 8mm]\nshow box([length(sizes) * 5mm, 10mm, 4mm])'),
    },
  },
  {
    id: 'concat', name: 'concat', category: 'lists',
    summary: text('Соединяет списки, сохраняя порядок элементов.', 'Joins lists while preserving item order.'),
    variants: {
      openscad: scad('concat(list1, list2, ...)', [parameter('lists', 'Списки, элементы которых нужно соединить.', 'Lists whose items should be joined.')], 'positions = concat([0, 12], [24, 36]);\nfor (x = positions)\n  translate([x, 0, 0]) cube([8, 8, 8]);'),
      modelgraph: graph('values.concat(other)', [parameter('other', 'Список, добавляемый в конец исходного.', 'List appended to the original list.')], 'positions = [0mm, 12mm].concat([24mm, 36mm])\nshow positions.select(x => box([8mm, 8mm, 8mm]).translate(x: x))'),
    },
  },
  {
    id: 'at', name: '[] / at', category: 'lists',
    summary: text('Получает элемент списка по индексу.', 'Gets a list item by its index.'),
    variants: {
      openscad: scad('values[index]', [parameter('index', 'Индекс первого элемента равен 0.', 'The first item has index 0.')], 'heights = [4, 8, 12];\ncube([12, 12, heights[1]]);'),
      modelgraph: graph('values[index] / at(values, index)', [parameter('values', 'Исходный список.', 'Input list.'), parameter('index', 'Целый индекс от 0 до length(values) - 1.', 'Integer index from 0 through length(values) - 1.')], 'heights = [4mm, 8mm, 12mm]\nshow box([12mm, 12mm, heights[1]])'),
    },
  },
  {
    id: 'range', name: 'range / ..', category: 'lists',
    summary: text('Создаёт равномерную последовательность значений.', 'Creates an evenly spaced sequence of values.'),
    variants: {
      openscad: scad('[start : step : end]', [parameter('start / end', 'Начальная и конечная точки.', 'Start and end values.'), parameter('step', 'Шаг; конечная точка входит в диапазон, если шаг её достигает.', 'Step; the endpoint is included when reached.')], 'for (x = [0 : 12 : 36])\n  translate([x, 0, 0]) sphere(r = 4, $fn = 24);'),
      modelgraph: graph('start..end by step / start..<end count n', [parameter('.. / ..<', 'Включить или исключить конечную точку.', 'Include or exclude the endpoint.'), parameter('by / count', 'Задать шаг или количество элементов; одновременно нельзя.', 'Set a step or item count, but not both.')], 'positions = 0mm..36mm by 12mm\nshow [for x in positions => sphere(4mm).translate(x: x)]', text('Для диапазонов с единицами нужно явно указать by или count. Не более 256 элементов.', 'Ranges with units require explicit by or count. At most 256 items.')),
    },
  },
  {
    id: 'select', name: 'select / map', category: 'lists',
    summary: text('Преобразует каждый элемент списка в значение или фигуру.', 'Transforms each list item into a value or shape.'),
    variants: {
      modelgraph: graph('values.select(function) / values.select(x => expression)', [parameter('x', 'Текущий элемент; второй аргумент лямбды может быть индексом.', 'Current item; an optional second lambda argument is the index.'), parameter('expression', 'Результат преобразования элемента.', 'Result of transforming one item.')], 'fn column height: length, index: int =>\n  box([8mm,8mm,height]).move(x:index*12mm)\nshow [5mm,10mm,15mm].select(column)', text('Можно передать имя функции. После select, возвращающего геометрию, доступны move, rotate, scale и mirror; фильтруйте и сортируйте заранее.', 'A function name can be passed directly. Geometry results support move, rotate, scale and mirror; filter and sort beforehand.')),
    },
  },
  {
    id: 'where', name: 'where / filter', category: 'lists',
    summary: text('Оставляет элементы списка, удовлетворяющие условию.', 'Keeps list items that satisfy a condition.'),
    variants: {
      modelgraph: graph('values.where(x => condition)', [parameter('condition', 'Условие: ненулевой результат сохраняет элемент.', 'Predicate: a nonzero result keeps the item.')], 'indices = (0..<6).where(i => i % 2 == 0)\nshow indices.select(i => cylinder(4mm, 8mm).translate(x: i * 12mm))'),
    },
  },
  {
    id: 'enumerate', name: 'enumerate', category: 'lists',
    summary: text('Добавляет к каждому элементу списка его индекс.', 'Pairs each list item with its index.'),
    variants: {
      modelgraph: graph('enumerate(values) / values.enumerate()', [parameter('values', 'Список; результат состоит из пар [индекс, значение].', 'Input list; produces [index, value] pairs.')], 'heights = [5mm, 10mm, 15mm]\nshow [for (i, h) in enumerate(heights) => box([8mm, 8mm, h]).translate(x: i * 12mm)]'),
    },
  },
  {
    id: 'zip', name: 'zip', category: 'lists',
    summary: text('Сопоставляет элементы нескольких списков по позиции.', 'Pairs items from several lists by position.'),
    variants: {
      modelgraph: graph('zip(list1, list2, ...)', [parameter('lists', 'От 2 до 8 списков одинаковой длины.', '2 to 8 lists of equal length.')], 'positions = [0mm, 15mm, 30mm]\nheights = [5mm, 10mm, 15mm]\nshow [for (x, h) in zip(positions, heights) => box([8mm, 8mm, h]).translate(x: x)]'),
    },
  },
  {
    id: 'take', name: 'take', category: 'lists',
    summary: text('Берёт первые элементы списка.', 'Takes the first items from a list.'),
    variants: {
      modelgraph: graph('values.take(count)', [parameter('count', 'Неотрицательное целое число элементов.', 'Nonnegative integer number of items.')], 'positions = [0mm, 12mm, 24mm, 36mm].take(3)\nshow positions.select(x => sphere(4mm).translate(x: x))'),
    },
  },
  {
    id: 'orderBy', name: 'orderBy', category: 'lists',
    summary: text('Сортирует список по возрастанию вычисленного ключа.', 'Sorts a list by an ascending calculated key.'),
    variants: {
      modelgraph: graph('values.orderBy(x => key)', [parameter('key', 'Ключ сортировки для каждого элемента.', 'Sort key for each item.')], 'heights = [15mm, 5mm, 10mm].orderBy(h => h)\nshow heights.select((h, i) => box([8mm, 8mm, h]).translate(x: i * 12mm))', text('orderByDescending сортирует по убыванию; thenBy добавляет следующий ключ.', 'orderByDescending sorts descending; thenBy adds a secondary key.')),
    },
  },
  {
    id: 'sum', name: 'sum', category: 'math',
    summary: text('Складывает числовые элементы списка.', 'Adds the numeric items of a list.'),
    variants: {
      modelgraph: graph('values.sum()', [parameter('values', 'Числовой список с совместимыми единицами.', 'Numeric list with compatible units.')], 'height = [2mm, 3mm, 5mm].sum()\nshow box([12mm, 12mm, height])', text('Для пустого списка результат — безразмерный 0.', 'An empty list returns dimensionless 0.')),
    },
  },
]

const normalizeSearch = (value: string) => value.normalize('NFKC').toLocaleLowerCase().replaceAll('ё', 'е')

/** Searches both locales, but only the selected language's callable signature and parameters. */
export function searchFunctionReference(
  query: string,
  language: ReferenceLanguage,
  category: ReferenceCategory | 'all' = 'all',
): ReferenceEntry[] {
  const terms = normalizeSearch(query).trim().split(/\s+/).filter(Boolean)
  return FUNCTION_REFERENCE.filter(entry => {
    const variant = entry.variants[language]
    if (!variant || (category !== 'all' && entry.category !== category)) return false
    const categoryLabel = REFERENCE_CATEGORIES.find(item => item.id === entry.category)!.label
    const haystack = normalizeSearch([
      entry.name, entry.id, entry.summary.ru, entry.summary.en, ...(entry.keywords ?? []),
      categoryLabel.ru, categoryLabel.en, variant.signature,
      ...variant.parameters.flatMap(item => [item.name, item.description.ru, item.description.en]),
      variant.notes?.ru ?? '', variant.notes?.en ?? '',
    ].join(' '))
    return terms.every(term => haystack.includes(term))
  })
}
