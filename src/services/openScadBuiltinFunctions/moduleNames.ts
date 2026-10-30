/** Dependency-free module inventory shared by binding and name discovery. */
export const OPENSCAD_BUILTIN_MODULE_NAMES = Object.freeze([
  'assign', 'assert', 'child', 'children', 'circle', 'color', 'cube', 'cylinder',
  'difference', 'dxf_linear_extrude', 'dxf_rotate_extrude', 'echo', 'for', 'group',
  'hull', 'if', 'import', 'import_dxf', 'import_off', 'import_stl',
  'intersection', 'intersection_for', 'let', 'linear_extrude', 'minkowski',
  'mirror', 'multmatrix', 'offset', 'polygon', 'polyhedron', 'projection',
  'render', 'resize', 'rotate', 'rotate_extrude', 'scale', 'sphere', 'square',
  'surface', 'text', 'translate', 'union',
] as const)
