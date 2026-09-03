/**
 * Machine-readable OpenSCAD language target for the repository-owned engine
 * and its optional differential oracle. The inventory is frozen to the
 * OpenSCAD 2021.01 release; neither the independent implementation nor a newer
 * separately pinned upstream snapshot is allowed to redefine this contract.
 */

export const OPENSCAD_2021_01_TAG = 'openscad-2021.01' as const
export const OPENSCAD_2021_01_COMMIT = '41f58fe57c03457a3a8b4dc541ef5654ec3e8c78' as const
export const OPENSCAD_2021_01_FUNCTION_COUNT = 38 as const
export const OPENSCAD_2021_01_MODULE_COUNT = 35 as const

export interface OpenScadSmokeCase {
  readonly call: string
  readonly source: string
  readonly fixtureIds: readonly string[]
  readonly expected: 'defined-value-and-3d-mesh' | '3d-mesh'
}

export interface OpenScadBuiltinFunctionContract {
  readonly name: string
  readonly category: 'math' | 'trigonometry' | 'sequence' | 'string' | 'version' | 'vector' | 'introspection'
  readonly smoke: OpenScadSmokeCase
}

export interface OpenScadBuiltinModuleContract {
  readonly name: string
  readonly category: 'primitive-2d' | 'primitive-3d' | 'transform' | 'csg' | 'extrusion' | 'file' | 'display' | 'control'
  readonly smoke: OpenScadSmokeCase
}

export interface OpenScadSmokeFixtureContract {
  readonly id: string
  readonly path: string
  readonly mediaType: string
  readonly provisioning: 'inline-text' | 'conformance-fixture-required' | 'runtime-cache-required'
  readonly text?: string
  readonly requirements: readonly string[]
}

function valueSmoke(call: string, source?: string, fixtureIds: readonly string[] = []): OpenScadSmokeCase {
  return Object.freeze({
    call,
    source: source ?? `echo(${call}); cube(1);`,
    fixtureIds: Object.freeze([...fixtureIds]),
    expected: 'defined-value-and-3d-mesh' as const,
  })
}

function meshSmoke(call: string, source?: string, fixtureIds: readonly string[] = []): OpenScadSmokeCase {
  return Object.freeze({
    call,
    source: source ?? `${call};`,
    fixtureIds: Object.freeze([...fixtureIds]),
    expected: '3d-mesh' as const,
  })
}

function builtinFunction(
  name: string,
  category: OpenScadBuiltinFunctionContract['category'],
  call: string,
  source?: string,
  fixtureIds: readonly string[] = [],
): OpenScadBuiltinFunctionContract {
  return Object.freeze({ name, category, smoke: valueSmoke(call, source, fixtureIds) })
}

function builtinModule(
  name: string,
  category: OpenScadBuiltinModuleContract['category'],
  call: string,
  source?: string,
  fixtureIds: readonly string[] = [],
): OpenScadBuiltinModuleContract {
  return Object.freeze({ name, category, smoke: meshSmoke(call, source, fixtureIds) })
}

