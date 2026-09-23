import {createMainSolidWorkerHandler} from '../services/mainSolidWorkerRuntime'
const handle=createMainSolidWorkerHandler((message,transfer)=>transfer?.length?self.postMessage(message,{transfer}):self.postMessage(message))
self.addEventListener('message',event=>{void handle(event.data)})
