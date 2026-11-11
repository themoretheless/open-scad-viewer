import {MainSolidWorkerClient} from './mainSolidWorkerClient'
/** Preview cancellation must not interrupt inspection or other modeling clients. */
export function createSolidPreviewWorker() {
 return new MainSolidWorkerClient(()=>new Worker(new URL('../workers/mainSolid.worker.ts',import.meta.url),{type:'module'}))
}