/** The 38 non-legacy functions registered by src/func.cc in 2021.01. */
export const OPENSCAD_2021_01_BUILTIN_FUNCTIONS = Object.freeze([
  builtinFunction('abs', 'math', 'abs(-3)'),
  builtinFunction('sign', 'math', 'sign(-3)'),
  builtinFunction('rands', 'math', 'rands(0, 1, 3, 17)'),
  builtinFunction('min', 'math', 'min([3, 1, 2])'),
  builtinFunction('max', 'math', 'max([3, 1, 2])'),
  builtinFunction('sin', 'trigonometry', 'sin(30)'),
  builtinFunction('cos', 'trigonometry', 'cos(60)'),
  builtinFunction('asin', 'trigonometry', 'asin(0.5)'),
  builtinFunction('acos', 'trigonometry', 'acos(0.5)'),
  builtinFunction('tan', 'trigonometry', 'tan(45)'),
  builtinFunction('atan', 'trigonometry', 'atan(1)'),
  builtinFunction('atan2', 'trigonometry', 'atan2(1, 1)'),
  builtinFunction('round', 'math', 'round(2.6)'),
  builtinFunction('ceil', 'math', 'ceil(2.1)'),
  builtinFunction('floor', 'math', 'floor(2.9)'),
  builtinFunction('pow', 'math', 'pow(2, 3)'),
  builtinFunction('sqrt', 'math', 'sqrt(9)'),
  builtinFunction('exp', 'math', 'exp(1)'),
  builtinFunction('len', 'sequence', 'len([1, 2, 3])'),
  builtinFunction('log', 'math', 'log(100)'),
  builtinFunction('ln', 'math', 'ln(exp(1))'),
  builtinFunction('str', 'string', 'str("Open", "SCAD", 2021)'),
  builtinFunction('chr', 'string', 'chr(65)'),
  builtinFunction('ord', 'string', 'ord("A")'),
  builtinFunction('concat', 'sequence', 'concat([1, 2], [3])'),
  builtinFunction('lookup', 'sequence', 'lookup(0.5, [[0, 0], [1, 10]])'),
  builtinFunction('search', 'sequence', 'search("b", "abc")'),
  builtinFunction('version', 'version', 'version()'),
  builtinFunction('version_num', 'version', 'version_num()'),
  builtinFunction('norm', 'vector', 'norm([3, 4])'),
  builtinFunction('cross', 'vector', 'cross([1, 0, 0], [0, 1, 0])'),
  builtinFunction(
    'parent_module',
    'introspection',
    'parent_module(0)',
    'module contract_parent_probe() { echo(parent_module(0)); cube(1); } contract_parent_probe();',
  ),
  builtinFunction('is_undef', 'introspection', 'is_undef(undef)'),
  builtinFunction('is_list', 'introspection', 'is_list([1])'),
  builtinFunction('is_num', 'introspection', 'is_num(1)'),
  builtinFunction('is_bool', 'introspection', 'is_bool(true)'),
  builtinFunction('is_string', 'introspection', 'is_string("x")'),
  builtinFunction('is_function', 'introspection', 'is_function(function(x) x)'),
] as const)

/**
 * The 35 non-deprecated built-in modules. Parser keywords which happen to be
 * implemented by module objects (for/if/let/assert/echo) intentionally remain
 * in this inventory because they are callable module-language surface.
 */
