import { describe, expect, it } from 'vitest'
import type { MeshData } from '../src/core/mesh'
import { OPENSCAD_2021_01_BUILTIN_MODULES } from '../src/core/openScad2021Contract'
import { AbortedError, OpenSCADParseError, parseOpenSCAD } from '../src/services/openscadParser'
import { EXAMPLES } from '../src/data/examples'
import { matchMeshesByProvenance } from '../src/services/meshInspection'

function bounds(meshes: MeshData[]) {
  const min = [Infinity, Infinity, Infinity]
  const max = [-Infinity, -Infinity, -Infinity]
  for (const mesh of meshes) {
    for (let i = 0; i < mesh.vertices.length; i += 6) {
      for (let axis = 0; axis < 3; axis++) {
        min[axis] = Math.min(min[axis], mesh.vertices[i + axis])
        max[axis] = Math.max(max[axis], mesh.vertices[i + axis])
      }
    }
  }
  return { min, max }
}

describe('OpenSCAD parser and geometry evaluator', () => {
  it('reports bind as its own nonnegative phase', async () => {
    const result = await parseOpenSCAD('cube(1);')
    expect(Object.keys(result.timings)).toEqual([
      'parseMs', 'bindMs', 'initializeMs', 'evaluateMs', 'analyzeMs',
    ])
    expect(result.timings.bindMs).toBeGreaterThanOrEqual(0)
    expect(result.timings.parseMs).toBeGreaterThanOrEqual(0)
  })

  it('evaluates variables and arithmetic', async () => {
    const result = await parseOpenSCAD('x = 10; cube([x, 2 + 3, 4]);')
    expect(result.meshes).toHaveLength(1)
    expect(result.volume).toBeCloseTo(200, 5)
    expect(bounds(result.meshes)).toEqual({ min: [0, 0, 0], max: [10, 5, 4] })
  })

  it('supports statement assertions as transparent geometry guards', async () => {
    const unguarded = await parseOpenSCAD('cube(1);')
    const guarded = await parseOpenSCAD(`
      assert(true);
      assert(true) cube(1);
      assert(condition = true, message = "valid") {
        sphere(1, $fn = 8);
        cylinder(h = 1, r = 1, $fn = 8);
      }
    `)

    expect(guarded.meshes).toHaveLength(3)
    expect(guarded.meshes[0].color).toEqual(unguarded.meshes[0].color)
  })

  it('evaluates assertion guards in module, loop, and children scopes', async () => {
    const source = `
      module guarded(ok = false) {
        assert(ok, message = str("invalid guard: ", ok)) children();
      }
      for (i = [1:2]) guarded(i > 0) if (i) cube(i);
    `
    const preview = await parseOpenSCAD(source, { quality: 'preview' })
    const full = await parseOpenSCAD(source, { quality: 'full' })

    expect(preview.meshes).toHaveLength(2)
    expect(full.meshes.map(mesh => mesh.entityId)).toEqual(preview.meshes.map(mesh => mesh.entityId))
  })

  it('reports positioned assertion failures and never evaluates their children', async () => {
    const source = `cube(1);
assert(2 + 2 == 5, "dimension contract failed") unsupported_child();`
    const failure = parseOpenSCAD(source)

    await expect(failure).rejects.toMatchObject({
      name: 'OpenSCADParseError',
      line: 2,
      column: 1,
    } satisfies Partial<OpenSCADParseError>)
    await expect(failure).rejects.toThrow("Assertion '2 + 2 == 5' failed: dimension contract failed")
    await expect(failure).rejects.not.toThrow('Unsupported geometry operation unsupported_child()')
  })

  it('uses OpenSCAD truthiness for assertions and conditional expressions', async () => {
    for (const condition of ['false', '0', 'undef', '""', '[]']) {
      await expect(parseOpenSCAD(`assert(${condition}) cube(1);`)).rejects.toThrow('Assertion')
    }
    for (const condition of ['true', '-1', '"false"', '[0]', '[[]]']) {
      await expect(parseOpenSCAD(`assert(${condition}) cube(1);`)).resolves.toMatchObject({
        meshes: [expect.any(Object)],
      })
    }

    const conditional = await parseOpenSCAD('if ("") sphere(1); else cube(1);')
    expect(conditional.meshes).toHaveLength(1)
    expect(bounds(conditional.meshes).max).toEqual([1, 1, 1])
  })

  it('strictly binds assertion arguments and eagerly validates a supplied message', async () => {
    const valid = await parseOpenSCAD(`
      assert(message = "named", condition = true,) cube(1);
      assert(true, message = ["mixed", 1]) sphere(1, $fn = 8);
    `)
    expect(valid.meshes).toHaveLength(2)

    await expect(parseOpenSCAD('assert();')).rejects.toThrow('requires a condition')
    await expect(parseOpenSCAD('assert(true, "one", "two");')).rejects.toThrow('does not accept argument _2')
    await expect(parseOpenSCAD('assert(true, detail = "nope");')).rejects.toThrow('does not accept argument detail')
    await expect(parseOpenSCAD('assert(_0 = true);')).rejects.toThrow('does not accept argument _0')
    await expect(parseOpenSCAD('assert(condition = true, _1 = "nope");')).rejects.toThrow('does not accept argument _1')
    await expect(parseOpenSCAD('assert(true, condition = true);')).rejects.toThrow('condition was provided more than once')
    await expect(parseOpenSCAD('assert(true, "one", message = "two");')).rejects.toThrow('message was provided more than once')
    await expect(parseOpenSCAD('assert(true, missing_message); cube(1);')).rejects.toThrow('Unknown variable missing_message')
  })

  it('rejects expression-form assert explicitly', async () => {
    await expect(parseOpenSCAD('value = assert(true); cube(1);'))
      .rejects.toThrow('Expression-form assert() is not supported; use statement assert()')
  })

  it('performs real boolean difference', async () => {
    const result = await parseOpenSCAD(`
      difference() {
        cube([10, 10, 10], center = true);
        sphere(r = 3, $fn = 48);
      }
    `)
    expect(result.meshes).toHaveLength(1)
    expect(result.volume).toBeGreaterThan(885)
    expect(result.volume).toBeLessThan(890)
    expect(result.meshes[0].color[3]).toBe(1)
  })

  it('composes nested transforms as parent * local', async () => {
    const result = await parseOpenSCAD('translate([10, 0, 0]) rotate([0, 0, 90]) cube([2, 4, 6]);')
    const box = bounds(result.meshes)
    expect(box.min[0]).toBeCloseTo(6, 5)
    expect(box.max[0]).toBeCloseTo(10, 5)
    expect(box.min[1]).toBeCloseTo(0, 5)
    expect(box.max[1]).toBeCloseTo(2, 5)
  })

  it('supports modules, ranges and loops', async () => {
    const result = await parseOpenSCAD(`
      module peg(x = 0) { translate([x, 0, 0]) cylinder(h = 2, r = 1, $fn = 16); }
      for (i = [0:2:4]) peg(i);
    `)
    expect(result.meshes).toHaveLength(3)
    expect(result.volume).toBeGreaterThan(18)
    expect(bounds(result.meshes).max[0]).toBeCloseTo(5, 5)
  })

  it('evaluates recursive, defaulted, named and anonymous functions in the full profile', async () => {
    const result = await parseOpenSCAD(`
      function factorial(n, acc = 1) = n <= 1 ? acc : factorial(n - 1, acc * n);
      callback = function(value) value + 1;
      cube([factorial(4), callback(2), [2, 3, 4].z]);
    `, { languageProfile: 'openscad/stable-2021.01' })

    expect(result.meshes).toHaveLength(1)
    expect(result.volume).toBeCloseTo(24 * 3 * 4, 5)
    expect(bounds(result.meshes)).toEqual({ min: [0, 0, 0], max: [24, 3, 4] })
  })

  it('uses OpenSCAD exponent, unary and right-associative precedence in the full profile', async () => {
    const result = await parseOpenSCAD(
      'cube([-2^2 + 5, 2^-2 * 4, 2^3^2 / 512]);',
      { languageProfile: 'openscad/stable-2021.01' },
    )
    expect(bounds(result.meshes)).toEqual({ min: [0, 0, 0], max: [1, 1, 1] })
  })

  it('keeps stable, distinct entity identities across quality for repeated module and loop instances', async () => {
    const source = `
      module peg(x = 0) {
        translate([x, 0, 0]) sphere(r = 1, $fn = 96);
      }
      for (i = [2, 0, 2]) peg(i);
      peg(6);
      peg(6);
    `
    const preview = await parseOpenSCAD(source, { quality: 'preview' })
    const full = await parseOpenSCAD(source, { quality: 'full' })
    const previewIds = preview.meshes.map(mesh => mesh.entityId)
    const fullIds = full.meshes.map(mesh => mesh.entityId)

    expect(preview.meshes).toHaveLength(5)
    expect(new Set(previewIds).size).toBe(5)
    expect(fullIds).toEqual(previewIds)
    expect(matchMeshesByProvenance(preview.meshes, full.meshes, { sameSourceSnapshot: true }))
      .toEqual([0, 1, 2, 3, 4])
    expect(full.meshes[0].indices.length).toBeGreaterThan(preview.meshes[0].indices.length)

    const references = preview.meshes.map(mesh => mesh.provenance.find(run => run.source)?.source)
    expect(references.every(reference => reference?.operationId && reference.instanceId)).toBe(true)
    expect(new Set(references.map(reference => reference?.operationId)).size).toBe(1)
    expect(new Set(references.map(reference => reference?.instanceId)).size).toBe(5)
    expect(references.map(reference => reference?.instanceId)).toEqual(previewIds)
  })

  it('matches reordered loop values by evaluated identity rather than scene order', async () => {
    const before = await parseOpenSCAD('for (i = [0, 2, 4]) translate([i, 0, 0]) cube(1);')
    const after = await parseOpenSCAD('for (i = [4, 0, 2]) translate([i, 0, 0]) cube(1);')

    expect(matchMeshesByProvenance(before.meshes, after.meshes)).toEqual([2, 0, 1])
  })

  it('keeps identities through whitespace edits and reordering unlike source offsets', async () => {
    const before = await parseOpenSCAD('cube(1);\ntranslate([3, 0, 0]) sphere(1);')
    const after = await parseOpenSCAD(`
      // An unrelated edit moves every source offset.
      translate([3, 0, 0]) sphere(1);

      cube(2);
    `)

    expect(after.meshes.map(mesh => mesh.entityId)).toEqual([
      before.meshes[1].entityId,
      before.meshes[0].entityId,
    ])
    expect(after.meshes[1].provenance[0].source?.start).not.toBe(before.meshes[0].provenance[0].source?.start)
    expect(matchMeshesByProvenance(before.meshes, after.meshes)).toEqual([1, 0])
  })

  it('does not claim identity for reordered same-name siblings across source snapshots', async () => {
    const before = await parseOpenSCAD('cube(1); cube(2);')
    const after = await parseOpenSCAD('cube(2); cube(1);')

    expect(after.meshes.map(mesh => mesh.entityId)).toEqual(before.meshes.map(mesh => mesh.entityId))
    expect(matchMeshesByProvenance(before.meshes, after.meshes)).toEqual([-1, -1])
    expect(matchMeshesByProvenance(before.meshes, after.meshes, { sameSourceSnapshot: true })).toEqual([0, 1])
  })

  it('supports 2D boolean geometry and extrusion', async () => {
    const result = await parseOpenSCAD(`
      linear_extrude(height = 5)
        difference() {
          square([10, 10], center = true);
          circle(r = 2, $fn = 48);
        }
    `)
    expect(result.meshes).toHaveLength(1)
    expect(result.volume).toBeGreaterThan(435)
    expect(result.volume).toBeLessThan(440)
  })

  it('attributes a generated 3D hull to the hull source call', async () => {
    const source = `hull() {
      cube([2, 2, 2], center = true);
      translate([8, 0, 0]) sphere(r = 2, $fn = 16);
    }`
    const result = await parseOpenSCAD(source)
    const runs = result.meshes[0].provenance

    expect(runs.length).toBeGreaterThan(0)
    expect(runs.every(run => run.source?.label === 'hull()')).toBe(true)
    expect(runs.every(run => run.source?.start === source.indexOf('hull'))).toBe(true)
    expect(runs.every(run => run.source?.end === source.length)).toBe(true)
  })

  it('records exact parser spans for separate source operations', async () => {
    const source = 'cube(1); /* a misleading ; and } */\n  sphere(2); // trailing comment'
    const result = await parseOpenSCAD(source)
    const references = result.meshes.map(mesh => mesh.provenance.find(run => run.source)?.source)

    expect(references[0]).toMatchObject({
      start: source.indexOf('cube'),
      end: source.indexOf(';') + 1,
      label: 'cube()',
    })
    expect(references[1]).toMatchObject({
      start: source.indexOf('sphere'),
      end: source.indexOf(';', source.indexOf('sphere')) + 1,
      label: 'sphere()',
    })
  })

  it('reports unsupported syntax with line and column', async () => {
    await expect(parseOpenSCAD('cube(1);\ntext("nope");')).rejects.toMatchObject({
      name: 'OpenSCADParseError',
      line: 2,
      column: 1,
    } satisfies Partial<OpenSCADParseError>)
  })

  it('clamps excessive tessellation', async () => {
    const result = await parseOpenSCAD('sphere(1, $fn = 9999);')
    expect(result.warnings[0]).toContain('clamped')
    expect(result.meshes[0].indices.length / 3).toBeLessThan(MAX_SAFE_TEST_TRIANGLES)
  })

  it('bounds expression nesting before the JavaScript stack overflows', async () => {
    const source = `value = ${'-'.repeat(300)}1; cube(value);`
    await expect(parseOpenSCAD(source)).rejects.toThrow('Expression exceeds 256 nested levels')
  })

  it('bounds left-associative expression evaluation before the JavaScript stack overflows', async () => {
    const source = `value = ${Array.from({ length: 3_000 }, () => '1').join('+')}; cube(value);`
    await expect(parseOpenSCAD(source)).rejects.toThrow('Expression exceeds 256 evaluated levels')
  })

  it('bounds nested geometry statements before recursive parsing overflows', async () => {
    const source = `${'translate([0, 0, 0]) '.repeat(300)}cube(1);`
    await expect(parseOpenSCAD(source)).rejects.toThrow('Model exceeds 128 nested statements')
  })

  it('bounds exponentially expanding evaluated values', async () => {
    const source = `x = [0];\n${'x = [x, x];\n'.repeat(24)}cube(1);`
    await expect(parseOpenSCAD(source)).rejects.toThrow(/value-allocation budget|Evaluated value exceeds/)
  })

  it('counts expression nodes as part of the syntax budget', async () => {
    const source = `values = [${Array.from({ length: 25_000 }, () => '0').join(',')}]; cube(1);`
    await expect(parseOpenSCAD(source)).rejects.toThrow('syntax node limit')
  })

  it('renders every bundled example', async () => {
    for (const [name, source] of Object.entries(EXAMPLES)) {
      const result = await parseOpenSCAD(source)
      expect(result.meshes.length, name).toBeGreaterThan(0)
      expect(result.meshes.every(mesh => mesh.vertices.every(Number.isFinite)), name).toBe(true)
    }
  })

  it('builds OpenSCAD-convention (clockwise-from-outside) polyhedra outward-facing', async () => {
    // Unit tetrahedron with faces wound per the OpenSCAD spec; volume must be
    // positive ~1/6 (the inverted-winding bug rejected or everted it).
    const result = await parseOpenSCAD(
      'polyhedron(points=[[0,0,0],[1,0,0],[0,1,0],[0,0,1]], faces=[[0,1,2],[0,2,3],[0,3,1],[1,3,2]]);',
    )
    expect(result.meshes).toHaveLength(1)
    expect(result.volume).toBeCloseTo(1 / 6, 3)
  })

  it('executes the canonical polyhedron smoke with a positive physical volume', async () => {
    const source = OPENSCAD_2021_01_BUILTIN_MODULES
      .find(entry => entry.name === 'polyhedron')!.smoke.source
    const result = await parseOpenSCAD(source, {
      languageProfile: 'openscad/stable-2021.01',
    })

    expect(result.meshes).toHaveLength(1)
    expect(result.volume).toBeCloseTo(1 / 6, 3)
  })

  it('accepts clockwise-wound polygon() point lists', async () => {
    // With the default Positive fill rule a CW outline yields an EMPTY shape.
    const result = await parseOpenSCAD('linear_extrude(height=2) polygon(points=[[0,0],[0,4],[4,4],[4,0]]);')
    expect(result.volume).toBeCloseTo(32, 3)
  })

  it('bounds nested loops that produce no geometry', async () => {
    const started = Date.now()
    await expect(parseOpenSCAD('for(i=[0:9999]) for(j=[0:9999]) x = i + j;')).rejects.toThrow(/evaluation step limit/)
    expect(Date.now() - started).toBeLessThan(10_000)
  })

  it('caps runaway concat() growth', async () => {
    const doubling = Array.from({ length: 40 }, () => 'a = concat(a, a);').join('\n')
    await expect(parseOpenSCAD(`a = [0:1:999];\n${doubling}\ncube(1);`)).rejects.toThrow(/concat\(\) result exceeds/)
  })

  it('caps linear_extrude slices before they reach the kernel', async () => {
    const started = Date.now()
    const result = await parseOpenSCAD('linear_extrude(height=10, twist=90, slices=100000000) square(5);')
    expect(result.meshes.length).toBeGreaterThan(0)
    expect(Date.now() - started).toBeLessThan(15_000)
  })
})

