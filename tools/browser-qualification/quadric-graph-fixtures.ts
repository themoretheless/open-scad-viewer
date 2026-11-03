import type {OriginalQuadricGraphQuery,OriginalQuadricGraphReport} from '../../src/services/originalQuadricGraphProtocol'
type Case={name:string;query:OriginalQuadricGraphQuery;check:(r:OriginalQuadricGraphReport)=>void}
function require(x:unknown,message:string):asserts x{if(!x)throw new Error(message)}
export function graphQuery():OriginalQuadricGraphQuery{return{curve:{degree:1,knots:[2,2,6,6],controlPoints:[[0,0,-.25],[1,1,.75]],weights:[1,1],periodic:false},patch:{degreeU:1,degreeV:1,knotsU:[-2,-2,2,2],knotsV:[10,10,14,14],controlPoints:[[[0,0,0],[0,1,0]],[[1,0,0],[1,1,1]]],weights:[[1,1],[1,1]],periodicU:false,periodicV:false},quadric:[[0,.5,0,0],[.5,0,0,0],[0,0,0,-.5],[0,0,-.5,0]],constantProducts:[],tolerance:1e-9,maxSpans:10,maxExactWork:1000000}}
function present(r:OriginalQuadricGraphReport,expected:[number,number,number,number][],method?:string){
 require(r.targetGraphProven&&r.curvePatchCoverageComplete&&r.status==='present-on-patch'&&r.finiteSurfaceUvAdmission,'Missing proved finite graph contact');require(r.contacts.length===expected.length,'Wrong finite contact count')
 for(let i=0;i<expected.length;i++){const c=r.contacts[i]!,[t,u,v,m]=expected[i]!;for(const [k,x] of [t,u,v].entries()){const b=c.parameterBox[k]!;require(b[0]<=x&&x<=b[1],'Natural finite coordinates outside box')}require(c.multiplicities[0]===m,'Wrong residual multiplicity');if(method)require(c.uvMethod===method,'Wrong finite UV proof')}
}
export const quadricGraphCases:Case[]=[]
quadricGraphCases.push({name:'graph-tangent',query:graphQuery(),check:r=>present(r,[[4,0,12,2]],'exact-singleton')})
{
 const q=graphQuery();q.curve.controlPoints=[[0,.5,.25],[1,.5,.25]]
 quadricGraphCases.push({name:'graph-transverse',query:q,check:r=>present(r,[[4,0,12,1]])})
}
for(const rational of [false,true]){
 const q=graphQuery();q.curve.controlPoints=[[0,0,-.1875],[1,1,.8125]];if(rational)q.curve.weights=[1,3]
 quadricGraphCases.push({name:`graph-two-roots-rational-${rational}`,query:q,check:r=>present(r,rational?[[2.4,-1,11,1],[4,1,13,1]]:[[3,-1,11,1],[5,1,13,1]])})
}
for(const tangent of [false,true]){
 const q=graphQuery();Object.assign(q.patch,{degreeU:2,degreeV:2,knotsU:[-2,-2,-2,2,2,2],knotsV:[10,10,10,14,14,14],controlPoints:[0,1,2].map(i=>[0,1,2].map(j=>[i/2,j/2,Number(i===2)+Number(j===2)])),weights:[[1,1,1],[1,1,1],[1,1,1]]});q.quadric=[[1,0,0,0],[0,1,0,0],[0,0,0,-.5],[0,0,-.5,0]];q.curve.controlPoints=tangent?[[-.5,.5,.25],[.5,.5,.25]]:[[0,.5,.75],[1,.5,.75]]
 quadricGraphCases.push({name:`graph-paraboloid-tangent-${tangent}`,query:q,check:r=>present(r,tangent?[[4,-2,12,2]]:[[2+4*Math.sqrt(.5),-2+4*Math.sqrt(.5),12,1]])})
}
{
 const q=graphQuery();q.curve.controlPoints=[[2,0,.5],[2,1,.5]]
 quadricGraphCases.push({name:'graph-finite-exclusion-carrier-present',query:q,check:r=>require(r.status==='absent-on-patch'&&r.patchAbsenceProven&&r.carrier?.presenceProven&&!r.finiteSurfaceUvAdmission,'Carrier root promoted despite finite exclusion')})
}
{
 const q=graphQuery();for(const row of q.patch.controlPoints)for(const p of row)[p[0],p[2]]=[p[2]!,p[0]!];q.patch.weights=[[7,7],[7,7]];q.curve.controlPoints=[[.25,.5,0],[.25,.5,1]];q.quadric=[[0,0,0,-.5],[0,0,.5,0],[0,.5,0,0],[-.5,0,0,0]]
 quadricGraphCases.push({name:'graph-rotated-chart-weight-gauge',query:q,check:r=>present(r,[[4,0,12,1]])})
}
for(const zero of [false,true]){
 const q=graphQuery();if(zero)q.maxExactWork=0;else q.patch.controlPoints[1]![1]![2]=1+Number.EPSILON
 quadricGraphCases.push({name:`graph-unproved-target-zero-${zero}`,query:q,check:r=>require(r.status==='unresolved'&&!r.targetGraphProven&&r.carrier===null&&!r.finiteSurfaceUvAdmission,'Unproved target admitted')})
}
{
 const q=graphQuery();q.curve.controlPoints=[[0,0,0],[1,0,0]]
 quadricGraphCases.push({name:'graph-coincident-component-unresolved',query:q,check:r=>require(r.targetGraphProven&&!r.curvePatchCoverageComplete&&r.status==='unresolved','Coincident component declared isolated-complete')})
}
export function boundaryQuery(edge=0):OriginalQuadricGraphQuery{
 const q=graphQuery(),l=[1,0,-1,-2],f=[1,-1,0,4]
 q.curve={degree:3,knots:[2,2,2,2,6,6,6,6],controlPoints:[0,1,2,3].map(i=>edge===0?[l[i]!,.5,.5*l[i]!-f[i]!]:edge===1?[1+l[i]!,.5,.5*(1+l[i]!)-f[i]!]:edge===2?[.5,l[i]!,.5*l[i]!-f[i]!]:edge===3?[.5,1+l[i]!,.5*(1+l[i]!)-f[i]!]:edge===4?[l[i]!,l[i]!,0]:[1+l[i]!,1+l[i]!,[3,1,-1,-3][i]!]),weights:[1,1,1,1],periodic:false};return q
}
for(let edge=0;edge<6;edge++){
 const q=boundaryQuery(edge),uv=edge===0?[-2,12]:edge===1?[2,12]:edge===2?[0,10]:edge===3?[0,14]:edge===4?[-2,10]:[2,14]
 quadricGraphCases.push({name:`graph-edge-${edge}`,query:q,check:r=>{present(r,[[2+4/3,uv[0]!,uv[1]!,2]],'quadric-coordinate-sign');require(r.boundaryExactWork>0,'Missing coordinate work')}})
}
for(const rotated of [false,true])for(const delta of [-(2**-40),2**-40]){
 const q=boundaryQuery(),f=[1,-1,0,4];q.curve.controlPoints=[1,0,-1,-2].map((l,i)=>[l+delta,.5,.5*(l+delta)-f[i]!])
 if(rotated){for(const row of q.patch.controlPoints)for(const p of row)[p[0],p[2]]=[p[2]!,p[0]!];for(const p of q.curve.controlPoints)[p[0],p[2]]=[p[2]!,p[0]!];q.quadric=[[0,0,0,-.5],[0,0,.5,0],[0,.5,0,0],[-.5,0,0,0]]}
 quadricGraphCases.push({name:`graph-near-edge-rotated-${rotated}-delta-${delta}`,query:q,check:r=>{if(delta>0)present(r,[[2+4/3,-2+4*delta,12,2]],'quadric-coordinate-sign');else require(r.status==='absent-on-patch'&&r.curvePatchCoverageComplete,'Exterior sign failed')}})
}
for(const delta of [-(2**-40),0,2**-40]){
 const q=graphQuery(),x=[-3,-3,-2,0,3],f=[3,3,1,-3,3];q.curve={degree:4,knots:[2,2,2,2,2,6,6,6,6,6],controlPoints:x.map((a,i)=>[a+delta,.5,.5*(a+delta)-f[i]!]),weights:[1,1,1,1,1],periodic:false}
 quadricGraphCases.push({name:`graph-irrational-edge-delta-${delta}`,query:q,check:r=>{if(delta>=0)present(r,[[2+4*Math.sqrt(.5),-2+4*delta,12,2]],'quadric-coordinate-sign');else require(r.status==='absent-on-patch'&&r.curvePatchCoverageComplete,'Irrational exterior failed')}})
}
{
 const q=graphQuery(),d=2**-8,x=[.1+d,.1-d,.1+d],f=[d,0,-d];q.patch.controlPoints=[[[.1,0,0],[.1,1,.1]],[[1.1,0,0],[1.1,1,1.1]]];q.curve={degree:2,knots:[2,2,2,6,6,6],controlPoints:x.map((a,i)=>[a,.5,.5*a-f[i]!]),weights:[1,2,4],periodic:false}
 quadricGraphCases.push({name:'graph-original-nonrepresentable-frame',query:q,check:r=>present(r,[[2+4/3,-2,12,1]],'quadric-coordinate-sign')})
}
for(const gauge of [1,7]){
 const q=graphQuery();q.patch.weights=[[gauge,3*gauge],[2*gauge,6*gauge]]
 quadricGraphCases.push({name:`graph-projective-target-gauge-${gauge}`,query:q,check:r=>{present(r,[[4,-2+4/3,11,2]]);require(r.targetParameterization==='separable-projective','Missing projective target proof')}})
}
{
 const q=graphQuery(),a=Math.sqrt(.5);Object.assign(q.patch,{degreeU:2,degreeV:2,knotsU:[-2,-2,-2,2,2,2],knotsV:[10,10,10,14,14,14],controlPoints:[0,1,2].map(i=>[0,1,2].map(j=>[i/2,j/2,Number(i===2)+Number(j===2)])),weights:[[1,3,9],[2,6,18],[4,12,36]]});q.quadric=[[1,0,0,0],[0,1,0,0],[0,0,0,-.5],[0,0,-.5,0]];q.curve.controlPoints=[[0,.5,.75],[1,.5,.75]]
 quadricGraphCases.push({name:'graph-projective-degree-two-target',query:q,check:r=>{present(r,[[2+4*a,-2+4*a/(2-a),11,1]]);require(r.targetParameterization==='separable-projective','Wrong degree-two target clock')}})
}
function generalTarget(q:OriginalQuadricGraphQuery){Object.assign(q.patch,{degreeU:2,degreeV:2,knotsU:[-2,-2,-2,2,2,2],knotsV:[10,10,10,14,14,14],controlPoints:[0,1,2].map(i=>[0,1,2].map(j=>[i/2,j/2,i*j/4])),weights:[[1,3,7],[2,6,14],[5,15,35]]});return q}
{
 const q=generalTarget(graphQuery()),u=(Math.sqrt(5)-1)/4,v=(Math.sqrt(7)-1)/6
 quadricGraphCases.push({name:'graph-general-separable-target-clocks',query:q,check:r=>{present(r,[[4,-2+4*u,10+4*v,2]]);require(r.targetParameterization==='separable-rational','General target clock not proven');for(const b of r.contacts[0]!.parameterBox.slice(1))require(b[0]<b[1]&&b[1]-b[0]<1e-12,'Clock inverse was not refined')}})
}
{
 const q=generalTarget(boundaryQuery()),v=(Math.sqrt(7)-1)/6
 quadricGraphCases.push({name:'graph-general-rational-target-edge',query:q,check:r=>{present(r,[[2+4/3,-2,10+4*v,2]],'quadric-coordinate-sign');require(r.targetParameterization==='separable-rational','General edge clock not proven')}})
}
for(let edge=0;edge<6;edge++){
 const q=boundaryQuery(edge);q.patch.weights=[[1,3],[2,6]];const uv=edge===0?[-2,11]:edge===1?[2,11]:edge===2?[-2+4/3,10]:edge===3?[-2+4/3,14]:edge===4?[-2,10]:[2,14]
 quadricGraphCases.push({name:`graph-projective-target-edge-${edge}`,query:q,check:r=>present(r,[[2+4/3,uv[0]!,uv[1]!,2]],'quadric-coordinate-sign')})
}
{
 const q=generalTarget(graphQuery());q.patch.controlPoints=[0,1,2].map(i=>[0,1,2].map(j=>[i/2,j/2,Number(i===2)+Number(j===2)]));q.patch.weights=[[1,3,9],[2,6,18],[5,15,45]];q.quadric=[[1,0,0,0],[0,1,0,0],[0,0,0,-.5],[0,0,-.5,0]]
 quadricGraphCases.push({name:'graph-separable-clock-wrong-quadric-refuses',query:q,check:r=>require(!r.targetGraphProven&&!r.finiteSurfaceUvAdmission&&r.status==='unresolved','Unproved weighted geometry admitted')})
}