export const OPENSCAD_2021_01_BUILTIN_MODULES = Object.freeze([
  builtinModule('render', 'display', 'render(convexity = 2) cube(1)'),
  builtinModule('color', 'display', 'color("red", 1) cube(1)'),
  builtinModule(
    'offset',
    'transform',
    'offset(r = 0.1) square(1)',
    'linear_extrude(height = 1) offset(r = 0.1) square(1);',
  ),
  builtinModule('group', 'csg', 'group() cube(1)'),
  builtinModule(
    'text',
    'primitive-2d',
    'text("A", size = 4, font = "Basic:style=Regular", halign = "center", valign = "center")',
    'linear_extrude(height = 1) text("A", size = 4, font = "Basic:style=Regular", halign = "center", valign = "center");',
    ['fontconfig-default-font'],
  ),
  builtinModule(
    'import',
    'file',
    'import(file = "fixtures/import-square.svg")',
    'linear_extrude(height = 1) import(file = "fixtures/import-square.svg");',
    ['import-square-svg'],
  ),
  builtinModule('cube', 'primitive-3d', 'cube([1, 2, 3], center = true)'),
  builtinModule('sphere', 'primitive-3d', 'sphere(r = 1, $fn = 12)'),
  builtinModule('cylinder', 'primitive-3d', 'cylinder(h = 2, r1 = 1, r2 = 0.5, center = true, $fn = 12)'),
  builtinModule(
    'polyhedron',
    'primitive-3d',
    'polyhedron(points = [[0,0,0], [1,0,0], [0,1,0], [0,0,1]], faces = [[0,1,2], [0,2,3], [0,3,1], [1,3,2]])',
  ),
  builtinModule('square', 'primitive-2d', 'square([1, 2], center = true)', 'linear_extrude(height = 1) square([1, 2], center = true);'),
  builtinModule('circle', 'primitive-2d', 'circle(r = 1, $fn = 12)', 'linear_extrude(height = 1) circle(r = 1, $fn = 12);'),
  builtinModule(
    'polygon',
    'primitive-2d',
    'polygon(points = [[0,0], [2,0], [1,1]])',
    'linear_extrude(height = 1) polygon(points = [[0,0], [2,0], [1,1]]);',
  ),
  builtinModule('scale', 'transform', 'scale([2, 1, 0.5]) cube(1)'),
  builtinModule('rotate', 'transform', 'rotate([10, 20, 30]) cube(1)'),
  builtinModule('mirror', 'transform', 'mirror([1, 0, 0]) translate([1, 0, 0]) cube(1)'),
  builtinModule('translate', 'transform', 'translate([1, 2, 3]) cube(1)'),
  builtinModule('multmatrix', 'transform', 'multmatrix([[1,0,0,1], [0,1,0,2], [0,0,1,3], [0,0,0,1]]) cube(1)'),
  builtinModule('union', 'csg', 'union() { cube(1); translate([1, 0, 0]) cube(1); }'),
  builtinModule('difference', 'csg', 'difference() { cube(2); translate([1, 1, 1]) cube(2); }'),
  builtinModule('intersection', 'csg', 'intersection() { cube(2); translate([1, 1, 1]) cube(2); }'),
  builtinModule('linear_extrude', 'extrusion', 'linear_extrude(height = 2, twist = 15, slices = 2) square(1)'),
  builtinModule('minkowski', 'csg', 'minkowski() { cube(1); sphere(r = 0.2, $fn = 8); }'),
  builtinModule('hull', 'csg', 'hull() { cube(1); translate([2, 0, 0]) cube(1); }'),
  builtinModule('resize', 'transform', 'resize([2, 3, 4], auto = false) cube(1)'),
  builtinModule(
    'children',
    'control',
    'children(0)',
    'module contract_children_probe() { children(0); } contract_children_probe() cube(1);',
  ),
  builtinModule('echo', 'control', 'echo("contract smoke") cube(1)'),
  builtinModule('assert', 'control', 'assert(true, "contract smoke") cube(1)'),
  builtinModule('for', 'control', 'for (i = [0:1]) translate([i, 0, 0]) cube(1)'),
  builtinModule('let', 'control', 'let (size = 1) cube(size)'),
  builtinModule('intersection_for', 'control', 'intersection_for (i = [0:1]) translate([i * 0.25, 0, 0]) cube(1)'),
  builtinModule('if', 'control', 'if (true) cube(1)', 'if (true) cube(1); else sphere(1);'),
  builtinModule(
    'projection',
    'transform',
    'projection(cut = false) cube(1)',
    'linear_extrude(height = 1) projection(cut = false) cube(1);',
  ),
  builtinModule(
    'surface',
    'file',
    'surface(file = "fixtures/heightmap.dat", center = true)',
    'surface(file = "fixtures/heightmap.dat", center = true);',
    ['heightmap-dat'],
  ),
  builtinModule('rotate_extrude', 'extrusion', 'rotate_extrude(angle = 180, $fn = 16) translate([2, 0]) square([1, 1])'),
] as const)