const MAX_SAFE_TEST_TRIANGLES = 150_000

describe('cooperative cancellation (shouldAbort)', () => {
  it('rejects with an AbortedError once shouldAbort turns true mid-evaluation', async () => {
    // >25 top-level statements so the loop reaches at least one yield point.
    const source = Array.from({ length: 120 }, (_, i) => `cube([1, 1, ${i + 1}]);`).join('\n')
    let calls = 0
    // False on the initial probe, true at every later (post-yield) check.
    const promise = parseOpenSCAD(source, { shouldAbort: () => calls++ > 0 })
    await expect(promise).rejects.toBeInstanceOf(AbortedError)
    await expect(promise).rejects.toMatchObject({ name: 'AbortedError' })
    expect(calls).toBeGreaterThan(1)
  })

  it('still succeeds with an always-false shouldAbort', async () => {
    const result = await parseOpenSCAD('x = 2; cube([x, x, x]);', { shouldAbort: () => false })
    expect(result.meshes).toHaveLength(1)
    expect(result.volume).toBeCloseTo(8, 5)
  })

  it('aborts inside a long for-loop at a mid-iteration yield, not only after the whole statement', async () => {
    let cancelled = false
    let yields = 0
    let clock = 0
    const result = parseOpenSCAD('for (i = [0:80]) cube([1, 1, 1]);', {
      shouldAbort: () => cancelled,
      // Each clock read advances past YIELD_EVERY_MS so yieldIfDue actually sleeps.
      now: () => { clock += 60; return clock },
      yieldControl: async () => { yields++; cancelled = true },
    })

    await expect(result).rejects.toBeInstanceOf(AbortedError)
    // First mid-loop yield cancels; forced post-eval yield is never reached.
    expect(yields).toBe(1)
  })

  it('aborts a for-loop nested in a module body, not only top-level loops', async () => {
    let cancelled = false
    let yields = 0
    let clock = 0
    const result = parseOpenSCAD('module row() { for (i = [0:80]) cube([1, 1, 1]); } row();', {
      shouldAbort: () => cancelled,
      now: () => { clock += 60; return clock },
      yieldControl: async () => { yields++; cancelled = true },
    })

    await expect(result).rejects.toBeInstanceOf(AbortedError)
    expect(yields).toBe(1)
  })

  it('still delivers a forced post-evaluation yield when a for-loop finishes without aborting', async () => {
    let yields = 0
    let clock = 0
    const result = await parseOpenSCAD('for (i = [0:2]) cube([1, 1, 1]);', {
      shouldAbort: () => false,
      now: () => { clock += 60; return clock },
      yieldControl: async () => { yields++ },
    })
    expect(result.meshes.length).toBeGreaterThan(0)
    // Mid-loop checks (range too small to hit every-25) + forced post-eval yield.
    expect(yields).toBeGreaterThanOrEqual(1)
  })
})

