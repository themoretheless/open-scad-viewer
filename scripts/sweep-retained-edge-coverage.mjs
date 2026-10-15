import assert from 'node:assert/strict'

// Receipt and topology coverage only. Every smoothness decision is native.
export function checkRetainedEdgeCoverage(item){
 const p=item.nativeProfileSmoothness,station=p.station
 const caps=new Set(item.physicalCapFaces??item.capFaces)
 const uses=new Map()
 for(const [face,loops] of item.faceLoops.entries())for(const loop of [loops.outer,...loops.holes])for(const edge of loop){
  assert.ok(Number.isSafeInteger(edge)&&edge>=0&&edge<item.edges)
  const faces=uses.get(edge)??[];faces.push(face);uses.set(edge,faces)
 }
 assert.equal(uses.size,item.edges,'Missing actual retained edge')
 const internal=[...uses].filter(([,faces])=>faces.length===2&&faces.every(face=>caps.has(face))).map(([edge])=>edge)
 const capAudits=[]
 if(internal.length){
  assert.equal(item.capRepresentation,'radial-annular','Unexpected unaccounted cap interior')
  const groups=item.capFaceGroups
  assert.ok(Array.isArray(groups)&&groups.length===2)
  assert.deepEqual(groups.flat(),[...(item.physicalCapFaces??item.capFaces)])
  assert.equal(new Set(groups.flat()).size,groups.flat().length)
  let construction=item.construction
  if(item.boundaryError)construction=construction[item.boundaryError.nodeId]
  const owner=construction.bodyBoundaryEvidence??construction
  const smooth=owner.retainedSmoothness
  assert.ok(smooth,'Missing native internal-cap smoothness report')
  assert.equal(smooth.capInternalSeams.length,groups.length)
  assert.ok(Number.isSafeInteger(smooth.maxWork)&&smooth.maxWork>=0&&smooth.maxWork<=4000000)
  const capsFirst=smooth.auditOrder==='caps-first'
  assert.ok(smooth.auditOrder===undefined||capsFirst,'Unknown native audit order')
  assert.ok(Number.isSafeInteger(smooth.walls.totalExactWork)&&smooth.walls.totalExactWork>=0&&smooth.walls.totalExactWork<=smooth.walls.maxWork,'Invalid native wall work')
  if(!capsFirst)assert.equal(smooth.walls.maxWork,smooth.maxWork,'Broken legacy native wall-work ceiling')
  let work=capsFirst?0:smooth.walls.totalExactWork
  for(const [index,group] of groups.entries()){
   const expected=internal.filter(edge=>uses.get(edge).every(face=>group.includes(face))).sort((a,b)=>a-b)
   const report=smooth.capInternalSeams[index]
   assert.ok(expected.length>0)
   assert.equal(report.method,'retained-annular-cap-internal-jets')
   assert.equal(report.extractionComplete,true);assert.deepEqual(report.unclassifiedFaces,[]);assert.deepEqual(report.unpairedEdges,[])
   assert.deepEqual([...report.edgeIds].sort((a,b)=>a-b),expected,'Uncovered or rebound native internal cap seam')
   assert.equal(report.maxWork,smooth.maxWork-work,'Broken shared native cap-work ledger')
   assert.ok(Number.isSafeInteger(report.exactWork)&&report.exactWork>=0&&report.exactWork<=report.maxWork)
   assert.equal(typeof report.g1Certified,'boolean');assert.equal(typeof report.g2Certified,'boolean')
   if(report.g1Certified||report.g2Certified)assert.ok(report.exactWork>0,'Positive cap jets require actual native work')
   if(report.g2Certified)assert.equal(report.g1Certified,true)
   work+=report.exactWork
   capAudits.push({group:index,edgeIds:expected,g1Certified:report.g1Certified,g2Certified:report.g2Certified,
    continuity:report.g2Certified?'G2':report.g1Certified?'G1':'unproved'})
  }
  if(capsFirst){assert.equal(smooth.walls.maxWork,smooth.maxWork-work);work+=smooth.walls.totalExactWork}
  assert.equal(work,smooth.totalExactWork);assert.ok(work<=smooth.maxWork)
  assert.equal(smooth.fullBoundarySmoothnessCertified,false)
 }
 const owned=[...p.edgeIds,...station.edgeIds,...station.capEdges,...internal]
 assert.equal(new Set(owned).size,owned.length,'Seam ownership overlaps')
 assert.deepEqual([...owned].sort((a,b)=>a-b),Array.from({length:item.edges},(_,edge)=>edge),'Uncovered retained boundary edges')
 return {internalCapEdges:internal.length,capInternalAudits:capAudits}
}
