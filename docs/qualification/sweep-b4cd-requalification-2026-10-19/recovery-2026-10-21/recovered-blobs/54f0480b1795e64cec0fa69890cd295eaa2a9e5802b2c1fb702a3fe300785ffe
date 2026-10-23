import {parseDirectDocument,type DirectDocument} from './directModeling'
import {pushPullFace,bevelBrepBody,shellSolid,splitSolid} from './directSolidTools'
import {solidExactEdgeFeature} from './solidExactEdgeFeature'
import {stringifyMeshJson} from './meshJson'
export type SolidBodyEditOperation='push'|'chamfer'|'edge-fillet'|'shell'|'split'
export interface SolidBodyEditOptions {
 operation:SolidBodyEditOperation;id:string;face:number;edges:number[];openings:number[];segments:number
 distance:number;radius:number;endRadius:number;filletMode:'constant'|'variable'|'corner';axis:'x'|'y'|'z'
}
export function isSolidBodyEdit(operation:unknown):operation is SolidBodyEditOperation {
 return ['push','chamfer','edge-fillet','shell','split'].includes(String(operation))
}
/** Worker-owned snapshot; the source document and its body identities are preserved. */
export function applySolidBodyEdit(document:DirectDocument,p:SolidBodyEditOptions):DirectDocument {
 const d=parseDirectDocument(stringifyMeshJson(document)),index=d.bodies.findIndex(b=>b.id===p.id),b=d.bodies[index]
 if(!b)throw Error('Select an existing body.')
 switch(p.operation){
  case 'push':d.bodies[index]=pushPullFace(b,p.face,p.distance);break
  case 'chamfer':case 'edge-fillet':{
   const kind=p.operation==='chamfer'?'chamfer':'fillet'
   d.bodies[index]=b.brep?solidExactEdgeFeature(b,p.edges,p.radius,kind,'brep',{mode:p.filletMode,endRadius:p.endRadius}).body:bevelBrepBody(b,p.edges,p.radius,kind,p.segments);break
  }
  case 'shell':d.bodies[index]=shellSolid(b,p.openings,p.distance);break
  case 'split':{
   const pair=splitSolid(b,p.axis==='x'?[1,0,0]:p.axis==='y'?[0,1,0]:[0,0,1],p.distance)
   pair[1].id='preview-split';d.bodies[index]=pair[0];d.bodies.push(pair[1]);break
  }
  default:throw Error('Unsupported body edit.')
 }
 return parseDirectDocument(stringifyMeshJson(d))
}
