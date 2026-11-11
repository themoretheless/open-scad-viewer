import {MainSolidWorkerClient} from './mainSolidWorkerClient'
/** Independent worker ownership prevents inspection from cancelling modeling jobs. */
export function createSolidClearanceWorker() {
 return new MainSolidWorkerClient(()=>new Worker(new URL('../workers/mainSolid.worker.ts',import.meta.url),{type:'module'}))
}
