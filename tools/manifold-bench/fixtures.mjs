/** Same IDs and sizes as benchmarks/cpu-fixtures.mjs. Built with Manifold, not the product parser. */
export const cpuFixtures = [
  {
    id: 'small-bracket',
    description: 'Small parametric plate with four screw holes; interactive edit baseline.',
  },
  {
    id: 'medium-csg',
    description: '64 cylindrical cuts in a panel; boolean evaluation and mesh analysis.',
  },
  {
    id: 'many-bodies',
    description: '256 independent cubes; per-entity overhead and publication validation.',
  },
  {
    id: 'dense-sphere',
    description: 'One mesh containing three spheres at the supported $fn=256 cap; dense analysis.',
  },
]

export function buildFixture(wasm, id) {
  const { Manifold } = wasm
  if (id === 'small-bracket') {
    const plate = Manifold.cube([40, 30, 4], true)
    const holes = [-14, 14].flatMap(x => [-9, 9].map(y => (
      Manifold.cylinder(8, 2, 2, 32, true).translate([x, y, 0])
    )))
    return Manifold.difference([plate, ...holes])
  }
  if (id === 'medium-csg') {
    const panel = Manifold.cube([86, 86, 8], true)
    const holes = Array.from({ length: 64 }, (_, i) => (
      Manifold.cylinder(12, 3, 3, 32, true).translate([
        -35 + (i % 8) * 10,
        -35 + Math.floor(i / 8) * 10,
        0,
      ])
    ))
    return Manifold.difference([panel, ...holes])
  }
  if (id === 'many-bodies') {
    return Manifold.compose(Array.from({ length: 256 }, (_, i) => (
      Manifold.cube([2, 2, 2], false).translate([(i % 16) * 3, Math.floor(i / 16) * 3, 0])
    )))
  }
  if (id === 'dense-sphere') {
    return Manifold.union([
      Manifold.sphere(30, 256),
      Manifold.sphere(30, 256).translate([70, 0, 0]),
      Manifold.sphere(30, 256).translate([140, 0, 0]),
    ])
  }
  throw new Error(`Unknown fixture ${id}`)
}
