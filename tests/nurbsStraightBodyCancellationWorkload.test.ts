import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbsAsync} from '../src/services/rushGraphNurbsKernel'
import {readSweepBodyBoundaryViewportEvidence} from '../src/services/sweepViewportEvidence'
import {inspectProgressiveSweepSolidAdmission} from '../src/services/sweepSolidAdmission'

it.each([{file:'progressive-hollow-boundary.r',faces:514},{file:'progressive-unsegmented-hollow-boundary.r',faces:578},{file:'corrected-frenet-straight-affine-hollow-body-boundary.r',faces:258},
 ...['corrected-frenet-rational-straight-affine-hollow-body-boundary.r','arc-length-corrected-frenet-rational-straight-affine-hollow-body-boundary.r','rmf-rational-straight-affine-hollow-body-boundary.r','arc-length-rmf-rational-straight-affine-hollow-body-boundary.r'].map(file=>({file,faces:514}))])(
 'qualifies a valid larger native cancellation body for $file',async({file,faces})=>{
 const original=readFileSync(`examples/rush/${file}`,'utf8')
 const source=original.replace(/initial_sections: 3,max_sections: (3|129)/,file==='corrected-frenet-straight-affine-hollow-body-boundary.r'?'initial_sections: 33,max_sections: 33':'initial_sections: 65,max_sections: 65')
 expect(source).not.toBe(original)
 const built=await buildOwnNurbsAsync(compileRushFrontend(source).document,{action:'build',display:{segments:2,subdivisionLevels:0}})
 if(!built.nativeGeometry)throw Error('Missing valid native cancellation body')
 expect(readSweepBodyBoundaryViewportEvidence(built.nativeGeometry)).toMatchObject({continuousBound:true,withinBudget:true,budget:.01})
 const model=JSON.parse(built.nativeGeometry.geometryJson).geometry,saved=JSON.stringify(model)
 expect(model.faces).toHaveLength(faces)
 expect(inspectProgressiveSweepSolidAdmission(built.nativeGeometry,model)).toMatchObject({solidGeometryCertified:true,
  allFacesInjective:true,allPairsClassified:true,nextPair:null,nesting:{rolesConsistent:true}})
 expect(JSON.stringify(model)).toBe(saved)
})
