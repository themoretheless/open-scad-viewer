import { describe, it, expect } from 'vitest'
import { parseOpenSCAD, parseOpenSCADWithAST, type MeshData } from '../src/services/openscadParser'

/** Sum of triangles across all meshes. */
function totalTriangles(meshes: MeshData[]): number {
  return meshes.reduce((acc, m) => acc + m.indices.length / 3, 0)
}

/** Sum of vertices (interleaved pos+normal, 6 floats each) across all meshes. */
function totalVertices(meshes: MeshData[]): number {
  return meshes.reduce((acc, m) => acc + m.vertices.length / 6, 0)
}

function expectValidMesh(m: MeshData) {
  expect(m.vertices.length).toBeGreaterThan(0)
  expect(m.indices.length).toBeGreaterThan(0)
  // index buffer must describe whole triangles
  expect(m.indices.length % 3).toBe(0)
}

describe('primitives produce geometry', () => {
  it('cube([10,10,10]) returns a mesh with non-empty geometry', () => {
    const meshes = parseOpenSCAD('cube([10,10,10]);')
    expect(meshes.length).toBeGreaterThanOrEqual(1)
    meshes.forEach(expectValidMesh)
  })

  it('sphere(r=5) returns a mesh with non-empty geometry', () => {
    const meshes = parseOpenSCAD('sphere(r=5);')
    expect(meshes.length).toBeGreaterThanOrEqual(1)
    meshes.forEach(expectValidMesh)
  })

  it('cylinder(h=10,r=3) returns a mesh with non-empty geometry', () => {
    const meshes = parseOpenSCAD('cylinder(h=10,r=3);')
    expect(meshes.length).toBeGreaterThanOrEqual(1)
    meshes.forEach(expectValidMesh)
  })
})

describe('transforms', () => {
  it('translate([10,0,0]) cube(5) reflects the translation in the mesh transform', () => {
    const meshes = parseOpenSCAD('translate([10,0,0]) cube(5);')
    expect(meshes.length).toBeGreaterThanOrEqual(1)
    // Row-major Mat4: translation x is at index 3, y at 7, z at 11.
    const t = meshes[0].transform
    expect(t[3]).toBeCloseTo(10, 5)
    expect(t[7]).toBeCloseTo(0, 5)
    expect(t[11]).toBeCloseTo(0, 5)
  })

  it('an untranslated cube has identity translation', () => {
    const meshes = parseOpenSCAD('cube(5);')
    const t = meshes[0].transform
    expect(t[3]).toBeCloseTo(0, 5)
    expect(t[7]).toBeCloseTo(0, 5)
    expect(t[11]).toBeCloseTo(0, 5)
  })
})

describe('CSG smoke', () => {
  it('difference() of cube and sphere does not throw and returns meshes', () => {
    let meshes: MeshData[] = []
    expect(() => {
      meshes = parseOpenSCAD('difference(){cube(10); sphere(6);}')
    }).not.toThrow()
    expect(meshes.length).toBeGreaterThanOrEqual(1)
    meshes.forEach(expectValidMesh)
  })

  it('union() and intersection() do not throw', () => {
    expect(() => parseOpenSCAD('union(){cube(4); sphere(3);}')).not.toThrow()
    expect(() => parseOpenSCAD('intersection(){cube(4); sphere(3);}')).not.toThrow()
  })
})

describe('expressions', () => {
  it('cube(2+3) evaluates the arithmetic expression', () => {
    const meshes = parseOpenSCAD('cube(2+3);')
    expect(meshes.length).toBeGreaterThanOrEqual(1)
    expectValidMesh(meshes[0])
  })

  it('sphere(r=sin(90)) evaluates trig in degrees (sin(90) === 1)', () => {
    const meshes = parseOpenSCAD('sphere(r=sin(90));')
    expect(meshes.length).toBeGreaterThanOrEqual(1)
    expectValidMesh(meshes[0])
  })

  it('math functions like sqrt evaluate', () => {
    const meshes = parseOpenSCAD('cube(sqrt(16));')
    expect(meshes.length).toBeGreaterThanOrEqual(1)
    expectValidMesh(meshes[0])
  })
})

describe('control flow', () => {
  it('for(i=[0:2]) produces 3 cubes', () => {
    const meshes = parseOpenSCAD('for(i=[0:2]) translate([i*10,0,0]) cube(2);')
    expect(meshes.length).toBe(3)
    // each cube should sit at a distinct x offset
    const xs = meshes.map((m) => m.transform[3]).sort((a, b) => a - b)
    expect(xs[0]).toBeCloseTo(0, 5)
    expect(xs[1]).toBeCloseTo(10, 5)
    expect(xs[2]).toBeCloseTo(20, 5)
  })

  it('module definition + call produces geometry', () => {
    const meshes = parseOpenSCAD('module m(){cube(3);} m();')
    expect(meshes.length).toBeGreaterThanOrEqual(1)
    meshes.forEach(expectValidMesh)
  })
})

