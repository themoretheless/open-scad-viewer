import {callNurbsRust} from './geometry/nurbs'
import type {NurbsBrep} from './geometry/brep'
import type {NurbsCurve} from './nurbsCurve'
import {decomposeNurbsCurve,inspectNurbsDecompositionBatch} from './nurbsCurve'
import {inspectNativeSweepCapContacts,type NativeSweepCapContactBudgets} from './nurbsSweepCapContacts'

export interface SweepRetainedCorrespondence {
 exact:boolean
 wallErrorUpper:0|null
 inspectedFaces:number
 reason:'exact-retained-coefficients'|'unsupported-section-decomposition'|'retained-wall-mismatch'|'retained-wall-domain-unproved'|'retained-body-face-coverage-unproved'|'work-limit'
}

/** Bind the complete face union to the single retained body's shells.
 * Orphan, omitted or repeated faces cannot establish two-sided coverage.
 * Shell winding and geometric embedding are still independent obligations. */
function retainedBodyFaceCoverage(model:NurbsBrep):boolean {
 try {
  return callNurbsRust<{faceCoverageCertified:boolean}>('sweep_retained_body_coverage_audit',{
   model:{faces:model.faces.map(()=>null),shells:model.shells,bodies:model.bodies},maxFaces:1026,
  }).faceCoverageCertified===true
 }catch{return false}
}

/** The actual face domain must be the complete unit square. Four clamped
 * rational linear sides with positive weights cover the exact rectangle;
 * cyclic start and either traversal orientation preserve its filled region.
 * Exact native coedge identities bind the rectangle to stored world edges.
 * Shell embedding remains a separate obligation. */
function retainedWallUnitDomain(model:NurbsBrep,index:number,budget:{remaining:number}):boolean {
 const face=model.faces[index]
 if(!face||!Number.isInteger(face.outer)||face.outer<0||budget.remaining<=0)return false
 const uses=model.loops[face.outer]?.coedges
 if(!uses||uses.length!==4)return false
 try {
  const coedges=uses.map(use=>{
   if(!Number.isInteger(use.edge)||use.edge<0||!model.edges[use.edge])throw new Error('Invalid retained edge reference')
   return {world:model.edges[use.edge]!.curve,uv:use.pcurve,reversed:use.reversed}
  })
  const audit=callNurbsRust<{domainCertified:boolean;work:number}>('sweep_retained_wall_domain_audit',{
   surface:face.surface,holes:face.holes,coedges,maxWork:budget.remaining,
  })
  if(!Number.isInteger(audit.work)||audit.work<0||audit.work>budget.remaining)return false
  budget.remaining-=audit.work
  return audit.domainCertified===true
 }catch{return false}
}

export interface SweepRetainedDecomposition {
 certified:boolean
 wallErrorUpper:number|null
 inspectedFaces:number
 products:number
 reason:string|null
}
/** Bind every actual ruled face to its original endpoint NURBS knot spans.
 * Equal positive weights along V make each face a linear endpoint blend.
 * The complete maximum then bounds every wall point; no cap/shell proof is
 * supplied. Numeric decomposition error is not relabelled as exact identity. */