function multispanTarget(q:OriginalQuadricGraphQuery){
 const x=[0,.25,.75,1];Object.assign(q.patch,{knotsU:[-2,-2,-1,1,2,2],knotsV:[10,10,11,13,14,14],controlPoints:x.map(a=>x.map(b=>[a,b,a*b])),weights:x.map(()=>x.map(()=>7))});return q
}
{
 const q=multispanTarget(graphQuery());quadricGraphCases.push({name:'graph-multispan-internal-knot',query:q,check:r=>{present(r,[[4,0,12,2]]);require(r.targetParameterization==='polynomial-affine','Missing original linear precision proof')}})
}
for(let edge=0;edge<6;edge++){
 const q=multispanTarget(boundaryQuery(edge)),uv=edge===0?[-2,12]:edge===1?[2,12]:edge===2?[0,10]:edge===3?[0,14]:edge===4?[-2,10]:[2,14]
 quadricGraphCases.push({name:`graph-multispan-outer-edge-${edge}`,query:q,check:r=>{present(r,[[2+4/3,uv[0]!,uv[1]!,2]],'quadric-coordinate-sign');require(r.boundaryExactWork>0,'Missing outer-frame proof')}})
}
{
 const q=graphQuery(),x=[0,.25,.75,1],z=[0,0,.5,1];Object.assign(q.patch,{degreeU:2,knotsU:[-2,-2,-2,0,2,2,2],controlPoints:x.map((a,i)=>[[a,0,z[i]!],[a,1,z[i]!]]),weights:x.map(()=>[1,1])});q.quadric=[[1,0,0,0],[0,0,0,0],[0,0,0,-.5],[0,0,-.5,0]];q.curve.controlPoints=[[.5,.5,-1],[.5,.5,1]]
 quadricGraphCases.push({name:'graph-multispan-quadratic-original-basis',query:q,check:r=>present(r,[[4.5,0,12,1]])})
}
for(const zero of [false,true]){
 const q=multispanTarget(graphQuery());if(zero)q.maxExactWork=0;else q.patch.controlPoints[1]![1]![2]!+=.125
 quadricGraphCases.push({name:`graph-multispan-unproved-target-zero-${zero}`,query:q,check:r=>require(!r.targetGraphProven&&r.carrier===null&&!r.finiteSurfaceUvAdmission&&r.status==='unresolved','Unproved multi-span target admitted')})
}

