import {editExactFullRoundSceneMesh} from '../kernel/exactFullRoundHistoryScene'
import {isHistoryRequest,isHistoryMesh} from '../kernel/exactFullRoundHistoryWorkerProtocol'
import {meshTransferables} from '../core/mesh'
let admitted=false
self.addEventListener('message',(event:MessageEvent<unknown>)=>{
  if(admitted)return
  admitted=true
  if(!isHistoryRequest(event.data)){self.close();return}
  const request=event.data
  try{
    const mesh=editExactFullRoundSceneMesh(request.mesh,request.edit,request.segments,request.budget)
    if(!isHistoryMesh(mesh))throw new Error('History result exceeds bounded mesh transport')
    self.postMessage({type:'exact-history-result/1',id:request.id,ok:true,mesh},{transfer:meshTransferables([mesh])})
  }catch(error){
    self.postMessage({type:'exact-history-result/1',id:request.id,ok:false,
      error:(error instanceof Error?error.message:'Native history edit failed').slice(0,1024)})
  }finally{self.close()}
})
