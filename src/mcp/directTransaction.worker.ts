import {parentPort,workerData} from 'node:worker_threads'
import {parseDirectDocument,serializeDirectDocument} from '../app/directModeling'
import {executeDirectTransaction} from '../app/directTransactions'
import {warmGeometryKernel} from '../kernel/geometry/kernel'
try{
 await warmGeometryKernel()
 const result=executeDirectTransaction(parseDirectDocument(workerData.document),workerData.script)
 parentPort!.postMessage({ok:true,document:serializeDirectDocument(result)})
}catch(e){parentPort!.postMessage({ok:false,error:e instanceof Error?e.message:String(e)})}