export const OPENSCAD_2021_01_SMOKE_FIXTURES = Object.freeze([
  Object.freeze({
    id: 'import-square-svg',
    path: 'fixtures/import-square.svg',
    mediaType: 'image/svg+xml',
    provisioning: 'inline-text' as const,
    text: '<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 10 10"><path d="M0 0h10v10H0z"/></svg>\n',
    requirements: Object.freeze(['UTF-8 bytes must be mounted at the exact project-relative path.']),
  }),
  Object.freeze({
    id: 'heightmap-dat',
    path: 'fixtures/heightmap.dat',
    mediaType: 'text/plain',
    provisioning: 'inline-text' as const,
    text: '0 0 0\n0 1 0\n0 0 0\n',
    requirements: Object.freeze(['Whitespace-separated rectangular numeric height grid with at least two rows and columns.']),
  }),
  Object.freeze({
    id: 'fontconfig-default-font',
    path: 'fonts/Basic-Regular.ttf',
    mediaType: 'font/ttf',
    provisioning: 'runtime-cache-required' as const,
    requirements: Object.freeze([
      'The checksum-pinned OFL Basic Regular font is mounted together with a Fontconfig configuration that indexes it.',
      'The official runtime process resolves Basic:style=Regular without ambient host-font access.',
    ]),
  }),
  Object.freeze({
    id: 'legacy-tetra-stl',
    path: 'fixtures/legacy-tetra.stl',
    mediaType: 'model/stl',
    provisioning: 'conformance-fixture-required' as const,
    requirements: Object.freeze(['Closed, consistently oriented tetrahedron accepted by OpenSCAD 2021.01 STL import.']),
  }),
  Object.freeze({
    id: 'legacy-tetra-off',
    path: 'fixtures/legacy-tetra.off',
    mediaType: 'model/vnd.off',
    provisioning: 'conformance-fixture-required' as const,
    requirements: Object.freeze(['Closed tetrahedron in OFF format accepted by OpenSCAD 2021.01.']),
  }),
  Object.freeze({
    id: 'legacy-square-dxf',
    path: 'fixtures/legacy-square.dxf',
    mediaType: 'image/vnd.dxf',
    provisioning: 'conformance-fixture-required' as const,
    requirements: Object.freeze(['Closed square path on layer 0 accepted by OpenSCAD 2021.01 DXF import.']),
  }),
  Object.freeze({
    id: 'legacy-dimensioned-dxf',
    path: 'fixtures/legacy-dimensioned.dxf',
    mediaType: 'image/vnd.dxf',
    provisioning: 'conformance-fixture-required' as const,
    requirements: Object.freeze([
      'Contains a named dimension entity contract_dimension for dxf_dim().',
      'Contains two isolated two-point paths on layer contract_cross for dxf_cross().',
    ]),
  }),
] satisfies readonly OpenScadSmokeFixtureContract[])

export const OPENSCAD_2021_01_SYNTAX_CONSTRUCTS = Object.freeze([
  { id: 'line-comment', form: '// ...' },
  { id: 'block-comment', form: '/* ... */' },
  { id: 'empty-statement', form: ';' },
  { id: 'statement-block', form: '{ statements }' },
  { id: 'assignment', form: 'name = expression;' },
  { id: 'module-definition', form: 'module name(parameters) statement' },
  { id: 'function-definition', form: 'function name(parameters) = expression;' },
  { id: 'anonymous-function', form: 'function(parameters) expression' },
  { id: 'module-instantiation', form: 'name(arguments) child-statement' },
  { id: 'if-else-module', form: 'if (condition) statement [else statement]' },
  { id: 'literal-number', form: '1 | 1.5 | 1e3' },
  { id: 'literal-string', form: '"UTF-8 string"' },
  { id: 'literal-boolean', form: 'true | false' },
  { id: 'literal-undefined', form: 'undef' },
  { id: 'vector', form: '[expression, ...]' },
  { id: 'range', form: '[begin:end] | [begin:step:end]' },
  { id: 'function-call', form: 'expression(arguments)' },
  { id: 'index-lookup', form: 'expression[index]' },
  { id: 'member-lookup', form: 'expression.member' },
  { id: 'ternary', form: 'condition ? when_true : when_false' },
  { id: 'let-expression', form: 'let(bindings) expression' },
  { id: 'assert-expression', form: 'assert(condition [, message]) [expression]' },
  { id: 'echo-expression', form: 'echo(arguments) [expression]' },
  { id: 'list-comprehension-for', form: '[for(bindings) expression]' },
  { id: 'list-comprehension-c-for', form: '[for(init; condition; update) expression]' },
  { id: 'list-comprehension-if', form: '[if(condition) expression [else expression]]' },
  { id: 'list-comprehension-let', form: '[let(bindings) expression]' },
  { id: 'list-comprehension-each', form: '[each expression]' },
  { id: 'named-and-positional-arguments', form: 'name(positional, named = expression)' },
  { id: 'default-parameters', form: 'function-or-module-name(parameter = expression)' },
  { id: 'trailing-commas', form: '[a,] | name(a,) | module name(a,) ...' },
  { id: 'recursive-modules', form: 'module name(...) { name(...); }' },
  { id: 'recursive-functions', form: 'function name(...) = ... name(...) ...;' },
  { id: 'include-directive', form: 'include <path>' },
  { id: 'use-directive', form: 'use <path>' },
] as const)

