/** Presentation guards for native interval receipts; no geometric decisions. */
export interface AnnularG2Evidence {
 capGeometricG2Certified?:boolean;capGeometricSeamCount?:number
 capNormalChordUpper?:number;capCurvatureTensorUpper?:number
}
export function readAnnularG2Evidence(smooth:any):AnnularG2Evidence {
 const caps=smooth?.capInternalSeams
 if(!Array.isArray(caps)||caps.length!==2||(!caps[0]?.geometricG2&&!caps[1]?.geometricG2))return {}
 const count=(x:any)=>Number.isSafeInteger(x)&&x>=0&&x<=100000
 const finite=(x:any)=>typeof x==='number'&&Number.isFinite(x)&&x>=0
 const validFace=(face:any)=>Number.isSafeInteger(face)&&face>=0&&face<1024
 let complete=true,cells=0,seams=0,normal=0,curvature=0
 for(const cap of caps){
  const g=cap?.geometricG2
  let edgesMatch=Array.isArray(g?.edgeIds)&&Array.isArray(cap?.edgeIds)&&g.edgeIds.length===cap.edgeIds.length
  if(edgesMatch){
   for(let i=0;i<g.edgeIds.length;i++){
    const edge=g.edgeIds[i]
    if(!Number.isSafeInteger(edge)||edge<0||edge!==cap.edgeIds[i]){edgesMatch=false;break}
   }
  }
  const declared=cap?.extractionComplete===true&&Array.isArray(cap.edgeIds)&&cap.edgeIds.length>0&&new Set(cap.edgeIds).size===cap.edgeIds.length&&
   g?.method==='owned-boundary-geometric-G2-intervals'&&g.exactIdentityClaim===false&&g.solidGeometryCertified===false&&
   count(g.cells)&&count(g.maxCells)&&g.cells<=g.maxCells&&finite(g.positionTolerance)&&finite(g.normalTolerance)&&g.normalTolerance<=2&&finite(g.curvatureTolerance)&&
   edgesMatch&&Array.isArray(g.seams)&&g.seams.length===cap.edgeIds.length
  if(!declared){complete=false;continue}
  cells+=g.cells;seams+=g.seams.length
  complete&&=g.complete===true&&g.withinTolerance===true
  let seamCells=0
  for(let i=0;i<g.seams.length;i++){
   const seam=g.seams[i]
   const r=seam?.inspection,branch=r?.branches?.[r.orientation==='same'?0:r.orientation==='opposite'?1:-1]
   const accepted=seam.edge===cap.edgeIds[i]&&Array.isArray(seam.faces)&&seam.faces.length===2&&seam.faces[0]!==seam.faces[1]&&
    validFace(seam.faces[0])&&validFace(seam.faces[1])&&count(r?.cells)&&r.cells<=g.maxCells&&r?.decision==='within-tolerance'&&r.regularityProven===true&&
    Array.isArray(r.positionBounds)&&r.positionBounds.length===2&&finite(r.positionBounds[0])&&finite(r.positionBounds[1])&&r.positionBounds[0]<=r.positionBounds[1]&&r.positionBounds[1]<=g.positionTolerance&&
    finite(branch?.normalLower)&&finite(branch.normalUpper)&&branch.normalLower<=branch.normalUpper&&branch.normalUpper<=g.normalTolerance&&
    finite(branch.curvatureLower)&&finite(branch.curvatureUpper)&&branch.curvatureLower<=branch.curvatureUpper&&branch.curvatureUpper<=g.curvatureTolerance
   seamCells+=count(r?.cells)?r.cells:0
   complete&&=accepted
   if(accepted){normal=Math.max(normal,branch.normalUpper);curvature=Math.max(curvature,branch.curvatureUpper)}
  }
  complete&&=seamCells===g.cells
 }
 complete&&=count(smooth.geometricCells)&&count(smooth.geometricMaxCells)&&cells===smooth.geometricCells&&cells<=smooth.geometricMaxCells
 return {capGeometricG2Certified:complete,capGeometricSeamCount:seams,...(complete?{capNormalChordUpper:normal,capCurvatureTensorUpper:curvature}:{})}
}