export function inspectSweepRetainedDecomposition(model:NurbsBrep,sections:NurbsCurve[][][],closed:boolean,maxProducts=100000,maxFaces=1024,maxExactWork=1000000):SweepRetainedDecomposition {
 let inspectedFaces=0
 const exactBudget={remaining:maxExactWork}
 const refuse=(reason:string,products=0):SweepRetainedDecomposition=>({certified:false,wallErrorUpper:null,inspectedFaces,products,reason})
 if(!Number.isInteger(maxExactWork)||maxExactWork<1||maxExactWork>1000000)return refuse('work-limit')
 if(!Number.isInteger(maxFaces)||maxFaces<1||maxFaces>1024||sections.length<2||sections.length>1025)return refuse('work-limit')
 if(model.faces.length>maxFaces+(closed?0:2))return refuse('work-limit')
 if(!retainedBodyFaceCoverage(model))return refuse('retained-body-face-coverage-unproved')
 const pairs:{curve:NurbsCurve;span:number;retained:NurbsCurve}[]=[]
 const source=sections.map(row=>row.flat())
 if(!source[0]?.length||source.some(row=>row.length!==source[0]!.length)||sections.some(row=>row.length!==sections[0]!.length||row.some((loop,i)=>loop.length!==sections[0]![i]!.length)))return refuse('profile-partition-mismatch')
 for(let layer=0;layer<source.length-1;layer++)for(let profile=0;profile<source[layer]!.length;profile++){
  const a=source[layer]![profile]!,b=source[layer+1]![profile]!
  if(a.periodic!==b.periodic||a.degree!==b.degree||a.controlPoints.length!==b.controlPoints.length||a.knots.length!==b.knots.length||a.knots.some((k,i)=>k!==b.knots[i])||a.weights.length!==b.weights.length||a.weights.some((w,i)=>w!==b.weights[i]))return refuse('source-profile-correspondence-unproved')
  for(let span=a.degree;span<a.controlPoints.length;span++){
   if(!(a.knots[span]!<a.knots[span+1]!))continue
   if(inspectedFaces===maxFaces)return refuse('work-limit')
   if(!retainedWallUnitDomain(model,inspectedFaces,exactBudget))return refuse('retained-wall-domain-unproved')
   const s=model.faces[inspectedFaces++]?.surface,p=a.degree
   if(!s||s.periodicU||s.periodicV||s.degreeU!==p||s.degreeV!==1||s.controlPoints.length!==p+1||s.weights.length!==p+1
    ||s.knotsU.length!==2*(p+1)||s.knotsU.some((v,i)=>v!==(i<=p?0:1))||s.knotsV.length!==4||s.knotsV.some((v,i)=>v!==(i<2?0:1))
    ||s.controlPoints.some(row=>row.length!==2||row.some(point=>point.length!==3||point.some(v=>!Number.isFinite(v))))
    ||s.weights.some(row=>row.length!==2||!Number.isFinite(row[0])||row[0]!<=0||row[0]!==row[1]))return refuse('retained-ruled-face-unproved')
   for(const [endpoint,curve] of [a,b].entries())pairs.push({curve,span,retained:{degree:p,knots:s.knotsU,controlPoints:s.controlPoints.map(row=>row[endpoint]!),weights:s.weights.map(row=>row[endpoint]!),periodic:false}})
  }
 }
 if(!inspectedFaces||inspectedFaces!==model.faces.length-(closed?0:2))return refuse('wall-coverage-unproved')
 try {
  const result=inspectNurbsDecompositionBatch(pairs,maxProducts)
  return {certified:result.errorUpper!==null,wallErrorUpper:result.errorUpper,inspectedFaces,products:result.products,reason:result.reason}
 }catch{return refuse('native-span-unproved')}
}
export interface SweepRetainedCaps {
 exact:boolean
 capErrorUpper:0|null
 exactWork:number
 faces:number[]
 reason:'exact-planar-regions'|'unsupported-section-decomposition'|'cap-contour-mismatch'|'cap-region-unproved'|'work-limit'
}
/** A simple planar region with the same oriented outer/hole contours is the
 * same material set. Native trim and chart certificates are recomputed here. */