export const OPENSCAD_2021_01_OPERATORS = Object.freeze([
  { id: 'postfix-call', tokens: ['()'], arity: 'postfix', precedence: 12, associativity: 'left' },
  { id: 'postfix-index', tokens: ['[]'], arity: 'postfix', precedence: 12, associativity: 'left' },
  { id: 'postfix-member', tokens: ['.'], arity: 'postfix', precedence: 12, associativity: 'left' },
  { id: 'exponent', tokens: ['^'], arity: 'binary', precedence: 11, associativity: 'right' },
  { id: 'unary', tokens: ['+', '-', '!'], arity: 'unary', precedence: 10, associativity: 'right' },
  { id: 'multiplicative', tokens: ['*', '/', '%'], arity: 'binary', precedence: 9, associativity: 'left' },
  { id: 'additive', tokens: ['+', '-'], arity: 'binary', precedence: 8, associativity: 'left' },
  { id: 'comparison', tokens: ['<', '<=', '>', '>='], arity: 'binary', precedence: 7, associativity: 'left' },
  { id: 'equality', tokens: ['==', '!='], arity: 'binary', precedence: 6, associativity: 'left' },
  { id: 'logical-and', tokens: ['&&'], arity: 'binary', precedence: 5, associativity: 'left' },
  { id: 'logical-or', tokens: ['||'], arity: 'binary', precedence: 4, associativity: 'left' },
  { id: 'conditional', tokens: ['?:'], arity: 'ternary', precedence: 3, associativity: 'right' },
] as const)

export const OPENSCAD_2021_01_CONSTANTS = Object.freeze([
  { name: 'true', valueKind: 'boolean', immutable: true },
  { name: 'false', valueKind: 'boolean', immutable: true },
  { name: 'undef', valueKind: 'undefined', immutable: true },
  { name: 'PI', valueKind: 'number', immutable: true },
] as const)

export const OPENSCAD_2021_01_SPECIAL_VARIABLES = Object.freeze([
  { name: '$fn', scope: 'dynamic-config', defaultValue: 0, purpose: 'fragment count' },
  { name: '$fs', scope: 'dynamic-config', defaultValue: 2, purpose: 'minimum fragment size' },
  { name: '$fa', scope: 'dynamic-config', defaultValue: 12, purpose: 'minimum fragment angle' },
  { name: '$t', scope: 'top-level-runtime', defaultValue: 0, purpose: 'animation time' },
  { name: '$preview', scope: 'top-level-runtime', defaultValue: false, purpose: 'preview versus render mode' },
  { name: '$vpt', scope: 'top-level-runtime', defaultValue: [0, 0, 0], purpose: 'viewport translation' },
  { name: '$vpr', scope: 'top-level-runtime', defaultValue: [55, 0, 25], purpose: 'viewport rotation' },
  { name: '$vpd', scope: 'top-level-runtime', defaultValue: 140, purpose: 'viewport camera distance' },
  { name: '$vpf', scope: 'top-level-runtime', defaultValue: 22.5, purpose: 'viewport field of view' },
  { name: '$children', scope: 'user-module', defaultValue: 'call-dependent', purpose: 'number of supplied child nodes' },
  { name: '$parent_modules', scope: 'user-module', defaultValue: 'call-stack-dependent', purpose: 'user-module stack depth' },
] as const)

export const OPENSCAD_2021_01_MODIFIERS = Object.freeze([
  { token: '!', name: 'root', previewEffect: 'render only the marked subtree as root' },
  { token: '#', name: 'highlight', previewEffect: 'highlight the marked subtree' },
  { token: '%', name: 'background', previewEffect: 'show as transparent background and exclude from normal render result' },
  { token: '*', name: 'disable', previewEffect: 'remove the marked subtree from evaluation' },
] as const)

