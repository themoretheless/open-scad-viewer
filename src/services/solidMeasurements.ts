import type {DirectBody} from './directModeling'
import {analyzeNurbsBrep} from './geometry/brep'
import {callGeometryRust} from './geometry/kernel'
export interface PointMeasurement {a:number[];b:number[];deltaMm:number[];distanceMm:number}
export interface CurveMeasurement {point:number[];curvaturePerMm:number;radiusMm:number|null;parameter:number}
export interface SolidMassMeasurement {
 volumeMm3:number
 surfaceAreaMm2:number
 surfaceToVolumeRatioPerMm:number
 centerOfMass:[number,number,number]
 bounds:{min:[number,number,number];max:[number,number,number]}
 dimensions:[number,number,number]
 principalInertiaMm5:[number,number,number]
}
export const solidVertexCount=(body:DirectBody)=>body.brep?body.brep.vertices.length:body.mesh.positions.length/3
function vertex(body:DirectBody,index:number) {
 if(!Number.isInteger(index)||index<0||index>=solidVertexCount(body))throw Error('Choose an existing vertex.')
 if(body.brep)return body.brep.vertices[index].point
 const p=body.mesh.positions,o=index*3
 return [p[o],p[o+1],p[o+2]]
}
export const measureSolidVertices=(a:DirectBody,ia:number,b:DirectBody,ib:number):PointMeasurement=>callGeometryRust('cad_measure_points',{a:vertex(a,ia),b:vertex(b,ib)})
export function measureSolidEdgeCurvature(body:DirectBody,edge:number,t:number):CurveMeasurement {
 if(!body.brep?.edges[edge])throw Error('Choose a B-rep edge.')
 return callGeometryRust('cad_curve_curvature',{curve:body.brep.edges[edge].curve,t})
}
function symmetricEigenvalues3x3(m:readonly [readonly [number,number,number],readonly [number,number,number],readonly [number,number,number]]):[number,number,number] {
 const a=[[m[0][0],m[0][1],m[0][2]],[m[1][0],m[1][1],m[1][2]],[m[2][0],m[2][1],m[2][2]]]
 for(let iter=0;iter<24;iter++){
  let p=0,q=1,max=Math.abs(a[0][1])
  for(const [r,c] of [[0,2],[1,2]] as const){
   const v=Math.abs(a[r][c])
   if(v>max){max=v;p=r;q=c}
  }
  if(max<=1e-12)break
  const diff=a[q][q]-a[p][p],apq=a[p][q]
  const t=Math.abs(diff)<=1e-15?1:diff/(2*apq),tau=Math.sign(t||1)/(Math.abs(t)+Math.hypot(1,t))
  const c=1/Math.hypot(1,tau),s=tau*c
  const app=a[p][p],aqq=a[q][q]
  a[p][p]=app-tau*apq;a[q][q]=aqq+tau*apq;a[p][q]=0;a[q][p]=0
  const r=3-p-q,arp=a[r][p],arq=a[r][q]
  a[r][p]=c*arp-s*arq;a[p][r]=a[r][p]
  a[r][q]=s*arp+c*arq;a[q][r]=a[r][q]
 }
 const vals:[number,number,number]=[Math.max(0,a[0][0]),Math.max(0,a[1][1]),Math.max(0,a[2][2])]
 vals.sort((x,y)=>x-y)
 return vals
}
export function measureSolidMassProperties(body:DirectBody):SolidMassMeasurement {
 const pos=body.mesh.positions,idx=body.mesh.indices
 if(pos.length<9||pos.length%3!==0)throw Error('Body has no valid mesh positions.')
 let minX=Infinity,minY=Infinity,minZ=Infinity,maxX=-Infinity,maxY=-Infinity,maxZ=-Infinity
 for(let i=0;i<pos.length;i+=3){
  const x=pos[i],y=pos[i+1],z=pos[i+2]
  if(x<minX)minX=x;if(x>maxX)maxX=x
  if(y<minY)minY=y;if(y>maxY)maxY=y
  if(z<minZ)minZ=z;if(z>maxZ)maxZ=z
 }
 if(body.brep&&body.brep.faces.length>0){
  const analysis=analyzeNurbsBrep(body.brep)
  const volumeMm3=Math.abs(analysis.signedVolumeMm3)
  const surfaceAreaMm2=analysis.surfaceAreaMm2
  const bMin:[number,number,number]=[Math.min(minX,analysis.conservativeBounds[0][0]),Math.min(minY,analysis.conservativeBounds[0][1]),Math.min(minZ,analysis.conservativeBounds[0][2])]
  const bMax:[number,number,number]=[Math.max(maxX,analysis.conservativeBounds[1][0]),Math.max(maxY,analysis.conservativeBounds[1][1]),Math.max(maxZ,analysis.conservativeBounds[1][2])]
  return {
   volumeMm3,
   surfaceAreaMm2,
   surfaceToVolumeRatioPerMm:volumeMm3>1e-12?surfaceAreaMm2/volumeMm3:0,
   centerOfMass:[...analysis.centroid],
   bounds:{min:[minX,minY,minZ],max:[maxX,maxY,maxZ]},
   dimensions:[Math.max(0,maxX-minX||bMax[0]-bMin[0]),Math.max(0,maxY-minY||bMax[1]-bMin[1]),Math.max(0,maxZ-minZ||bMax[2]-bMin[2])],
   principalInertiaMm5:symmetricEigenvalues3x3(analysis.inertiaMm5),
  }
 }
 let vol6=0,area2=0,cx=0,cy=0,cz=0
 for(let i=0;i+2<idx.length;i+=3){
  const i0=idx[i]*3,i1=idx[i+1]*3,i2=idx[i+2]*3
  const x0=pos[i0],y0=pos[i0+1],z0=pos[i0+2]
  const x1=pos[i1],y1=pos[i1+1],z1=pos[i1+2]
  const x2=pos[i2],y2=pos[i2+1],z2=pos[i2+2]
  const ux=x1-x0,uy=y1-y0,uz=z1-z0
  const vx=x2-x0,vy=y2-y0,vz=z2-z0
  const nx=uy*vz-uz*vy,ny=uz*vx-ux*vz,nz=ux*vy-uy*vx
  area2+=Math.hypot(nx,ny,nz)
  const det=x0*nx+y0*ny+z0*nz
  vol6+=det
  cx+=det*(x0+x1+x2)
  cy+=det*(y0+y1+y2)
  cz+=det*(z0+z1+z2)
 }
 const volumeMm3=Math.abs(vol6)/6
 const surfaceAreaMm2=area2/2
 const centerOfMass:[number,number,number]=Math.abs(vol6)>1e-12
  ?[cx/(4*vol6),cy/(4*vol6),cz/(4*vol6)]
  :[(minX+maxX)/2,(minY+maxY)/2,(minZ+maxZ)/2]
 let ixx=0,iyy=0,izz=0,ixy=0,ixz=0,iyz=0
 for(let i=0;i+2<idx.length;i+=3){
  const i0=idx[i]*3,i1=idx[i+1]*3,i2=idx[i+2]*3
  const ax=pos[i0]-centerOfMass[0],ay=pos[i0+1]-centerOfMass[1],az=pos[i0+2]-centerOfMass[2]
  const bx=pos[i1]-centerOfMass[0],by=pos[i1+1]-centerOfMass[1],bz=pos[i1+2]-centerOfMass[2]
  const dx=pos[i2]-centerOfMass[0],dy=pos[i2+1]-centerOfMass[1],dz=pos[i2+2]-centerOfMass[2]
  const det=ax*(by*dz-bz*dy)+ay*(bz*dx-bx*dz)+az*(bx*dy-by*dx)
  const sx2=ax*ax+bx*bx+dx*dx+ax*bx+ax*dx+bx*dx
  const sy2=ay*ay+by*by+dy*dy+ay*by+ay*dy+by*dy
  const sz2=az*az+bz*bz+dz*dz+az*bz+az*dz+bz*dz
  const sxy=2*(ax*ay+bx*by+dx*dy)+ax*by+ax*dy+bx*ay+bx*dy+dx*ay+dx*by
  const sxz=2*(ax*az+bx*bz+dx*dz)+ax*bz+ax*dz+bx*az+bx*dz+dx*az+dx*bz
  const syz=2*(ay*az+by*bz+dy*dz)+ay*bz+ay*dz+by*az+by*dz+dy*az+dy*bz
  ixx+=det*(sy2+sz2)/60
  iyy+=det*(sx2+sz2)/60
  izz+=det*(sx2+sy2)/60
  ixy-=det*sxy/120
  ixz-=det*sxz/120
  iyz-=det*syz/120
 }
 const sign=vol6<0?-1:1
 const tensor:[[number,number,number],[number,number,number],[number,number,number]]=[
  [sign*ixx,sign*ixy,sign*ixz],
  [sign*ixy,sign*iyy,sign*iyz],
  [sign*ixz,sign*iyz,sign*izz],
 ]
 return {
  volumeMm3,
  surfaceAreaMm2,
  surfaceToVolumeRatioPerMm:volumeMm3>1e-12?surfaceAreaMm2/volumeMm3:0,
  centerOfMass,
  bounds:{min:[minX,minY,minZ],max:[maxX,maxY,maxZ]},
  dimensions:[Math.max(0,maxX-minX),Math.max(0,maxY-minY),Math.max(0,maxZ-minZ)],
  principalInertiaMm5:symmetricEigenvalues3x3(tensor),
 }
}

