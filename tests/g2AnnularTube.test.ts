import {beforeAll,expect,it} from 'vitest'
import {createG2AnnularTube} from '../src/services/geometricG2'
import {warmGeometryKernel} from '../src/services/geometry/kernel'
import {tessellateNurbsBrep} from '../src/services/geometry/brep'
import {emptyDirectDocument,parseDirectDocument} from '../src/services/directModeling'
import {stringifyMeshJson} from '../src/services/meshJson'
// Actual Rust/WASM qualification. Requires the matching rebuilt geometry kernel.
beforeAll(async()=>{await warmGeometryKernel()})
it.each([[8,4,16,.5],[10,7,20,.5],[5.15,2.3,10.3,.25],[10,7,20,.1]])('qualifies all G2 seams and material for annulus %s/%s/%s/%s and preserves its saved geometry',(outer,inner,height,extent)=>{
 const {model,report}=createG2AnnularTube(outer,inner,height,extent)
 expect(report).toMatchObject({fullBoundaryG2Certified:true,solidGeometryCertified:true,allFacesInjective:true,allPairsClassified:true,totalPairs:496,coveredPairs:496,nextPair:null,profileSeams:32,stationSeams:32,maxWork:2000000})
 expect(report.exactWork).toBeLessThanOrEqual(2000000)
 expect(report.radialWallThicknessLower).toBeGreaterThan(0)
 expect(report.radialWallThicknessLower).toBeLessThanOrEqual(outer-inner-2*extent)
 expect(report.angularRadiusBounds[0]).toBeLessThanOrEqual(1)
 expect(report.angularRadiusBounds[1]).toBeGreaterThanOrEqual(1)
 expect(model.faces).toHaveLength(32);expect(model.edges).toHaveLength(64)
 const points=model.vertices.map(v=>v.point)
 expect([0,1,2].map(a=>[Math.min(...points.map(p=>p[a])),Math.max(...points.map(p=>p[a]))])).toEqual([[-outer,outer],[-outer,outer],[0,height]])
 const mesh=tessellateNurbsBrep(model,4),document=emptyDirectDocument()
 document.bodies.push({id:'g2-ring',name:'G2 ring',brep:model,mesh:{positions:mesh.positions,indices:mesh.indices}})
 const restored=parseDirectDocument(stringifyMeshJson(document)).bodies[0].brep!
 for(const field of ['vertices','edges','loops','faces','shells','bodies'] as const)expect(restored[field]).toEqual(model[field])
})
it('refuses transitions that collapse the cap or radial wall',()=>{
 for(const extent of [0,-.5,1.5,3])expect(()=>createG2AnnularTube(10,7,20,extent)).toThrow()
 expect(()=>createG2AnnularTube(10,7,1,.5)).toThrow()
})
