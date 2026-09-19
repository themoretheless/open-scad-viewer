import {createHash} from 'node:crypto'
import {writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {spawnSync} from 'node:child_process'

/** Keep captured child output on disk and a bounded failure tail in CI's console. */
export function runQualificationStep({id,command,args,cwd,output,options={}}) {
 const result=spawnSync(command,args,{cwd,encoding:'utf8',...options})
 const log=(result.stdout??'')+(result.stderr??'')
 const logPath=resolve(output,`${id}.log`)
 writeFileSync(logPath,log)
 if(result.error||result.status!==0){
  const detail=result.error?.message??(result.signal?`signal ${result.signal}`:`exit ${result.status}`)
  process.stderr.write(`${id} failed (${detail}); full log: ${logPath}\n${log.slice(-16_384)}\n`)
  throw Error(`${id} failed (${detail})`,{cause:result.error})
 }
 return {id,result:'pass',artifactHash:createHash('sha256').update(log).digest('hex')}
}
