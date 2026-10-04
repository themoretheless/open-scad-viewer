import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { compileModelGraphText } from '../src/services/modelGraphText'
import { parseOpenSCAD } from '../src/services/openscadParser'
import { buildOwnNurbs } from '../src/services/modelGraphNurbsKernel'
import { compileModelGraphNurbs } from '../src/services/modelGraphNurbsCompiler'
import { importMeshFile } from '../src/services/meshImport'

it('validates dimensions and lowers primitive parameters through the language WASM', () => {
  const source = readFileSync('examples/rush/elliptic-cylinder.r', 'utf8')
  const compiled = compileModelGraphText(source)
  expect(compiled.execution_target).toBe('own-nurbs')
  const node = compiled.document.nodes.find(n => n.op === 'elliptic_cylinder_surface')
  expect(node).toMatchObject({ radius_x: 10, radius_y: 6, height: 12 })
  expect(() => compileModelGraphText(source.replace('radius_x: 10mm', 'radius_x: 10deg'))).toThrow()
  const parabola = compileModelGraphText(readFileSync('examples/rush/parabola-extrusion.r', 'utf8'))
  expect(parabola.document.nodes.find(n => n.op === 'parabola_curve')).toMatchObject({start: -2,end: 2,axis_u:[8,0,0]})
  const sheetSource = readFileSync('examples/rush/hyperboloid-two-sheet.r','utf8')
  expect(compileModelGraphText(sheetSource).document.nodes.find(n=>n.op==='hyperboloid_two_sheet')).toMatchObject({lower:true})
  expect(compileModelGraphText(sheetSource.replace('lower: true','lower: false')).document.nodes.find(n=>n.op==='hyperboloid_two_sheet')).toMatchObject({lower:false})
  expect(compileModelGraphText(sheetSource.replace(', lower: true','')).document.nodes.find(n=>n.op==='hyperboloid_two_sheet')).toMatchObject({lower:false})
  expect(()=>compileModelGraphText(sheetSource.replace('start: 0','start: 0mm'))).toThrow()
})

for (const name of ['ellipsoid', 'elliptic-torus', 'ellipse-extrusion', 'parabola-extrusion', 'hyperbola-extrusion', 'elliptic-cylinder', 'cone-frustum', 'quadratic-saddle', 'quadratic-paraboloid', 'hyperboloid-one-sheet', 'hyperboloid-two-sheet', 'polynomial-graph', 'polynomial-curve', 'polynomial-surface', 'rational-polynomial-curve', 'rational-polynomial-surface', 'line-extrusion', 'polyline-extrusion', 'circle-extrusion', 'circle-arc-extrusion', 'bezier-extrusion', 'composite-curve-extrusion', 'plane-patch', 'bilinear-patch', 'bezier-surface', 'profile-revolution', 'ruled-surface', 'section-loft', 'coons-patch', 'sphere-surface', 'cylinder-surface', 'cone-surface', 'formula-curve-extrusion', 'rational-formula-curve', 'formula-surface', 'rational-formula-surface', 'formula-curve-text', 'formula-surface-text', 'translation-sweep', 'framed-sweep', 'closed-framed-sweep', 'hermite-extrusion', 'natural-spline-extrusion', 'clamped-spline-extrusion', 'closed-spline-extrusion', 'hermite-patch', 'grid-spline-surface', 'natural-loft-surface', 'clamped-loft-surface', 'closed-loft-surface', 'gordon-surface', 'triangular-patch', 'boundary-fill', 'scaled-sweep', 'profile-sweep', 'closed-profile-sweep', 'progressive-sweep', 'closed-progressive-sweep', 'arc-length-progressive-sweep', 'multi-profile-progressive-sweep', 'progressive-hollow-body', 'affine-progressive-sweep', 'authored-progressive-sweep', 'contact-progressive-sweep', 'contact-progressive-hollow-body', 'authored-progressive-hollow-body', 'affine-hollow-body', 'closed-progressive-hollow-body', 'two-guide-sweep', 'two-guide-closed-sweep', 'twist-sweep', 'helix-extrusion', 'elliptic-helix-extrusion', 'conical-helix-extrusion', 'variable-pitch-helix-extrusion', 'involute-extrusion', 'logarithmic-spiral-extrusion', 'lissajous-extrusion', 'trochoid-extrusion', 'cycloid-extrusion', 'epicycloid-extrusion', 'hypocycloid-extrusion', 'archimedean-spiral-extrusion', 'catenary-extrusion', 'catenoid-surface', 'helicoid-surface', 'toroidal-spiral-extrusion', 'torus-knot-extrusion', 'spherical-spiral-extrusion', 'clothoid-extrusion', 'screw-surface', 'catenoid-patches', 'pipe-surface', 'closed-pipe-surface', 'variable-pipe-surface', 'closed-variable-pipe-surface', 'helicoid-patches', 'ribbon-surface', 'closed-ribbon-surface', 'circle-transition-surface', 'ellipse-transition-surface', 'circle-rectangle-transition']) {
  for (const format of ['obj','ply'] as const) {
    it(`round-trips the ${name} display mesh through ${format}`, async () => {
      const compiled = compileModelGraphText(readFileSync(`examples/rush/${name}.r`, 'utf8'))
      const built = buildOwnNurbs(compiled.document, {action:'build'})
      if (!('mesh' in built) || !built.mesh) throw new Error('Missing source mesh')
      const exported = buildOwnNurbs(compiled.document, {action:'export',format})
      if (!('artifact' in exported) || !exported.artifact || !('base64' in exported.artifact)) throw new Error('Missing mesh export')
      const bytes = Uint8Array.from(atob(exported.artifact.base64), c => c.charCodeAt(0))
      const imported = await importMeshFile(`shape.${format}`,bytes,{weld:false})
      expect(Array.from(imported.indices)).toEqual(Array.from(built.mesh.indices))
      expect(imported.positions.length).toBe(built.mesh.positions.length)
      let maxErrorMm = 0
      imported.positions.forEach((value,i) => {maxErrorMm=Math.max(maxErrorMm,Math.abs(value-built.mesh!.positions[i]!))})
      expect(maxErrorMm).toBeLessThan(1e-5)
    })
  }
  it(`round-trips the ${name} definition through JSON without changing rational controls`, () => {
    const source = readFileSync(`examples/rush/${name}.r`, 'utf8')
    const compiled = compileModelGraphText(source)
    const exported = buildOwnNurbs(compiled.document, {action: 'export',format: 'json'})
    if (!('artifact' in exported) || !exported.artifact) throw new Error('Missing JSON export')
    const imported = compileModelGraphNurbs(JSON.parse(exported.artifact.text))
    expect(imported.document_sha256).toBe(compiled.document_sha256)
    const rebuilt = buildOwnNurbs(imported.document, {action: 'build'})
    expect(rebuilt.report.definitions).toEqual(exported.report.definitions)
  })
  it(`compiles and renders the ${name} Rush example through the editor entrypoint`, async () => {
    const source = readFileSync(`examples/rush/${name}.r`, 'utf8')
    const compiled = compileModelGraphText(source)
    expect(compiled.execution_target).toBe('own-nurbs')
    const result = await parseOpenSCAD(source)
    expect(result.meshes).toHaveLength(1)
    expect(result.meshes[0]!.indices.length).toBeGreaterThan(0)
    expect(Array.from(result.meshes[0]!.vertices).every(Number.isFinite)).toBe(true)
  })
}