export const OPENSCAD_2021_01_FILE_SEMANTICS = Object.freeze({
  directives: Object.freeze([
    {
      name: 'include',
      syntax: 'include <path>',
      semantics: 'Parse the target at the directive location, including assignments, definitions, and top-level geometry.',
    },
    {
      name: 'use',
      syntax: 'use <path>',
      semantics: 'Load public module and function definitions without executing target assignments or top-level geometry.',
    },
  ]),
  resolution: Object.freeze({
    relativeBase: 'directory-of-referencing-scad-file',
    librarySearch: 'OPENSCADPATH entries followed by platform built-in library paths',
    recursiveOpenProtection: true,
    mcpBoundary: 'Every source/library/asset is mounted in a bounded project-relative MEMFS; ambient host paths are unavailable.',
  }),
  import: Object.freeze({
    extensions: Object.freeze(['stl', 'off', 'dxf', 'nef3', '3mf', 'amf', 'svg']),
    conditionalExtensions: Object.freeze([{ extension: 'nef3', requirement: 'CGAL-enabled build' }]),
    dimension: 'STL/OFF/NEF3/3MF/AMF are 3D; DXF/SVG are 2D.',
  }),
  surface: Object.freeze({
    formats: Object.freeze(['PNG', 'whitespace-separated DAT height grid']),
    dimension: '3D',
  }),
  text: Object.freeze({
    dependency: 'Fontconfig/Freetype font discovery and the runtime font set',
    dimension: '2D',
  }),
})

export const OPENSCAD_2021_01_COMPATIBILITY_ALIASES = Object.freeze([
  {
    symbol: 'assign', kind: 'module', replacement: 'regular assignment or let',
    smoke: meshSmoke('assign(size = 1) cube(size)'),
  },
  {
    symbol: 'child', kind: 'module', replacement: 'children',
    smoke: meshSmoke('child(0)', 'module legacy_child_probe() { child(0); } legacy_child_probe() cube(1);'),
  },
  {
    symbol: 'dxf_linear_extrude', kind: 'module', replacement: 'linear_extrude',
    smoke: meshSmoke('dxf_linear_extrude(height = 1) square(1)'),
  },
  {
    symbol: 'dxf_rotate_extrude', kind: 'module', replacement: 'rotate_extrude',
    smoke: meshSmoke('dxf_rotate_extrude($fn = 12) translate([2, 0]) square(1)'),
  },
  {
    symbol: 'import_stl', kind: 'module', replacement: 'import',
    smoke: meshSmoke('import_stl(file = "fixtures/legacy-tetra.stl")', undefined, ['legacy-tetra-stl']),
  },
  {
    symbol: 'import_off', kind: 'module', replacement: 'import',
    smoke: meshSmoke('import_off(file = "fixtures/legacy-tetra.off")', undefined, ['legacy-tetra-off']),
  },
  {
    symbol: 'import_dxf', kind: 'module', replacement: 'import',
    smoke: meshSmoke(
      'import_dxf(file = "fixtures/legacy-square.dxf")',
      'linear_extrude(height = 1) import_dxf(file = "fixtures/legacy-square.dxf");',
      ['legacy-square-dxf'],
    ),
  },
  {
    symbol: 'dxf_dim', kind: 'function', replacement: 'no direct replacement; parse DXF dimension metadata externally',
    smoke: valueSmoke(
      'dxf_dim(file = "fixtures/legacy-dimensioned.dxf", name = "contract_dimension")',
      undefined,
      ['legacy-dimensioned-dxf'],
    ),
  },
  {
    symbol: 'dxf_cross', kind: 'function', replacement: 'no direct replacement; parse DXF intersection metadata externally',
    smoke: valueSmoke(
      'dxf_cross(file = "fixtures/legacy-dimensioned.dxf", layer = "contract_cross")',
      undefined,
      ['legacy-dimensioned-dxf'],
    ),
  },
] as const)

export const OPENSCAD_2021_01_COMPATIBILITY_TAIL = Object.freeze([
  {
    id: 'polyhedron-triangles-parameter',
    legacy: 'polyhedron(triangles = ...)',
    replacement: 'polyhedron(faces = ...)',
  },
  {
    id: 'import-filename-parameter',
    legacy: 'import(filename = ...)',
    replacement: 'import(file = ...)',
  },
  {
    id: 'import-layername-parameter',
    legacy: 'import(layername = ...)',
    replacement: 'import(layer = ...)',
  },
  {
    id: 'extrude-file-parameters',
    legacy: 'linear_extrude(file = ...) / rotate_extrude(file = ...)',
    replacement: 'extrusion module with an import() child',
  },
  {
    id: 'descending-implicit-range',
    legacy: '[begin:end] where begin > end',
    replacement: '[begin:-1:end]',
  },
  {
    id: 'document-root-asset-fallback',
    legacy: 'resolve a missing module-relative asset in the root document directory',
    replacement: 'place the asset relative to the referencing module',
  },
] as const)

