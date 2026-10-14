import {readFileSync,writeFileSync} from 'node:fs'
import {inspectSweepProjectiveSeams} from '../src/services/nurbsSweepAudit'
const candidates=JSON.parse(readFileSync(process.argv[2]??'/tmp/sweep-circle-repair-candidates.json','utf8'))
const reports=candidates.map((candidate:any)=>{
 const seams=Array.from({length:candidate.patches.length},(_,i)=>({patches:[i,Math.floor(i/4)*4+(i+1)%4] as [number,number],boundaries:['uMax','uMin'] as ['uMax','uMin'],order:2 as const,normalScale:1,jetTolerance:0}))
 return {file:candidate.file,displacementUpper:candidate.displacementUpper,audit:inspectSweepProjectiveSeams(candidate.patches,seams,2000000)}
})
writeFileSync(process.argv[3]??'/tmp/sweep-circle-repair-audit.json',JSON.stringify(reports,null,2))
for(const r of reports)console.log(r.file,r.audit.certifiedOrder,r.audit.exactWork)
