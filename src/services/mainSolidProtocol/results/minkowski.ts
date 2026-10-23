import type {SolidMinkowskiResult} from '../../solidMinkowski'
import {finite,type MainSolidExpectation} from '../../mainSolidProtocol'

export function validMinkowskiResult(job:Extract<MainSolidExpectation,{kind:'minkowski3d'}>,value:unknown,validateDocument:(value:unknown)=>boolean):boolean {
    const v=value as SolidMinkowskiResult
    if(!v||!validateDocument(v.document)||(v.resultId!==null&&v.resultId!==job.createdId)
      ||v.document.bodies.length!==job.bodyIds.length+(v.resultId===null?0:1)||!job.bodyIds.every((id,i)=>v.document.bodies[i]?.id===id)
      ||v.resultId!==null&&!v.document.bodies.some(b=>b.id===v.resultId)||!finite(v.volumeMm3)||v.volumeMm3<0
      ||!finite(v.maximumCoordinateRoundingMm)||v.maximumCoordinateRoundingMm<0||v.roundingTopologyCertified!==true
      ||v.maximumSetDeviationMm!==null&&(!finite(v.maximumSetDeviationMm)||v.maximumSetDeviationMm<0))return false
    const validMesh=(m:unknown,i:number)=>{const p=m as {positions:number[];indices:number[]};return !!p&&Array.isArray(p.positions)&&p.positions.length===job.sourceSizes[i]?.[0]&&p.positions.every(finite)&&Array.isArray(p.indices)&&p.indices.length===job.sourceSizes[i]?.[1]&&p.indices.every(n=>Number.isSafeInteger(n)&&n>=0&&n<p.positions.length/3)}
    const p=v.proof
    if(!p||p.meshPrecision!=='f32'||![1,2,3,4].includes(p.certificate?.version)||!Array.isArray(p.certificate.sources)||p.certificate.sources.length!==2||!p.certificate.sources.every(h=>typeof h==='string'&&/^[0-9a-f]{64}$/.test(h)))return false
    if(p.version===1)return p.certificate.version===1&&v.approximationTopology===null&&job.sourceKinds.every(k=>k==='mesh')&&v.sourceHadBrep===false&&v.curvedSourceErrorCertified===false&&validMesh(p.a,0)&&validMesh(p.b,1)&&p.certificate.scope==='represented-positive-filled-polyhedral-minkowski-sum'&&Array.isArray(p.certificate.vertices)&&Array.isArray(p.certificate.triangles)
    if(p.version!==2)return false
    const c=p.certificate,topology=v.approximationTopology,reports=c.sourceTopology
    if(![1,2,3,4].includes(c.version))return false
    const ratio=(q:unknown)=>Array.isArray(q)&&q.length===2&&q.every(x=>typeof x==='string'&&x.length<=2500)&&/^-?(0|[1-9]\d*)$/.test(q[0])&&/^[1-9]\d*$/.test(q[1])
    const count=(n:unknown,max=65536)=>Number.isSafeInteger(n)&&Number(n)>=0&&Number(n)<=max
    const axis=(n:unknown)=>count(n,2)
    const pair=(p:unknown)=>Array.isArray(p)&&p.length===2&&p.every(ratio)
    const radial=(r:NonNullable<typeof reports>[number])=>r===null||r.kind==='radial-homeomorphism'&&r.radialDegree===1&&r.filledSet==='closed-three-ball'&&Array.isArray(r.center)&&r.center.length===3&&r.center.every(ratio)&&count(r.chartCells)&&count(r.nominalTriangles)&&r.nominalTriangles>0
    if(c.version>=2&&(!Array.isArray(reports)||reports.length!==2||!reports.every(radial)))return false
    const atlas=c.sourceAtlasTopology,sum=c.sumTopology
    if(c.version<3&&(atlas!==undefined||sum!==undefined))return false
    if(c.version<3&&v.sourceApproximationTopology!==undefined){
      if(!Array.isArray(c.sourceHomotopyBounds)||c.sourceHomotopyBounds.length!==2||!c.sourceHomotopyBounds.every(ratio)||JSON.stringify(v.sourceApproximationTopology)!==JSON.stringify(c.sourceHomotopyBounds.map((r,i)=>r[0]==='0'||!!reports?.[i])))return false
    }
    if(c.version>=3){
      if(!Array.isArray(c.sourceHomotopyBounds)||c.sourceHomotopyBounds.length!==2||!c.sourceHomotopyBounds.every(ratio)||!Array.isArray(atlas)||atlas.length!==2)return false
      for(const a of atlas){
        if(a===null)continue
        if(!a||a.preservesComplement!==true)return false
        if(a.kind!=='rational-atlas-ambient-isotopy'&&!axis(a.axis))return false
        if(a.kind==='signed-prismatic-ambient-isotopy'){
          if(!Array.isArray(a.levels)||!a.levels.length||!a.levels.every(ratio)||!Array.isArray(a.curveParents)||!a.curveParents.length||a.curveParents.length>32||a.curveParents.some((p,i)=>p!==null&&(!count(p,a.curveParents.length-1)||p===i))||!count(a.nominalProfileVertices)||!Array.isArray(a.shells)||!a.shells.length||a.shells.some(s=>!count(s.shell)||!pair(s.interval)||!Array.isArray(s.curves)||!s.curves.length||s.curves.some(i=>!count(i,a.curveParents.length-1))||!Array.isArray(s.coefficients)||s.coefficients.length!==s.curves.length||s.coefficients.some(i=>i!==1&&i!==-1)))return false
        }else if(a.kind==='polynomial-shear-ambient-isotopy'){
          if(!Array.isArray(a.polynomial)||!a.polynomial.length||a.polynomial.length>33||a.polynomial.some(r=>!Array.isArray(r)||!r.length||r.length>33||r.length!==a.polynomial[0].length||!r.every(ratio))||!count(a.sourceChartCells,512)||!a.sourceChartCells||!count(a.referenceTriangles)||!a.referenceTriangles||![1,2,4,8].includes(a.homotopySegments)||!count(a.checkedPairs,32e6))return false
        }else if(a.kind==='rational-atlas-ambient-isotopy'){
          if(c.version<4||!count(a.chartTriangles,1024)||!a.chartTriangles||!count(a.vertexStars,1024)||!a.vertexStars||!count(a.disjointPairs,524288)||!count(a.boundaryPairs,4e6))return false
        }else return false
      }
      if(sum!==null&&(!sum||sum.kind!=='convex-hole-prismatic-minkowski-homeomorphism'||!axis(sum.axis)||!pair(sum.interval)||!count(sum.holedOperand,1)||!Array.isArray(sum.holes)||sum.holes.length>32||sum.holes.some((h,i)=>!count(h.hole,32)||h.hole!==i+1||typeof h.open!=='boolean'||!count(h.depth,6)||![h.innerCore,h.outerCore].every(p=>Array.isArray(p)&&p.length<=4096&&p.every(pair)))||sum.survivingHoles!==sum.holes.filter(h=>h.open).length))return false
      const sourceProof=c.sourceHomotopyBounds.map((r,i)=>r[0]==='0'||!!reports?.[i]||!!atlas[i])
      if(!Array.isArray(v.sourceApproximationTopology)||JSON.stringify(v.sourceApproximationTopology)!==JSON.stringify(sourceProof))return false
    }
    const exact=c.version>=3&&c.sourceHomotopyBounds.every(r=>r[0]==='0')
    const expectedKind=exact?'exact-source-minkowski-identity':sum?'prismatic-minkowski-homeomorphism':reports?.every(r=>r!==null)?'radial-minkowski-homeomorphism':null
    if(topology===null){if(expectedKind!==null)return false}
    else{
      if(!topology||topology.status!=='proved'||topology.includesMeshRounding!==true||topology.kind!==expectedKind)return false
      if(topology.kind==='radial-minkowski-homeomorphism'&&(topology.filledSet!=='closed-three-ball'||topology.boundary!=='two-sphere'||JSON.stringify(topology.sourceCenters)!==JSON.stringify(reports?.map(r=>r?.center))))return false
      if(topology.kind==='prismatic-minkowski-homeomorphism'&&(!sum||topology.axis!==sum.axis||topology.survivingHoles!==sum.survivingHoles))return false
    }
    return p.version===2&&v.sourceHadBrep===true&&v.curvedSourceErrorCertified===true&&p.certificate.scope==='represented-nurbs-filled-minkowski-enclosures'
      &&[p.a,p.b].every((s,i)=>s&&s.kind===job.sourceKinds[i]&&(s.kind==='mesh'?validMesh(s.mesh,i):!!s.model&&Array.isArray(s.model.faces)&&s.model.faces.length>0))
      &&[p.certificate.nominal,p.certificate.inner,p.certificate.outer].every(c=>!!c&&c.version===1&&Array.isArray(c.vertices)&&Array.isArray(c.triangles))
  
}