describe('preview reduction flag', () => {
  it.each([
    'cube([1, 2, 3]);',
    'sphere(r = 5, $fn = 16);',
    'module peg(x) { translate([x, 0, 0]) cylinder(h=4, r=1, $fn=12); } union() { peg(0); peg(2); }',
  ])('produces full-equivalent authoritative output whenever reduced=false: %s', async source => {
    const preview = await parseOpenSCAD(source, { quality: 'preview' })
    const full = await parseOpenSCAD(source, { quality: 'full' })
    const normalizeEphemeralKernelIds = (result: typeof preview) => ({
      ...result,
      quality: 'full' as const,
      timings: { parseMs: 0, bindMs: 0, initializeMs: 0, evaluateMs: 0, analyzeMs: 0 },
      meshes: result.meshes.map(mesh => ({
        ...mesh,
        provenance: mesh.provenance.map(run => ({
          ...run,
          source: run.source ? { ...run.source, originalId: 0 } : null,
        })),
      })),
    })

    expect(preview.reduced).toBe(false)
    // Kernel run IDs are process-global allocation handles and intentionally
    // differ across independent builds; stable source/evaluated identities and
    // every authoritative output byte must still match.
    expect(normalizeEphemeralKernelIds(preview)).toEqual(normalizeEphemeralKernelIds(full))
  })

  it('reports reduced=true when preview clamps a large $fn, false under full', async () => {
    const source = 'sphere(r = 5, $fn = 96);'
    const preview = await parseOpenSCAD(source, { quality: 'preview' })
    expect(preview.reduced).toBe(true)
    const full = await parseOpenSCAD(source, { quality: 'full' })
    expect(full.reduced).toBe(false)
  })

  it('reports reduced=false under both qualities when $fn is below the preview clamp', async () => {
    const source = 'sphere(r = 5, $fn = 16);'
    const preview = await parseOpenSCAD(source, { quality: 'preview' })
    expect(preview.reduced).toBe(false)
    const full = await parseOpenSCAD(source, { quality: 'full' })
    expect(full.reduced).toBe(false)
  })

  it('reports reduced=true when the preview default fallback is lower than the full one', async () => {
    // No $fn: sphere() falls back to 32 segments under full but 24 under preview.
    const preview = await parseOpenSCAD('sphere(r = 5);', { quality: 'preview' })
    expect(preview.reduced).toBe(true)
    const full = await parseOpenSCAD('sphere(r = 5);', { quality: 'full' })
    expect(full.reduced).toBe(false)
  })
})
