import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'
import { OpenScadProject } from '../src/services/openScadProject'
import { parseOpenSCAD, parseOpenScadProject } from '../src/services/openscadParser'

const stable = { languageProfile: 'openscad/stable-2021.01' as const, quality: 'full' as const }
const DIMENSIONED_DXF = readFileSync(
  new URL('./fixtures/legacy-dimensioned.dxf', import.meta.url),
  'utf8',
)
const TETRA_STL = [
  'solid tetra',
  'facet normal 0 0 -1', 'outer loop', 'vertex 0 0 0', 'vertex 0 1 0', 'vertex 1 0 0', 'endloop', 'endfacet',
  'facet normal 0 -1 0', 'outer loop', 'vertex 0 0 0', 'vertex 1 0 0', 'vertex 0 0 1', 'endloop', 'endfacet',
  'facet normal -1 0 0', 'outer loop', 'vertex 0 0 0', 'vertex 0 0 1', 'vertex 0 1 0', 'endloop', 'endfacet',
  'facet normal 1 1 1', 'outer loop', 'vertex 1 0 0', 'vertex 0 1 0', 'vertex 0 0 1', 'endloop', 'endfacet',
  'endsolid tetra',
].join('\n')
const TETRA_OFF = [
  'OFF', '4 4 0',
  '0 0 0', '1 0 0', '0 1 0', '0 0 1',
  '3 0 2 1', '3 0 1 3', '3 0 3 2', '3 1 2 3',
].join('\n')
const PROFILE_DXF = [
  '0', 'SECTION', '2', 'ENTITIES',
  '0', 'LWPOLYLINE', '8', 'profile', '70', '1',
  '10', '2', '20', '0',
  '10', '3', '20', '0',
  '10', '3', '20', '1',
  '10', '2', '20', '1',
  '0', 'ENDSEC', '0', 'EOF',
].join('\n')

describe('OpenSCAD 2021.01 direct compatibility host wiring', () => {
  it('runs assign(), child(), and both DXF extrusion aliases in child mode', async () => {
    const assign = await parseOpenSCAD(`
      outer = 10;
      assign(a = echo("first") 1, b = outer + 1, a = echo("second") 2,
        echo("ignored positional") 99) {
        a = 4;
        echo(a, b);
        cube([a, b, 1]);
      }
    `, stable)
    expect(assign.volume).toBeCloseTo(44, 6)
    expect(assign.warnings).toEqual([
      'ECHO: "first"',
      'ECHO: "second"',
      'ECHO: 4, 11',
    ])

    const child = await parseOpenSCAD(`
      module pick(which) { child(which, echo("ignored extra") 99); }
      pick() cube(1);
      pick("bad") translate([3, 0, 0]) cube(2);
      pick(-1) cube(3);
      pick(5) { cube(4); cube(5); }
    `, stable)
    expect(child.volume).toBeCloseTo(9, 6)
    expect(child.warnings).toEqual([
      'child() will be removed in future releases. Use children() instead.',
      'Negative child index (-1) not allowed',
      'Child index (5) out of bounds (2 children)',
    ])

    const linear = await parseOpenSCAD(
      'dxf_linear_extrude(echo("height") 2, $fn = 8) square(1);',
      stable,
    )
    expect(linear.volume).toBeCloseTo(2, 6)
    expect(linear.warnings).toEqual([
      'The dxf_linear_extrude() module will be removed in future releases. Use linear_extrude() instead.',
      'ECHO: "height"',
    ])

    const rotate = await parseOpenSCAD(
      'dxf_rotate_extrude(file = undef, angle = 90, $fn = 12) translate([2, 0]) square(1);',
      stable,
    )
    expect(rotate.meshes).toHaveLength(1)
    expect(rotate.volume).toBeGreaterThan(0)
    expect(rotate.warnings).toEqual([
      'The dxf_rotate_extrude() module will be removed in future releases. Use rotate_extrude() instead.',
    ])
  })

  it('prepares dynamic neutral-extension paths and runs all file-backed aliases and queries', async () => {
    const project = new OpenScadProject({
      entrypoint: 'main.scad',
      files: [
        {
          kind: 'source',
          path: 'main.scad',
          source: `
            stl_path = "assets/tetra.mesh";
            off_path = "assets/tetra.model";
            profile_path = "assets/profile.shape";
            query_path = "assets/dimensions.data";
            echo(
              dxf_dim(file = query_path, layer = "dims", name = "contract_dimension"),
              dxf_cross(file = query_path, layer = "contract_cross")
            );
            union() {
              import_stl(file = stl_path);
              translate([3, 0, 0]) import_off(file = off_path);
              translate([6, 0, 0]) linear_extrude(height = 1)
                import_dxf(file = profile_path, layer = "profile");
              translate([10, 0, 0])
                dxf_linear_extrude(file = profile_path, layer = "profile", height = 1);
              translate([14, 0, 0])
                dxf_rotate_extrude(file = profile_path, layer = "profile", angle = 90, $fn = 12);
            }
          `,
        },
        { kind: 'source', path: 'assets/tetra.mesh', source: TETRA_STL },
        { kind: 'source', path: 'assets/tetra.model', source: TETRA_OFF },
        { kind: 'source', path: 'assets/profile.shape', source: PROFILE_DXF },
        { kind: 'source', path: 'assets/dimensions.data', source: DIMENSIONED_DXF },
      ],
    })

    const result = await parseOpenScadProject(project, { quality: 'full' })
    expect(result.meshes).toHaveLength(1)
    expect(result.volume).toBeGreaterThan(4)
    expect(result.warnings).toEqual(expect.arrayContaining([
      'ECHO: 6, [3, 1]',
      'The import_stl() module will be removed in future releases. Use import() instead.',
      'The import_off() module will be removed in future releases. Use import() instead.',
      'The import_dxf() module will be removed in future releases. Use import() instead.',
      'The dxf_linear_extrude() module will be removed in future releases. Use linear_extrude() instead.',
      'The dxf_rotate_extrude() module will be removed in future releases. Use rotate_extrude() instead.',
    ]))
  })
})