export const OPENSCAD_2021_01_EXECUTION_SEMANTICS = Object.freeze({
  degreesForTrigonometry: true,
  dynamicScopeForDollarVariables: true,
  lexicalScopeForOrdinaryVariables: true,
  recursiveModules: true,
  recursiveFunctions: true,
  tailCallOptimization: Object.freeze({
    kind: 'direct-self-tail-calls',
    transparentExpressionWrappers: Object.freeze(['ternary-selected-branch', 'assert', 'echo', 'let']),
    stableEvaluatorIterationLimit: 1_000_001,
  }),
})

export const OPENSCAD_2021_01_EXECUTION_RUNTIME = Object.freeze({
  provider: 'OpenSCAD' as const,
  role: 'qualification-oracle' as const,
  channel: 'official-snapshot' as const,
  version: '2026.09.01' as const,
  archive: 'OpenSCAD-2026.09.01-WebAssembly-node.zip' as const,
  archiveUrl: 'https://files.openscad.org/snapshots/OpenSCAD-2026.09.01-WebAssembly-node.zip' as const,
  archiveSha256: '82054dfb4911686de0ee3ea36771dbf81f3d014c3460c8ea069ab4f933f6d888' as const,
  relationshipToLanguageTarget: 'different-release-validated-against-target-not-contract-source' as const,
  extensionPolicy: 'Snapshot-only syntax and built-ins do not count toward 2021.01 conformance.' as const,
  compatibilityTailPolicy: 'Deprecated 2021.01 aliases require an explicit probe or adapter shim; they are outside the 38+35 stable gate.' as const,
})

export const OPENSCAD_2021_01_INDEPENDENT_ENGINE = Object.freeze({
  provider: 'open-scad-viewer' as const,
  engineId: 'open-scad-viewer/independent-2021.01-dev.1' as const,
  stage: 'development' as const,
  upstreamRuntimeFallback: false as const,
  completeLanguageClaim: false as const,
})

export const OPENSCAD_2021_01_CONTRACT = Object.freeze({
  schemaVersion: 1 as const,
  id: 'openscad/stable-2021.01' as const,
  languageTarget: Object.freeze({
    project: 'OpenSCAD' as const,
    release: '2021.01' as const,
    tag: OPENSCAD_2021_01_TAG,
    commit: OPENSCAD_2021_01_COMMIT,
    sourceEvidence: Object.freeze([
      'src/func.cc',
      'src/dxfdim.cc',
      'src/builtin.cc',
      'src/parser.y',
      'src/lexer.l',
      'src/builtincontext.cc',
      'src/control.cc',
      'src/primitives.cc',
    ]),
  }),
  builtins: Object.freeze({
    functions: OPENSCAD_2021_01_BUILTIN_FUNCTIONS,
    modules: OPENSCAD_2021_01_BUILTIN_MODULES,
  }),
  syntax: OPENSCAD_2021_01_SYNTAX_CONSTRUCTS,
  operators: OPENSCAD_2021_01_OPERATORS,
  constants: OPENSCAD_2021_01_CONSTANTS,
  specialVariables: OPENSCAD_2021_01_SPECIAL_VARIABLES,
  modifiers: OPENSCAD_2021_01_MODIFIERS,
  fileSemantics: OPENSCAD_2021_01_FILE_SEMANTICS,
  compatibility: Object.freeze({
    callableAliases: OPENSCAD_2021_01_COMPATIBILITY_ALIASES,
    semanticTail: OPENSCAD_2021_01_COMPATIBILITY_TAIL,
  }),
  executionSemantics: OPENSCAD_2021_01_EXECUTION_SEMANTICS,
  independentEngine: OPENSCAD_2021_01_INDEPENDENT_ENGINE,
  executionRuntime: OPENSCAD_2021_01_EXECUTION_RUNTIME,
  smokeFixtures: OPENSCAD_2021_01_SMOKE_FIXTURES,
})

export type OpenScad202101Contract = typeof OPENSCAD_2021_01_CONTRACT