function rationalMultispanTarget(q:OriginalQuadricGraphQuery){multispanTarget(q);q.patch.weights=[1,3,5,7].map(a=>[1,2,6,8].map(b=>a*b));return q}
{
 const q=rationalMultispanTarget(graphQuery());quadricGraphCases.push({name:'graph-rational-multispan-target-clocks',query:q,check:r=>{present(r,[[4,-.25,11.5,2]]);require(r.targetParameterization==='separable-rational','Missing original multi-span clock proof')}})
}
for(let edge=0;edge<6;edge++){
 const q=rationalMultispanTarget(boundaryQuery(edge)),uv=edge===0?[-2,11.5]:edge===1?[2,11.5]:edge===2?[-.25,10]:edge===3?[-.25,14]:edge===4?[-2,10]:[2,14]
 quadricGraphCases.push({name:`graph-rational-multispan-edge-${edge}`,query:q,check:r=>present(r,[[2+4/3,uv[0]!,uv[1]!,2]],'quadric-coordinate-sign')})
}

function coupledPlane(q:OriginalQuadricGraphQuery){for(const row of q.patch.controlPoints)for(const p of row)p[2]=0;q.patch.weights=[[1,3],[2,2]];q.quadric=[[0,0,0,0],[0,0,0,0],[0,0,0,.5],[0,0,.5,0]];return q}
{
 const q=coupledPlane(graphQuery());q.curve.controlPoints=[[.5,.625,-1],[.5,.625,1]]
 quadricGraphCases.push({name:'graph-coupled-planar-bilinear-target',query:q,check:r=>{present(r,[[4,0,12,1]]);require(r.targetParameterization==='coupled-bilinear','Missing coupled joint inverse proof')}})
}
{
 const q=coupledPlane(boundaryQuery()),f=[1,-1,0,4];q.curve.controlPoints=q.curve.controlPoints.map((p,i)=>[p[0]!,.75,-f[i]!])
 quadricGraphCases.push({name:'graph-coupled-planar-algebraic-edge',query:q,check:r=>present(r,[[2+4/3,-2,12,2]],'quadric-coordinate-sign')})
}

