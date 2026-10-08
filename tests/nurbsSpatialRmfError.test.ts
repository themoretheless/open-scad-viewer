import {expect,it} from 'vitest'
import {callNurbsRust} from '../src/services/geometry/nurbs'
import {bezierNurbsCurve} from '../src/services/nurbsConstructors'
type Proof={continuousBound:boolean;withinBudget:boolean;errorUpper:number|null;cells:number;patches:unknown[]|null;solidCertified:boolean;globalEmbeddingCertified:boolean;capJoinsCertified:boolean}
const constant=(v:number[])=>bezierNurbsCurve([v,v])
function request(){
 const vertices=[[1,0,0],[0,1,1],[-1,0,0],[0,-1,.5]],tangents=[[0,.25,.25],[-.25,0,-.25],[0,-.25,.25],[.25,.125,-.25]]
 const poles=[vertices[0]!]
 for(let i=0;i<4;i++){const j=(i+1)%4;poles.push(vertices[i]!.map((x,k)=>x+tangents[i]![k]!),vertices[j]!.map((x,k)=>x-tangents[j]![k]!),vertices[j]!)}
 const path={...bezierNurbsCurve([[0,0,0],[1,1,1]]),degree:3,controlPoints:poles,weights:Array(13).fill(1),knots:[0,0,0,0,.25,.25,.25,.5,.5,.5,.75,.75,.75,1,1,1,1]}
 return {profile:bezierNurbsCurve([[1.1,0,0],[1.2,0,0]]),path,scale:constant([1.1,0,0]),twist:constant([.125,0,0]),axis_scale:constant([1,1.25,.75]),center_law:constant([.01,-.02,.03]),normal:[1,0,0],orientation:'rmf',spacing:'arc_length',length_tolerance:.01,length_max_cells:100000,initial_sections:5,max_sections:129,max_deviation:.25,preview_sections:129,transport_steps:4096,maxCells:100000,maxProducts:1000000}
}
it('carries explicit spatial RMF precision and refusal through packaged WASM',()=>{
 const args=request(),before=JSON.stringify(args)
 const proof=callNurbsRust<Proof>('surface_progressive_sweep_spatial_rmf_error',args)
 expect(proof).toMatchObject({continuousBound:true,withinBudget:true,solidCertified:false,globalEmbeddingCertified:false,capJoinsCertified:false})
 expect(proof.errorUpper).toBeGreaterThan(0)
 expect(proof.errorUpper).toBeLessThan(.25)
 expect(proof.patches!.length).toBeGreaterThan(0)
 const short=callNurbsRust<Proof>('surface_progressive_sweep_spatial_rmf_error',{...args,maxCells:proof.cells-1})
 expect(short).toMatchObject({continuousBound:false,errorUpper:null,patches:null,solidCertified:false})
 expect(short.cells).toBeLessThanOrEqual(proof.cells-1)
 expect(()=>callNurbsRust('surface_progressive_sweep_spatial_rmf_error',{...args,transport_steps:4095})).toThrow()
 expect(JSON.stringify(args)).toBe(before)
})

