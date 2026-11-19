import { buildOwnNurbs } from '../src/language/rushGraphNurbsKernel'
import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { compileRushFrontend } from '../src/language/rushFrontend'
import { buildTextNurbsScene } from '../src/language/rushFrontendScene'

import { evaluateNurbsCurve } from '../src/kernel/nurbsCurve'
import { evaluateNurbsSurface } from '../src/kernel/nurbsSurface'
import type { NurbsBrep } from '../src/kernel/geometry/brep'

const source = readFileSync('examples/rush/periodic-trim-edge-queries.r', 'utf8')
const fraction = (numerator: number, denominator = 1) => ({ numerator: String(numerator), denominator: String(denominator) })

async function reports(program: string) {
  const scene = await buildTextNurbsScene(compileRushFrontend(program).document)
  return scene.measurements!.map(row => JSON.parse(row.report))
}

it('preserves original traversal through a reversed range across the periodic seam', async () => {
  const [split, range, world, pairs] = await reports(source)
  expect(split.result).toHaveLength(2)
  const ids = split.result.map((hit: { edgeId: string }) => hit.edgeId)
  expect(new Set(ids).size).toBe(2)
  expect(range.result.map((part: { edgeId: string }) => part.edgeId)).toEqual([...ids].reverse())
  expect(range.result.map((part: { inputParameters: unknown[] }) => part.inputParameters)).toEqual([
    [fraction(3, 4), fraction(1, 2)], [fraction(1, 2), fraction(1, 4)],
  ])
  const sheetProgram = readFileSync('examples/rush/periodic-trim-sheet.r', 'utf8')
  const sheet = await buildTextNurbsScene(compileRushFrontend(sheetProgram).document)
  const model: NurbsBrep = JSON.parse(sheet.meshes[0]!.nativeGeometry!.geometryJson).geometry
  for (const [i, piece] of pairs.result.entries()) {
    expect(piece.range).toEqual(range.result[i])
    expect(piece.curve).toEqual(world.result[i].curve)
    expect(piece.pcurve.controlPoints.length).toBeGreaterThanOrEqual(2)
    expect(piece.faceId).toBe(pairs.result[0].faceId)
    const face = model.faces[model.topologyIds!.faces.indexOf(piece.faceId)]!
    const [a,b] = piece.range.edgeParameters.map((q:{numerator:string;denominator:string})=>Number(q.numerator)/Number(q.denominator))
    for (const u of [a, (a+b)/2, b]) {
      const uv=evaluateNurbsCurve(piece.pcurve,u).point
      expect(evaluateNurbsSurface(face.surface,uv[0]!,uv[1]!).point).toEqual(evaluateNurbsCurve(piece.curve,u).point)
    }
  }
  expect(pairs.result).toHaveLength(2)
  const inverse = source.replace('locate_input_parameter','locate_edge_parameter')
    .replace('contour:0,segment:0,exact_parameter:"1/2"',`edge_id:"${ids[0]}",exact_parameter:"${split.result[0].parameter.numerator}/${split.result[0].parameter.denominator}"`)
  expect((await reports(inverse))[0].result).toEqual({contour:0, segment:0, parameter:fraction(1,2)})
})

it('uses the selected world curve as the profile of a new surface', async () => {
  const selected = source.replace(/(query4 = [^\n]+)\)/, '$1,piece:0)')
    .replace('show query4.tessellate(8,4)', 'wall = query4.surface_extrude(vector:[0,0,2mm])\nshow wall.tessellate(8,4)')
  const scene = await buildTextNurbsScene(compileRushFrontend(selected).document)
  expect(scene.surfaceArea).toBeCloseTo(0.5, 7)
  await expect(buildTextNurbsScene(compileRushFrontend(selected.replace('piece:0','piece:2')).document)).rejects.toThrow(/piece index/i)
})

it('withholds all extracted pieces when a rational endpoint cannot be represented exactly',async()=>{
  const [,range,world,pairs]=await reports(source.replaceAll('range_start:"3/4"','range_start:"2/3"'))
  expect(range.result[0].inputParameters[0]).toEqual(fraction(2,3))
  for (const report of [world,pairs]) {
    expect(report.result).toBeNull()
    expect(report.refusal).toBe('NotRepresentable')
  }
})

it('evaluates an extracted face-local curve as editable UV geometry',async()=>{
  const paired=(await reports(source))[3].result[0].pcurve
  const t=(paired.knots[paired.degree]+paired.knots[paired.controlPoints.length])/2
  const program=source.replace(/(query4 = [^\n]+)\)/,'$1,piece:0,use_pcurve:true)')
    .replace('show query4.tessellate(8,4)','show query4')
  const document=compileRushFrontend(program).document as {root:string}
  const evaluated=buildOwnNurbs(document,{action:'evaluate',evaluations:[{node:document.root,u:t}]})
  expect(evaluated.evaluations[0]!.point).toEqual(evaluateNurbsCurve(paired,t).point)
})