function coupledDegreeTwo(q:OriginalQuadricGraphQuery){coupledPlane(q);Object.assign(q.patch,{degreeU:2,degreeV:2,knotsU:[-2,-2,-2,2,2,2],knotsV:[10,10,10,14,14,14],controlPoints:[0,.5,1].map(x=>[0,.5,1].map(y=>[x,y,0])),weights:[[1,1,1],[1,1,1],[1,1,17]]});return q}
{
 const q=coupledDegreeTwo(graphQuery());q.curve.controlPoints=[[.75,.75,-1],[.75,.75,1]]
 quadricGraphCases.push({name:'graph-coupled-degree-two-planar-target',query:q,check:r=>{present(r,[[4,0,12,1]]);require(r.targetParameterization==='coupled-bernstein','Missing coupled Bernstein mean proof');for(const b of r.contacts[0]!.parameterBox.slice(1))require(b[1]-b[0]<2,'Joint inverse did not refine')}})
}
{
 const q=coupledDegreeTwo(boundaryQuery()),f=[1,-1,0,4];q.curve.controlPoints=q.curve.controlPoints.map((p,i)=>[p[0]!,.5,-f[i]!])
 quadricGraphCases.push({name:'graph-coupled-degree-two-algebraic-edge',query:q,check:r=>present(r,[[2+4/3,-2,12,2]],'quadric-coordinate-sign')})
}

{
 const q=coupledPlane(graphQuery());Object.assign(q.patch,{degreeU:8,degreeV:8,knotsU:[...Array(9).fill(-2),...Array(9).fill(2)],knotsV:[...Array(9).fill(10),...Array(9).fill(14)],controlPoints:Array.from({length:9},(_,i)=>Array.from({length:9},(_,j)=>[i,j,0])),weights:Array.from({length:9},(_,i)=>Array.from({length:9},(_,j)=>i===8&&j===8?65537:1))});q.curve.controlPoints=[[6,6,-1],[6,6,1]]
 quadricGraphCases.push({name:'graph-coupled-degree-eight-original-target',query:q,check:r=>present(r,[[4,0,12,1]])})
}
