import type {SurfaceIsoTargetQuery,SurfaceIsoTargetReport} from '../../src/services/surfaceIsoTargetProtocol'
function require(x:unknown,m:string):asserts x{if(!x)throw Error(m)}
function query(axis:0|1,sameWorld=false,rational=false,tangent=false):SurfaceIsoTargetQuery{
 const a=7/16,b=tangent?a:13/16,ab=a*b,sum=a+b,z=[ab,ab-sum/2,ab-sum+1]
 const q:SurfaceIsoTargetQuery={axis,parameter:.5,maxCells:10000,window:[[2,6],[-3,5],[10,14]],source:{degreeU:2,degreeV:1,knotsU:[2,2,2,6,6,6],knotsV:[-2,-2,2,2],controlPoints:[0,.5,1].map((x,i)=>[[sameWorld?.5:x,0,z[i]!],[sameWorld?.5:x,1,z[i]!]]),weights:[[1,1],[1,1],[1,1]],periodicU:false,periodicV:false},target:{degreeU:1,degreeV:1,knotsU:[-3,-3,5,5],knotsV:[10,10,14,14],controlPoints:[[[0,0,0],[0,1,0]],[[1,0,0],[1,1,0]]],weights:rational?[[1,3],[2,6]]:[[1,1],[1,1]],periodicU:false,periodicV:false}}
 if(axis===1){const s=q.source;s.controlPoints=[0,1].map(j=>[0,1,2].map(i=>s.controlPoints[i]![j]!));s.weights=[0,1].map(j=>[0,1,2].map(i=>s.weights[i]![j]!));[s.degreeU,s.degreeV]=[s.degreeV,s.degreeU];[s.knotsU,s.knotsV]=[s.knotsV,s.knotsU]}
 return q
}
type Case={name:string;query:SurfaceIsoTargetQuery;check:(r:SurfaceIsoTargetReport)=>void}
export const rootExcisionCases:Case[]=[]
function noCredit(r:SurfaceIsoTargetReport){require(!r.allSurfaceBranchesEnumerated&&!r.trimAdmission&&!r.bodyAdmission,'Window proof promoted into general topology')}
for(const axis of [0,1]as const)for(const sameWorld of [false,true])for(const rational of [false,true]){
 const q=query(axis,sameWorld,rational)
 rootExcisionCases.push({name:`root-excision-axis-${axis}-same-world-${sameWorld}-rational-${rational}`,query:q,check:r=>{
  noCredit(r);require(r.windowCoverageComplete&&r.unresolved.length===0&&r.roots.length===2,'Artificial split root coverage incomplete')
  for(const t of [7/16,13/16]){
   const natural=2+4*t,roots=r.roots.filter(x=>x.parameterBox[0][0]<=natural&&natural<=x.parameterBox[0][1]);require(roots.length===1,'Distinct original parameter roots merged or duplicated')
   const root=roots[0]!,x=sameWorld?.5:t,u=rational?x/(2-x):x,v=rational?.25:.5
   for(const[k,a]of[natural,-3+8*u,10+4*v].entries()){const b=root.parameterBox[k]!;require(b[0]<=a&&a<=b[1],'Natural root outside proved enclosure');require(b[1]-b[0]<1e-8,'Root precision lost')}
   for(const[k,a]of[x,.5,0].entries()){const b=root.positionBounds[k]!;require(b[0]<=a&&a<=b[1],'Original world root outside enclosure')}
   require(root.crossingSign===(t<.5?-1:1),'Wrong transverse orientation')
  }
 }})
}
for(const axis of [0,1]as const){
 const boundary=query(axis);boundary.window[0][0]=2+4*7/16
 rootExcisionCases.push({name:`root-excision-caller-boundary-axis-${axis}`,query:boundary,check:r=>{noCredit(r);require(!r.windowCoverageComplete&&r.unresolved.length>0,'Caller boundary silently removed')}})
 const tangent=query(axis,false,false,true)
 rootExcisionCases.push({name:`root-excision-tangent-axis-${axis}`,query:tangent,check:r=>{noCredit(r);require(!r.windowCoverageComplete&&r.unresolved.length>0&&r.roots.length===0,'Singular contact falsely certified')}})
 const short=query(axis);short.maxCells=1
 rootExcisionCases.push({name:`root-excision-budget-axis-${axis}`,query:short,check:r=>{noCredit(r);require(r.cells===1&&!r.windowCoverageComplete&&r.unresolved.length>0,'Pending coverage dropped on budget exhaustion')}})
}