export interface FaceDistanceOptions {
 a:import('./geometry/brep').NurbsBrep;b:import('./geometry/brep').NurbsBrep
 faceA:number;faceB:number;toleranceMm:number;toleranceUv:number;maxCells:number;maxDomainCells:number
}
export interface FaceDistanceResult {
 method:'interval-trimmed-surface-subdivision';scope:'trimmed-surfaces-bounded-joins'
 distanceIntervalMm:[number,number|null]
 parameters:[[number,number],[number,number]]|null
 points:[[number,number,number],[number,number,number]]|null
 pointEnclosures:[Array<[number,number]>,Array<[number,number]>]|null
 converged:boolean;reason:'tolerance'|'work-limit'|'domain-work-limit'|'precision-limit'|'empty-domain'
 cells:number;maxCells:number;domainCells:number;maxDomainCells:number;toleranceMm:number;toleranceUv:number
}
export const measureFaceDistance=(options:FaceDistanceOptions):FaceDistanceResult=>callGeometryRust('cad_face_distance',options)

export type ShellDistanceOptions=Omit<FaceDistanceOptions,'faceA'|'faceB'>
export interface ShellDistanceResult extends Omit<FaceDistanceResult,'method'|'scope'|'reason'> {
 method:'interval-trimmed-face-pairs';scope:'boundary-shells-bounded-joins';containment:'not-classified'
 faces:[number,number]|null;pairs:number;evaluatedPairs:number
 reason:'tolerance'|'work-limit'|'domain-work-limit'|'pair-resolution-limit'|'empty-domain'
}
export const measureShellDistance=(options:ShellDistanceOptions):ShellDistanceResult=>callGeometryRust('cad_shell_distance',options)
