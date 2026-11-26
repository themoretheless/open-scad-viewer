import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {inspectNurbsBrep} from '../src/kernel/geometry/brep'
import {readNativeGeometryData} from '../src/kernel/nativeGeometrySnapshot'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

it('sews a six-face shell on either side of the original panel',async()=>{
  const source=readFileSync('examples/rush/thicken-nurbs-panel.r','utf8')
  for(const [literal,min,max] of [['4mm',0,4],['-0.4cm',-4,0]] as const){
    const scene=await buildTextNurbsScene(compileRushFrontend(source.replace('thickness_mm:4mm','thickness_mm:'+literal)).document)
    const report=JSON.parse(scene.measurements!.at(-1)!.report)
    expect(report.stop).toBe('ConstructedSampledThickness')
    expect(report.maxSampledThicknessDeviationMm).toBeLessThan(1e-8)
    const model=readNativeGeometryData(scene.meshes[0]!.nativeGeometry!).geometry
    expect(model.faces).toHaveLength(6)
    expect(inspectNurbsBrep(model)).toMatchObject({topologyValid:true,boundaryEdgeCount:0})
    const zs=model.vertices.map(vertex=>vertex.point[2])
    expect(Math.min(...zs)).toBeCloseTo(min,8)
    expect(Math.max(...zs)).toBeCloseTo(max,8)
    expect(scene.volume).toBeCloseTo(24,8)
  }
},180000)