export function inspectSweepRetainedCaps(model:NurbsBrep,endpoints:[NurbsCurve[][],NurbsCurve[][]],budgets:NativeSweepCapContactBudgets,maxEdges=1024):SweepRetainedCaps {
 const faces=[model.faces.length-2,model.faces.length-1]
 let exactWork=0
 const refuse=(reason:SweepRetainedCaps['reason']):SweepRetainedCaps=>({exact:false,capErrorUpper:null,exactWork,faces,reason})
 if(!Number.isInteger(maxEdges)||maxEdges<1||maxEdges>1024||!Number.isInteger(budgets.maxExactWork)||budgets.maxExactWork<1||budgets.maxExactWork>1000000)return refuse('work-limit')
 let visited=0,remainingExactWork=budgets.maxExactWork
 for(let endpoint=0;endpoint<2;endpoint++){
  const face=model.faces[faces[endpoint]!]
  if(!face)return refuse('cap-contour-mismatch')
  const wires=[face.outer,...face.holes],rings=endpoints[endpoint]!
  if(wires.length!==rings.length)return refuse('cap-contour-mismatch')
  for(let ring=0;ring<rings.length;ring++){
   const parts=rings[ring]!.map(retainedPieces)
   if(parts.some(p=>p===null))return refuse('unsupported-section-decomposition')
   const expected=parts.flatMap(p=>p!),uses=model.loops[wires[ring]!]!.coedges
   if(expected.length!==uses.length||!uses.length)return refuse('cap-contour-mismatch')
   visited+=uses.length
   if(visited>maxEdges)return refuse('work-limit')
   try {
    const contour=callNurbsRust<{contourIdentity:boolean;work:number}>('sweep_retained_cap_contour_audit',{
     expected:expected.map(part=>({degree:part.degree,knots:Array(part.degree+1).fill(0).concat(Array(part.degree+1).fill(1)),controlPoints:part.points,weights:part.weights,periodic:false})),
     actual:uses.map(use=>model.edges[use.edge]!.curve),reversed:uses.map(use=>use.reversed),maxWork:remainingExactWork,
    })
    if(!Number.isInteger(contour.work)||contour.work<0||contour.work>remainingExactWork)return refuse('work-limit')
    exactWork+=contour.work;remainingExactWork-=contour.work
    if(!contour.contourIdentity)return refuse(remainingExactWork===0?'work-limit':'cap-contour-mismatch')
   }catch{return refuse('cap-contour-mismatch')}
  }
  const audit=inspectNativeSweepCapContacts(model,faces[endpoint]!,faces,{...budgets,maxExactWork:remainingExactWork})
  exactWork+=audit.exactWork
  remainingExactWork-=audit.exactWork
  if(!audit.capCertified||!audit.planarControlHullCertified)return refuse('cap-region-unproved')
 }
 return {exact:true,capErrorUpper:0,exactWork,faces,reason:'exact-planar-regions'}
}
const retainedPieces=(c:NurbsCurve)=>{
 try {
  const pieces=callNurbsRust<NurbsCurve[]|null>('curve_segmented_bezier_controls',{curve:c,maxControlRows:1000000})
  return pieces?.map(part=>({degree:part.degree,points:part.controlPoints,weights:part.weights}))??null
 }catch{return null}
}
/** Bind the interpolation family to actual retained walls. This proves zero
 * extra wall error only for already segmented Bezier sections; caps and the
 * authored-family interpolation bound are separate obligations. */
export function inspectSweepRetainedCorrespondence(model:NurbsBrep,sections:NurbsCurve[][][],closed:boolean,maxFaces=1024,maxExactWork=1000000):SweepRetainedCorrespondence {
 let inspectedFaces=0
 const exactBudget={remaining:maxExactWork}
 const refuse=(reason:SweepRetainedCorrespondence['reason']):SweepRetainedCorrespondence=>({exact:false,wallErrorUpper:null,inspectedFaces,reason})
 if(!Number.isInteger(maxExactWork)||maxExactWork<1||maxExactWork>1000000)return refuse('work-limit')
 if(!Number.isInteger(maxFaces)||maxFaces<1||maxFaces>1024)return refuse('work-limit')

 if(sections.length<2||sections.length>1025)return refuse('unsupported-section-decomposition')
 if(model.faces.length>maxFaces+(closed?0:2))return refuse('work-limit')
 if(!retainedBodyFaceCoverage(model))return refuse('retained-body-face-coverage-unproved')
 const prepared=sections.map(s=>s.flatMap(r=>r.map(retainedPieces)))
 if(prepared.some(s=>!s.length||s.some(p=>p===null)))return refuse('unsupported-section-decomposition')
 try {
  const family=callNurbsRust<{coefficientFamilyIdentity:boolean}>('sweep_retained_wall_family_audit',{
   surfaces:model.faces.map(face=>face.surface),sections,closed,maxFaces,
  })
  if(family.coefficientFamilyIdentity!==true)return refuse('retained-wall-mismatch')
 }catch{return refuse('retained-wall-mismatch')}
 const wallFaces=model.faces.length-(closed?0:2)
 for(let face=0;face<wallFaces;face++){
  if(!retainedWallUnitDomain(model,face,exactBudget))return refuse('retained-wall-domain-unproved')
  inspectedFaces++
 }
 return {exact:true,wallErrorUpper:0,inspectedFaces,reason:'exact-retained-coefficients'}
}

