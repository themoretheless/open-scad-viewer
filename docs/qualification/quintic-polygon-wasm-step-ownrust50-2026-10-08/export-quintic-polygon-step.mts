// Only transport: authored inputs, construction, audits, evaluation and STEP are Rust.
import {mkdirSync,readFileSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {gunzipSync} from 'node:zlib'
import {createHash} from 'node:crypto'
import {callGeometryRust} from '../src/services/geometry/kernel'
import type {NurbsBrep} from '../src/services/geometry/brep'
import {evaluateNurbsCurve} from '../src/services/nurbsCurve'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {exportDirectStepV5,importDirectStepV5} from '../src/services/cadNurbsStep'
import {sweepStepArtifactProvenance} from './sweep-step-provenance.mjs'
const root=resolve(process.argv[2]??'/tmp/quintic-polygon-wasm-step')
mkdirSync(root,{recursive:true})
const sha=(data:string|Buffer)=>createHash('sha256').update(data).digest('hex')
const cases=[]
const inputs=[]
for(const count of [6,7,10,12]) {
 const path=`docs/qualification/quintic-polygon-ownrust50-2026-10-08/step-${count}/native-bundle.json.gz`
 const source=readFileSync(path)
 inputs.push({path,sha256:sha(source)})
 const bundle=JSON.parse(gunzipSync(source).toString('utf8'))
 for(const input of bundle.cases) {
  const body=callGeometryRust<{accepted:boolean;model:NurbsBrep;stationContinuity:string;volume:unknown;station:unknown}>(
   'brep_nurbs_smooth_polygon_station_body',input.request)
  if(!body.accepted||body.stationContinuity!=='G2')throw Error(`Unproved body ${count}`)
  const model=body.model,text=exportDirectStepV5(model).text
  // Exercise the delivered Rust importer too; independent OCCT checks follow.
  const imported=importDirectStepV5(text)
  const mode=input.spatial?'spatial':'planar',weight=input.weighted?'weighted':'unit'
  const file=`quintic-${mode}-${weight}-${count}.step`
  writeFileSync(resolve(root,file),text)
  writeFileSync(resolve(root,file+'.import.json'),JSON.stringify(imported)+'\n')
  cases.push({file,mode,stations:count,weighted:input.weighted,
   faces:model.faces.length,solids:1,shells:1,capHoleFaces:0,capFaces:[],requireNativeSolid:true,
   nativeVolume:body.volume,nativeStation:body.station,sourceShells:model.shells,
   edges:model.edges.length,faceLoops:model.faces.map(face=>({outer:model.loops[face.outer]!.coedges.map(c=>c.edge),holes:[]})),
   edgeCurves:model.edges.map(edge=>({curve:edge.curve,samples:[0,.25,.5,.75,1].map(u=>({u,point:evaluateNurbsCurve(edge.curve,u).point}))})),
   wallCoedges:model.faces.map(face=>model.loops[face.outer]!.coedges.map(c=>({edge:c.edge,reversed:c.reversed,pcurve:c.pcurve}))),
   wallSurfaces:model.faces.map(face=>face.surface),wallSamples:model.faces.map(face=>[0,.17,.5,.83,1].flatMap(u=>[0,.13,.5,.87,1].map(v=>{const e=evaluateNurbsSurface(face.surface,u,v);return {u,v,point:e.point,du:e.du,dv:e.dv}}))),
   surfaceToleranceMm:1e-8,relativeVolumeTolerance:1e-7,sha256:sha(text),expectedVolume:null})
  console.log(`Packaged WASM STEP exported/imported: ${file}`)
 }
}
writeFileSync(resolve(root,'manifest.json'),JSON.stringify({schema:'sweep-external-step/2',units:'mm',inputs,
 artifactProvenance:sweepStepArtifactProvenance(),exporterSha256:sha(readFileSync(new URL(import.meta.url))),cases},null,2)+'\n')