it.each(['parameter','arc_length'])('certifies closed rational nonuniform RMF through packaged WASM: %s',spacing=>{
 const base=request()
 const path={...base.path,knots:[0,0,0,0,.125,.125,.125,.375,.375,.375,.75,.75,.75,1,1,1,1],
  controlPoints:[[1,0,0],[1,.125,.125],[.125,1,1.125],[0,1,1],[-.25,1,.75],[-1,.25,-.25],[-1,0,0],
   [-1,-.375,.375],[-.375,-1.1875,.875],[0,-1,.5],[.25,-.875,.25],[1,-.25,-.25],[1,0,0]],
  weights:[1,1.25,1.25,1,1.25,1.25,1,1.25,1.25,1,1.25,1.25,1]}
 const args={...base,path,spacing,initial_sections:65,max_sections:65,preview_sections:65,max_deviation:2,length_tolerance:.001}
 const before=JSON.stringify(args)
 const proof=callNurbsRust<Proof>('surface_progressive_sweep_spatial_rmf_error',args)
 expect(proof).toMatchObject({continuousBound:true,withinBudget:true,solidCertified:false,globalEmbeddingCertified:false,capJoinsCertified:false})
 expect(proof.errorUpper).toBeGreaterThan(0)
 expect(proof.errorUpper).toBeLessThan(2)
 expect(proof.patches!.length).toBeGreaterThan(0)
 expect(callNurbsRust<Proof>('surface_progressive_sweep_spatial_rmf_error',{...args,maxCells:0})).toMatchObject({continuousBound:false,errorUpper:null,patches:null})
 const broken=structuredClone(path);broken.weights[1]=1.2500000000000002
 expect(callNurbsRust<Proof>('surface_progressive_sweep_spatial_rmf_error',{...args,path:broken})).toMatchObject({continuousBound:false,errorUpper:null,patches:null})
 expect(JSON.stringify(args)).toBe(before)
})

it.each(['parameter','arc_length'])('encloses antipodal closed RMF holonomy without bypassing tolerance: %s',spacing=>{
 const directions=[[1,0,0],[0,1,0],[0,0,1],[1,0,0],[0,1,0],[-1,0,0],[0,0,-1],[0,-1,0],[-1,0,0],[0,-1,0],[1,0,0]]
 let position=[0,0,0];const poles=[position]
 for(let i=0;i<10;i++){
  poles.push(position.map((x,k)=>x+directions[i]![k]!))
  position=position.map((x,k)=>x+directions[i]![k]!+directions[i+1]![k]!)
  poles.push(position)
 }
 const path={degree:2,knots:[0,0,0,...Array.from({length:9},(_,i)=>[i+1,i+1]).flat(),10,10,10],controlPoints:poles,weights:Array(21).fill(1),periodic:false}
 const args={...request(),path,normal:[0,0,1],profile:bezierNurbsCurve([[.01,0,0],[.02,0,0]]),
  scale:constant([1,0,0]),twist:constant([0,0,0]),axis_scale:constant([1,1,1]),center_law:constant([0,0,0]),
  spacing,initial_sections:65,max_sections:65,preview_sections:65,max_deviation:1,length_tolerance:.001}
 const before=JSON.stringify(args)
 const bound=callNurbsRust<Proof>('surface_progressive_sweep_spatial_rmf_error',args)
 expect(bound).toMatchObject({continuousBound:true,withinBudget:true,solidCertified:false,globalEmbeddingCertified:false,capJoinsCertified:false})
 expect(bound.errorUpper).toBeGreaterThan(1e-6)
 expect(bound.errorUpper).toBeLessThan(1)
 const strict=callNurbsRust<Proof>('surface_progressive_sweep_spatial_rmf_error',{...args,max_deviation:1e-6})
 expect(strict).toMatchObject({continuousBound:true,withinBudget:false,errorUpper:bound.errorUpper,solidCertified:false})
 expect(callNurbsRust<Proof>('surface_progressive_sweep_spatial_rmf_error',{...args,maxCells:0})).toMatchObject({continuousBound:false,errorUpper:null,patches:null})
 const broken=structuredClone(path);broken.controlPoints[1]![0]=1.0000000000000002
 expect(callNurbsRust<Proof>('surface_progressive_sweep_spatial_rmf_error',{...args,path:broken})).toMatchObject({continuousBound:false,errorUpper:null,patches:null})
 expect(JSON.stringify(args)).toBe(before)
})

it('certifies parameter spatial RMF and charges copied seam through packaged WASM',()=>{
 const args={...request(),spacing:'parameter'},before=JSON.stringify(args)
 const proof=callNurbsRust<Proof&{scope:string}>('surface_progressive_sweep_spatial_rmf_error',args)
 expect(proof).toMatchObject({continuousBound:true,withinBudget:true,solidCertified:false,scope:'retained-patches-relative-to-original-spatial-rmf-parameter-transport'})
 expect(proof.errorUpper).toBeGreaterThan(0)
 expect(proof.errorUpper).toBeLessThan(.25)
 expect(proof.patches!.length).toBeGreaterThan(0)
 expect(callNurbsRust<Proof>('surface_progressive_sweep_spatial_rmf_error',{...args,maxCells:proof.cells-1})).toMatchObject({continuousBound:false,errorUpper:null,patches:null})
 expect(JSON.stringify(args)).toBe(before)
})

