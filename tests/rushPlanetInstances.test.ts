import {hasSafeSuccessPayload} from '../src/kernel/geometryWorkerProtocol/isGeometryEvaluationResultPayload'
import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
import {readNativeGeometryData} from '../src/kernel/nativeGeometrySnapshot'

async function build(example:string, index=9) {
  const source=readFileSync('examples/rush/'+example+'.r','utf8').replace('index:9','index:'+index)
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  const model=readNativeGeometryData(scene.meshes[0]!.nativeGeometry!).geometry
  const measurement=scene.measurements![0]!
  return {model,report:JSON.parse(measurement.report),step:measurement.step}
}
it('places eighteen planet occurrences from one definition and returns the selected closed gear',async()=>{
  const {model,report}=await build('planet-gear-occurrences')
  expect(report.definitionCount).toBe(1)
  expect(report.placements).toHaveLength(18)
  expect(report.placements[9][0][3]).toBeCloseTo(-27)
  expect(report.placements[9][1][3]).toBeCloseTo(0)
  expect(model.faces).toHaveLength(50)
  expect(model.shells[0].closed).toBe(true)
  expect(model.vertices.every(v=>v.point[0]<0)).toBe(true)
  await expect(build('planet-gear-occurrences',18)).rejects.toThrow(/index|range/i)
},120000)
it('exports the edited placements as shared AP242 part definitions',async()=>{
  for(const [name,definitions,occurrences] of [['planet-occurrences-step',1,18],['planetary-gear-set-step',3,20]] as const){
    const {report,step}=await build(name)
    expect(report.definitionCount).toBe(definitions)
    expect(report.occurrenceCount).toBe(occurrences)
    expect(step).toMatch(/^ISO-10303-21;/)
    const measurement={node:'assembly',operation:'planet_occurrences_step',report:'{}',step}
    const result={meshes:[],warnings:[],measurements:[measurement]}
    expect(hasSafeSuccessPayload(result)).toBe(true)
    expect(hasSafeSuccessPayload({...result,measurements:[{...measurement,step:12}]})).toBe(false)
    expect(step!.match(/MANIFOLD_SOLID_BREP\(/g)).toHaveLength(definitions)
    expect(step!.match(/NEXT_ASSEMBLY_USAGE_OCCURRENCE\(/g)).toHaveLength(occurrences)
    if (name === 'planetary-gear-set-step') {
      const directions=step!.split(/\r?\n/).flatMap(line=>{
        const match=/^#\d+=(DIRECTION\(.+)$/.exec(line)
        return match ? [match[1]] : []
      })
      expect(directions.length).toBeGreaterThanOrEqual(3)
      expect(directions.length).toBeLessThan(2*occurrences)
      expect(new Set(directions).size).toBe(directions.length)
      expect(step!.match(/ITEM_DEFINED_TRANSFORMATION\(/g)).toHaveLength(occurrences)
    }
  }
},120000)
