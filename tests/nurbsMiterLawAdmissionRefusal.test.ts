import {expect,it} from 'vitest'
import {miterLawMatrixModes,miterLawMatrixSource} from '../scripts/miter-law-matrix-sources'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbsAsync} from '../src/services/rushGraphNurbsKernel'
import {inspectProgressiveSweepSolidAdmission} from '../src/services/sweepSolidAdmission'
import {DEFAULT_SWEEP_VOLUME_BUDGETS,inspectSweepVolume} from '../src/services/nurbsSweepEmbedding'

it.each(miterLawMatrixModes)('refuses exhausted or changed actual miter bodies in %j',async mode=>{
 const built=await buildOwnNurbsAsync(compileRushFrontend(miterLawMatrixSource(mode)).document,
  {action:'build',display:{segments:2,subdivisionLevels:0}})
 if(!built.nativeGeometry)throw Error('Missing actual native miter body')
 const model=JSON.parse(built.nativeGeometry.geometryJson).geometry
 const saved=JSON.stringify(model),caps=mode.closed?[]:[model.faces.length-2,model.faces.length-1]
 expect(inspectProgressiveSweepSolidAdmission(built.nativeGeometry,model)?.solidGeometryCertified).toBe(true)
 expect(()=>inspectSweepVolume(model,caps,{...DEFAULT_SWEEP_VOLUME_BUDGETS,maxExactWork:0})).toThrow(/Exact boundary work/)
 const exhausted=inspectSweepVolume(model,caps,{...DEFAULT_SWEEP_VOLUME_BUDGETS,maxExactWork:1,maxCells:1,maxDomainCells:1})
 expect(exhausted.solidGeometryCertified).toBe(false)
 expect(exhausted.boundaryEmbeddingCertified).toBe(false)
 expect(exhausted.nesting).toBeNull()
 const damaged=JSON.parse(saved)
 // Alter an actual represented coefficient, even when the visual change is
 // below any tessellation tolerance. Source replay binding is exact.
 const point=damaged.faces[0].surface.controlPoints[0][0]
 const bits=new DataView(new ArrayBuffer(8))
 bits.setFloat64(0,point[0])
 bits.setBigUint64(0,bits.getBigUint64(0)^1n)
 point[0]=bits.getFloat64(0)
 expect(point[0]).not.toBe(model.faces[0].surface.controlPoints[0][0][0])
 expect(()=>inspectProgressiveSweepSolidAdmission(built.nativeGeometry,damaged)).toThrow()
 expect(JSON.stringify(model)).toBe(saved)
})