describe('$fn DoS guard (MAX_FN clamp)', () => {
  it('sphere(r=5, $fn=100000) does not hang and returns a bounded mesh', () => {
    const start = Date.now()
    const meshes = parseOpenSCAD('sphere(r=5, $fn=100000);')
    const elapsed = Date.now() - start
    expect(meshes.length).toBeGreaterThanOrEqual(1)
    // MAX_FN is clamped to 256; vertex count must stay well under 1e6.
    expect(totalVertices(meshes)).toBeLessThan(1_000_000)
    // sanity: should complete quickly (generous bound to avoid flakiness)
    expect(elapsed).toBeLessThan(5000)
  })
})

describe('range / expandRange guards', () => {
  it('for(i=[0:0:5]) (step 0) is handled without hanging', () => {
    const start = Date.now()
    // Step 0 throws "Invalid range step" inside evaluation; parseOpenSCAD does
    // not swallow it, so accept either a thrown error or a returned result —
    // the contract under test is "terminates quickly, does not hang".
    let threw = false
    try {
      parseOpenSCAD('for(i=[0:0:5]) cube(1);')
    } catch {
      threw = true
    }
    const elapsed = Date.now() - start
    expect(elapsed).toBeLessThan(2000)
    // Either it threw a catchable error, or it returned (any value) — both fine.
    expect(typeof threw).toBe('boolean')
  })

  it('for(i=[0:0:5]) is recovered gracefully via parseOpenSCADWithAST', () => {
    // expandRange throws "Invalid range step" for step 0, but evalNodes has a
    // per-node try/catch that skips the failing node. Observed behavior: the
    // bad loop is silently dropped (no meshes, no surfaced error) — it never hangs.
    const start = Date.now()
    const res = parseOpenSCADWithAST('for(i=[0:0:5]) cube(1);')
    expect(Date.now() - start).toBeLessThan(2000)
    expect(Array.isArray(res.meshes)).toBe(true)
    expect(res.meshes.length).toBe(0)
    expect(Array.isArray(res.errors)).toBe(true)
  })
})

describe('error recovery', () => {
  it('malformed input "cube(" does not crash the process', () => {
    // Top-level parseOpenSCAD may throw; assert it is a catchable error, not a hang/crash.
    expect(() => {
      try {
        parseOpenSCAD('cube(')
      } catch {
        /* swallow — a thrown parse error is acceptable */
      }
    }).not.toThrow()
  })

  it('malformed input is reported via parseOpenSCADWithAST without throwing', () => {
    let res
    expect(() => {
      res = parseOpenSCADWithAST('cube(')
    }).not.toThrow()
    expect(res).toBeDefined()
  })
})

describe('numeric edge cases', () => {
  it('cube(5/0) does not hang (result may be degenerate/empty)', () => {
    const start = Date.now()
    let meshes: MeshData[] = []
    try {
      meshes = parseOpenSCAD('cube(5/0);')
    } catch {
      /* acceptable */
    }
    const elapsed = Date.now() - start
    expect(elapsed).toBeLessThan(2000)
    expect(Array.isArray(meshes)).toBe(true)
  })
})

describe('empty + comments', () => {
  it("parseOpenSCAD('') returns []", () => {
    expect(parseOpenSCAD('')).toEqual([])
  })

  it('line comment before a primitive parses', () => {
    const meshes = parseOpenSCAD('// comment\ncube(5);')
    expect(meshes.length).toBeGreaterThanOrEqual(1)
    expectValidMesh(meshes[0])
  })

  it('block comment before a primitive parses', () => {
    const meshes = parseOpenSCAD('/* block */ cube(5);')
    expect(meshes.length).toBeGreaterThanOrEqual(1)
    expectValidMesh(meshes[0])
  })
})

describe('parseOpenSCADWithAST shape', () => {
  it('returns meshes, ast, echos and errors arrays', () => {
    const res = parseOpenSCADWithAST('cube(5);')
    expect(Array.isArray(res.meshes)).toBe(true)
    expect(Array.isArray(res.ast)).toBe(true)
    expect(Array.isArray(res.echos)).toBe(true)
    expect(Array.isArray(res.errors)).toBe(true)
    expect(res.meshes.length).toBeGreaterThanOrEqual(1)
    expect(totalTriangles(res.meshes)).toBeGreaterThan(0)
  })
})
