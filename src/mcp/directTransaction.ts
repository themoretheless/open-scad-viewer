import {Worker} from 'node:worker_threads'
/** Each request owns a disposable execution realm; cancellation also stops native WASM. */
export async function runDirectTransaction(document:string,script:string,signal?:AbortSignal):Promise<string>{
 if(Buffer.byteLength(document)>64*1024*1024||Buffer.byteLength(script)>16*1024*1024)throw Error('Transaction input exceeds limits.')
 if(signal?.aborted)throw new DOMException('Cancelled','AbortError')
 const worker=new Worker(new URL('./directTransaction.worker.mjs',import.meta.url),{workerData:{document,script},env:{},stdout:true,stderr:true,resourceLimits:{maxOldGenerationSizeMb:512,maxYoungGenerationSizeMb:32}})
 worker.stdout?.resume();worker.stderr?.resume()
 let timeout:ReturnType<typeof setTimeout>|undefined,abort:(()=>void)|undefined
 try{return await new Promise<string>((resolve,reject)=>{
  abort=()=>reject(new DOMException('Cancelled','AbortError'))
  signal?.addEventListener('abort',abort,{once:true})
  timeout=setTimeout(()=>reject(Error('Transaction exceeded 60 seconds.')),60000)
  worker.once('error',reject)
  worker.once('exit',()=>reject(Error('Transaction worker exited before returning a result.')))
  worker.once('message',(value:unknown)=>{
   const v=value as {ok?:boolean;document?:string;error?:string}
   if(v?.ok===true&&typeof v.document==='string'&&Buffer.byteLength(v.document)<=64*1024*1024)resolve(v.document)
   else reject(Error(v?.error??'Invalid transaction result.'))
  })
  if(signal?.aborted)abort()
 })}finally{clearTimeout(timeout);if(abort)signal?.removeEventListener('abort',abort);await worker.terminate()}
}
