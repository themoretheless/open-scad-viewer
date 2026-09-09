import {inspectCadPairs} from '../services/cadInspection'
import {cadOperation} from '../services/cadWorkbench'
import {mainOperation} from '../services/mainModeling'
self.onmessage = event => {
 try {if(event.data.kind==='inspect'){self.postMessage({report:inspectCadPairs(event.data.bodies)});return}if(event.data.kind==='cad'){self.postMessage({document:cadOperation(event.data.document,event.data.options)});return}const {meshes,selected,hit,operation,parameters}=event.data;self.postMessage({document:mainOperation(meshes,selected,hit,operation,parameters)})}
 catch(e){self.postMessage({error:e instanceof Error?e.message:String(e)})}
}
