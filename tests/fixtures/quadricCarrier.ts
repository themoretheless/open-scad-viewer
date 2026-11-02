import type {OriginalQuadricCarrierQuery} from '../../src/services/originalQuadricCarrierProtocol'
export function quadricQuery():OriginalQuadricCarrierQuery{return{
 curve:{degree:1,knots:[-2,-2,5,5],controlPoints:[[-2,0,0],[2,0,0]],weights:[1,1],periodic:false},
 quadric:[[1,0,0,0],[0,1,0,0],[0,0,1,0],[0,0,0,-1]],constantProducts:[],tolerance:1e-9,maxSpans:10,maxExactWork:100000,
}}
/** Synthetic transport fixture; work counts are not native qualification. */
export function quadricReport():any{return{
 schema:'nurbs-original-quadric-carrier-roots/1',status:'present-on-carrier',presenceProven:true,carrierAbsenceProven:false,curveCarrierCoverageComplete:true,
 parameterCoordinates:['curve-natural-t'],roots:[[-.25,-1],[3.25,1]].map(([t,x])=>({method:'original-quadric-polynomial',parameter:[t,t],positionBounds:[[x,x],[0,0],[0,0]],sourceSpans:[1],multiplicities:[1]})),
 scalarProofs:[{span:1,status:'roots',normalizedOriginalSpanRoots:[{interval:[.25,.25],multiplicity:1},{interval:[.75,.75],multiplicity:1}],reason:null,workUsed:30}],
 unresolvedSpans:[],spans:1,maxSpans:10,exactWork:30,maxExactWork:100000,proofContextSerialized:false,finiteSurfaceUvAdmission:false,allSurfaceBranchesEnumerated:false,trimAdmission:false,bodyAdmission:false,
}}
