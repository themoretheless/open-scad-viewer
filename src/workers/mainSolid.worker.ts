import {createMainSolidWorkerHandler} from '../services/mainSolidWorkerRuntime'
const handle=createMainSolidWorkerHandler(message=>self.postMessage(message))
self.addEventListener('message',event=>{void handle(event.data)})