it('carries antipodal closed RMF correction through Rush and viewport with strict refusal',async()=>{
 const {readFileSync}=await import('node:fs')
 const {compileRushFrontend}=await import('../src/services/rushFrontend')
 const {buildOwnNurbsAsync}=await import('../src/services/rushGraphNurbsKernel')
 const {readSweepPatchViewportEvidence}=await import('../src/services/sweepViewportEvidence')
 const source=readFileSync('examples/rush/closed-antipodal-spatial-rmf-progressive-sweep.r','utf8')
 const document=compileRushFrontend(source).document
 const built=await buildOwnNurbsAsync(document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 const node=document.nodes.find(n=>n.op==='progressive_sweep')!
 expect(built.report.construction![node.id]).toMatchObject({accepted:true,continuousBound:true,closedPath:true,seamContinuity:'C0'})
 if(!('nativeGeometry' in built))throw Error('Missing native geometry')
 expect(readSweepPatchViewportEvidence(built.nativeGeometry)).toMatchObject({continuousBound:true,budget:1})
 await expect(buildOwnNurbsAsync(compileRushFrontend(source.replace('max_deviation: 1mm','max_deviation: 0.000001mm')).document,{action:'build'})).rejects.toThrow(/continuous retained-patch error/)
})

it('carries closed spatial RMF through Rush to viewport with strict refusal',async()=>{
 const {readFileSync}=await import('node:fs')
 const {compileRushFrontend}=await import('../src/services/rushFrontend')
 const {buildOwnNurbsAsync}=await import('../src/services/rushGraphNurbsKernel')
 const {readSweepPatchViewportEvidence}=await import('../src/services/sweepViewportEvidence')
 const source=readFileSync('examples/rush/closed-spatial-rmf-affine-progressive-sweep.r','utf8')
 const document=compileRushFrontend(source).document
 const built=await buildOwnNurbsAsync(document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 const node=document.nodes.find(n=>n.op==='progressive_sweep')!
 expect(built.report.construction![node.id]).toMatchObject({accepted:true,continuousBound:true,closedPath:true,seamContinuity:'C0'})
 if(!('nativeGeometry' in built))throw Error('Missing native geometry')
 expect(readSweepPatchViewportEvidence(built.nativeGeometry)).toMatchObject({continuousBound:true,budget:3})
 await expect(buildOwnNurbsAsync(compileRushFrontend(source.replace('max_deviation: 3mm','max_deviation: 0.001mm')).document,{action:'build'})).rejects.toThrow(/continuous retained-patch error/)
})

it('uses explicit RMF precision options in ordinary Rush construction',async()=>{
 const {readFileSync}=await import('node:fs')
 const {compileRushFrontend}=await import('../src/services/rushFrontend')
 const {buildOwnNurbsAsync}=await import('../src/services/rushGraphNurbsKernel')
 const {readSweepPatchViewportEvidence}=await import('../src/services/sweepViewportEvidence')
 const source=readFileSync('examples/rush/closed-spatial-rmf-affine-progressive-sweep.r','utf8')
  .replace('initial_sections: 5,max_sections: 17','initial_sections: 129,max_sections: 129')
  .replace('length_tolerance: 0.01mm','length_tolerance: 0.0001mm')
  .replace('max_deviation: 3mm','max_deviation: 0.25mm')
  .replace('orientation: "rmf"','orientation: "rmf",rmf_transport_steps: 4096,error_max_cells: 100000,error_max_products: 1000000')
 const document=compileRushFrontend(source).document
 const node=document.nodes.find(n=>n.op==='progressive_sweep')!
 expect(node).toMatchObject({rmf_transport_steps:4096,error_max_cells:100000,error_max_products:1000000})
 const built=await buildOwnNurbsAsync(document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 expect(built.report.construction![node.id]).toMatchObject({accepted:true,continuousBound:true,budget:.25,sections:129})
 if(!('nativeGeometry' in built))throw Error('Missing native geometry')
 expect(readSweepPatchViewportEvidence(built.nativeGeometry)).toMatchObject({continuousBound:true,budget:.25})
 for(const value of ['4095','4096mm'])expect(()=>compileRushFrontend(source.replace('rmf_transport_steps: 4096',`rmf_transport_steps: ${value}`))).toThrow()
 await expect(buildOwnNurbsAsync(compileRushFrontend(source.replace('error_max_cells: 100000','error_max_cells: 0')).document,{action:'build'})).rejects.toThrow(/continuous retained-patch error/)
})

it.each(['closed-rational-nonuniform-spatial-rmf-affine-hollow-body.r','closed-spatial-rmf-affine-body-certified.r','closed-periodic-spatial-rmf-affine-body-certified.r','open-spatial-rmf-affine-corrected-body-certified.r','open-spatial-rmf-varying-affine-hollow-corrected-body.r','open-spatial-rmf-parameter-varying-affine-hollow-corrected-body.r'])('owns spatial RMF body error through Rush and recomputes Solid admission: %s',async(file)=>{
 const {readFileSync}=await import('node:fs')
 const {compileRushFrontend}=await import('../src/services/rushFrontend')
 const {buildOwnNurbsAsync}=await import('../src/services/rushGraphNurbsKernel')
 const {readSweepBodyBoundaryViewportEvidence}=await import('../src/services/sweepViewportEvidence')
 const {inspectProgressiveSweepSolidAdmission}=await import('../src/services/sweepSolidAdmission')
 const source=readFileSync(`examples/rush/${file}`,'utf8')
 const document=compileRushFrontend(source).document
 const before=JSON.stringify(document)
 expect(document.nodes.find(n=>n.op==='brep_progressive_sweep')).toMatchObject({rmf_transport_steps:4096,error_max_cells:100000,error_max_products:1000000})
 const built=await buildOwnNurbsAsync(document,{action:'build',display:{segments:file.includes('hollow')?2:4,subdivisionLevels:0}})
 if(!('nativeGeometry' in built))throw Error('Missing retained spatial RMF body')
 expect(readSweepBodyBoundaryViewportEvidence(built.nativeGeometry)).toMatchObject({continuousBound:true,withinBudget:true,budget:file.startsWith('closed-rational-nonuniform-')?.4:.25})
 const model=JSON.parse(built.nativeGeometry.geometryJson).geometry
 const faces=file.startsWith('open-')&&file.includes('hollow')?1010:file.startsWith('open-')?514:512
 expect(model.faces).toHaveLength(faces)
 if(file.startsWith('open-')&&file.includes('hollow'))expect(model.faces.slice(-2).map((f:{holes:unknown[]})=>f.holes.length)).toEqual([1,1])
 const volume=inspectProgressiveSweepSolidAdmission(built.nativeGeometry,model)
 expect(volume).toMatchObject({solidGeometryCertified:true,boundaryEmbeddingCertified:true,allFacesInjective:true,allPairsClassified:true,nextPair:null})
 expect(volume!.nesting?.rolesConsistent).toBe(true)
 expect(volume!.individualPairs+volume!.groupedPairs).toBe(faces*(faces-1)/2)
 expect(JSON.stringify(document)).toBe(before)
 if(file.startsWith('closed-rational-nonuniform-')){
  expect(model.shells).toHaveLength(2)
  expect(model.bodies[0].innerShells).toEqual([1])
  expect(volume).toMatchObject({nesting:{rolesConsistent:true,parents:[null,0]}})
  expect(volume!.orientations.map(o=>[o.expectedOutward,o.outward])).toEqual([[true,true],[false,false]])
  await expect(buildOwnNurbsAsync(compileRushFrontend(source.replace('max_deviation: 0.4mm','max_deviation: 0.25mm')).document,{action:'build'})).rejects.toThrow(/continuous retained-patch error/)
 }
 if(file.startsWith('open-'))await expect(buildOwnNurbsAsync(compileRushFrontend(source.replace('cap_correction_max_work: 1000000','cap_correction_max_work: 1')).document,{action:'build'})).rejects.toThrow(/correction/)
 if(file.includes('periodic'))await expect(buildOwnNurbsAsync(compileRushFrontend(source.replace('knots: [0,1,2','knots: [0,1.0000000000000002,2')).document,{action:'build'})).rejects.toThrow(/continuous retained-patch error/)
 await expect(buildOwnNurbsAsync(compileRushFrontend(source.replace('error_max_cells: 100000','error_max_cells: 0')).document,{action:'build'})).rejects.toThrow()
 expect(()=>compileRushFrontend(source.replace('rmf_transport_steps: 4096','rmf_transport_steps: 4095'))).toThrow()
})

it.each(['closed-spatial-rmf-affine-hollow-body-certified.r','closed-spatial-rmf-varying-affine-hollow-body.r','closed-periodic-spatial-rmf-varying-affine-hollow-body.r','closed-spatial-rmf-parameter-varying-affine-hollow-body.r','closed-periodic-spatial-rmf-parameter-varying-affine-hollow-body.r','closed-periodic-spatial-rmf-multiple-holes-body.r','closed-periodic-spatial-rmf-parameter-multiple-holes-body.r'])('certifies spatial RMF shell roles and refuses exhausted global work: %s',async(file)=>{
 const {readFileSync}=await import('node:fs')
 const {compileRushFrontend}=await import('../src/services/rushFrontend')
 const {buildOwnNurbsAsync}=await import('../src/services/rushGraphNurbsKernel')
 const {readSweepBodyBoundaryViewportEvidence}=await import('../src/services/sweepViewportEvidence')
 const {inspectProgressiveSweepSolidAdmission}=await import('../src/services/sweepSolidAdmission')
 const {inspectSweepVolume,DEFAULT_SWEEP_VOLUME_BUDGETS}=await import('../src/services/nurbsSweepEmbedding')
 const source=readFileSync(`examples/rush/${file}`,'utf8')
 const built=await buildOwnNurbsAsync(compileRushFrontend(source).document,{action:'build',display:{segments:2,subdivisionLevels:0}})
 if(!('nativeGeometry' in built))throw Error('Missing retained hollow spatial RMF body')
 expect(readSweepBodyBoundaryViewportEvidence(built.nativeGeometry)).toMatchObject({continuousBound:true,withinBudget:true,budget:.25})
 const model=JSON.parse(built.nativeGeometry.geometryJson).geometry
 const holes=file.includes('multiple-holes')?3:1
 expect(model.faces).toHaveLength(1024)
 expect(model.shells).toHaveLength(holes+1)
 expect(model.bodies[0].innerShells).toEqual(Array.from({length:holes},(_,i)=>i+1))
 const volume=inspectProgressiveSweepSolidAdmission(built.nativeGeometry,model)!
 expect(volume).toMatchObject({solidGeometryCertified:true,allFacesInjective:true,allPairsClassified:true,nextPair:null,nesting:{rolesConsistent:true,parents:[null,...Array(holes).fill(0)]}})
 expect(volume.individualPairs+volume.groupedPairs).toBe(1024*1023/2)
 expect(volume.orientations.map(o=>[o.expectedOutward,o.outward])).toEqual([[true,true],...Array.from({length:holes},()=>[false,false])])
 expect(inspectSweepVolume(model,[],{...DEFAULT_SWEEP_VOLUME_BUDGETS,maxPairs:1,maxCells:1})).toMatchObject({solidGeometryCertified:false,boundaryEmbeddingCertified:false})
})