export interface SweepRetainedCapDecomposition {
 certified:boolean
 capErrorUpper:[number,number]|null
 products:number
 regions:SweepRetainedCaps|null
 reason:string|null
}
/** Certify original endpoint spans against the actual decomposed cap contours.
 * Region identity is established only for the decomposed curves, with the
 * original outer/hole partition preserved. Both endpoints share one budget. */
export function inspectSweepRetainedCapDecomposition(model:NurbsBrep,endpoints:[NurbsCurve[][],NurbsCurve[][]],budgets:NativeSweepCapContactBudgets,maxProducts=100000,maxEdges=1024):SweepRetainedCapDecomposition {
 let products=0,regions:SweepRetainedCaps|null=null
 const refuse=(reason:string):SweepRetainedCapDecomposition=>({certified:false,capErrorUpper:null,products,regions,reason})
 if(!Number.isInteger(maxProducts)||maxProducts<0||maxProducts>1000000||!Number.isInteger(maxEdges)||maxEdges<1||maxEdges>1024)return refuse('work-limit')
 const prepared:NurbsCurve[][][]=[],bounds:number[]=[]
 let edges=0
 try{
  for(const endpoint of endpoints){
   const rings:NurbsCurve[][]=[],pairs:{curve:NurbsCurve;span:number;retained:NurbsCurve}[]=[]
   for(const ring of endpoint){
    const curves:NurbsCurve[]=[]
    for(const original of ring){
     const copied=retainedPieces(original)
     const pieces=copied?copied.map(part=>({degree:part.degree,knots:Array(part.degree+1).fill(0).concat(Array(part.degree+1).fill(1)),controlPoints:part.points,weights:part.weights,periodic:false})):decomposeNurbsCurve(original).map(part=>({...part.curve,knots:Array(original.degree+1).fill(0).concat(Array(original.degree+1).fill(1))}))
     const spans:number[]=[]
     for(let span=original.degree;span<original.controlPoints.length;span++)if(original.knots[span]!<original.knots[span+1]!)spans.push(span)
     if(!pieces.length||pieces.length!==spans.length)return refuse('source-span-layout-unproved')
     edges+=pieces.length
     if(edges>maxEdges)return refuse('work-limit')
     pieces.forEach((retained,i)=>pairs.push({curve:original,span:spans[i]!,retained}))
     curves.push(...pieces)
    }
    rings.push(curves)
   }
   const audit=inspectNurbsDecompositionBatch(pairs,maxProducts-products)
   products+=audit.products
   if(audit.errorUpper===null)return refuse(audit.reason??'decomposition-unproved')
   bounds.push(audit.errorUpper);prepared.push(rings)
  }
  regions=inspectSweepRetainedCaps(model,prepared as [NurbsCurve[][],NurbsCurve[][]],budgets,maxEdges)
  if(!regions.exact)return refuse(regions.reason)
  return {certified:true,capErrorUpper:bounds as [number,number],products,regions,reason:null}
 }catch{return refuse('native-span-unproved')}
}
