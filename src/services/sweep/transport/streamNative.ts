import {callNurbsRust} from '../../geometry/nurbs'
import type {ProgressiveSweepStreamOptions} from '../../nurbsConstructors'

/** Browser scheduling and cancellation only; Rust owns source snapshots and refinement. */
export async function* streamNativeSweep<P,R>(request:Record<string,unknown>,control:ProgressiveSweepStreamOptions):AsyncGenerator<P,R,void>{
 const checkAbort=()=>{control.signal?.throwIfAborted();if(control.shouldAbort?.())throw new DOMException('Build cancelled','AbortError')}
 checkAbort()
 const {stream}=callNurbsRust<{stream:string}>('brep_sweep_stream_start',request)
 try{for(;;){
  await new Promise<void>(resolve=>setTimeout(resolve,0));checkAbort()
  const next=callNurbsRust<{done:false;value:P}|{done:true;value:R}>('brep_sweep_stream_next',{stream})
  checkAbort();if(next.done)return next.value;yield next.value
 }}finally{callNurbsRust('brep_sweep_stream_release',{stream})}
}

