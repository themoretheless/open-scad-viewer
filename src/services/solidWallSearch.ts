import type {NurbsBrep} from './geometry/brep'
import {evaluateNurbsSurface,type NurbsSurface,type NurbsSurfaceEvaluation} from './nurbsSurface'
type Point=[number,number,number]
export interface WallSearchCandidate {face:number;uv:[number,number];origin:Point;direction:Point}
/** Candidate generation only. Each line still needs the original trimmed-volume audit.
 * Samples never certify coverage between sample locations. */
export function wallSearchCandidates(model:NurbsBrep,groups:[number[],number[]],evaluate:(s:NurbsSurface,u:number,v:number)=>NurbsSurfaceEvaluation=evaluateNurbsSurface,maxCandidates=64):WallSearchCandidate[]{
 if(!Number.isSafeInteger(maxCandidates)||maxCandidates<1||maxCandidates>256)throw Error('Invalid wall search budget')
 if(!groups.every(g=>g.length>0&&new Set(g).size===g.length&&g.every(i=>Number.isSafeInteger(i)&&i>=0&&i<model.faces.length))||groups[0].some(i=>groups[1].includes(i)))throw Error('Invalid wall search groups')
 const points=model.faces.flatMap(f=>f.surface.controlPoints.flat())
 const bounds=[0,1,2].map(k=>[Math.min(...points.map(p=>p[k])),Math.max(...points.map(p=>p[k]))])
 const diagonal=Math.hypot(...bounds.map(([lo,hi])=>hi-lo))
 if(!Number.isFinite(diagonal)||diagonal<=0)throw Error('Invalid wall search bounds')
 const domain=(s:NurbsSurface):[[number,number],[number,number]]=>[[s.knotsU[s.degreeU],s.knotsU[s.knotsU.length-s.degreeU-1]],[s.knotsV[s.degreeV],s.knotsV[s.knotsV.length-s.degreeV-1]]]
 const at=(s:NurbsSurface,a:number,b:number)=>{const d=domain(s);const uv:[number,number]=[d[0][0]*(1-a)+d[0][1]*a,d[1][0]*(1-b)+d[1][1]*b];return {uv,value:evaluate(s,...uv)}}
 const targets=groups[1].map(i=>at(model.faces[i].surface,.5,.5).value.point)
 const result:WallSearchCandidate[]=[]
 // Interleave faces so a small budget cannot consume every sample on the first face.
 for(const a of [.5,.25,.75])for(const b of [.5,.25,.75])for(const face of groups[0]){
  const {uv,value}=at(model.faces[face].surface,a,b),n=value.normal,p=value.point
  if(!n||!n.every(Number.isFinite)||!p.every(Number.isFinite))continue
  const length=Math.hypot(...n);if(length<=0)continue
  const unit=n.map(x=>x/length)
  const projections=targets.map(t=>t.reduce((sum,x,k)=>sum+(x-p[k])*unit[k],0)).filter(x=>Number.isFinite(x)&&Math.abs(x)>model.toleranceMm)
  if(!projections.length)continue
  const projection=projections.reduce((best,x)=>Math.abs(x)<Math.abs(best)?x:best)
  const sign=Math.sign(projection),pad=Math.max(model.toleranceMm*10,Math.abs(projection)*.05)
  const ray=unit.map(x=>x*sign),offset=2*diagonal
  result.push({face,uv,origin:p.map((x,k)=>x-ray[k]*offset) as Point,direction:ray.map(x=>x*(offset+Math.abs(projection)+pad)) as Point})
  if(result.length===maxCandidates)return result
 }
 return result
}
