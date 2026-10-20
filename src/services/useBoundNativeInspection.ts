import {ref,shallowRef,watch,onScopeDispose,toRaw,type Ref} from 'vue'
/** Request identity guards successes and failures, including raw mutations. */
export function useBoundNativeInspection<O,R>(options:Ref<O|undefined>,active:Ref<boolean>,
 expectation:(options:O)=>{requestIdentity:string},dispatch:(options:O)=>Promise<R>,
 worker:{cancel:()=>void;dispose:()=>void}) {
 const result=shallowRef<R|null>(null),pending=ref(false),error=ref(''),errorCode=ref('')
 let generation=0,disposed=false
 function cancel(){generation++;worker.cancel();pending.value=false;result.value=null;error.value='';errorCode.value=''}
 async function run(){
  if(disposed)return
  cancel()
  if(!active.value||!options.value)return
  const ticket=generation
  pending.value=true
  let dispatched=false,identity:string|undefined
  try{
   const request=structuredClone(toRaw(options.value))
   identity=expectation(request).requestIdentity
   dispatched=true
   const answer=await dispatch(request)
   if(ticket!==generation||!active.value||!options.value)return
   if(expectation(toRaw(options.value)).requestIdentity!==identity){cancel();return}
   result.value=answer
  }catch(cause){if(ticket===generation){
   // A raw recipe mutation can bypass Vue's watcher. Late failures must
   // obey the same request identity as successful worker responses.
   if(dispatched){
    try{
     if(!active.value||!options.value||expectation(toRaw(options.value)).requestIdentity!==identity){cancel();return}
    }catch{cancel();return}
   }
   error.value=cause instanceof Error?cause.message:String(cause)
   errorCode.value=typeof (cause as {code?:unknown})?.code==='string'?(cause as {code:string}).code:dispatched?'CAD_OPERATION':'CAD_REQUEST'
  }}
  finally{if(ticket===generation)pending.value=false}
 }
 watch(()=>[options.value,active.value],cancel,{deep:true,flush:'sync'})
 onScopeDispose(()=>{disposed=true;cancel();worker.dispose()})
 return {result,pending,error,errorCode,run,retry:run,cancel}
}
