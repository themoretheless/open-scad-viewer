import type {Ref} from 'svet'
import type {OffsetDeviationOptions} from '../kernel/nurbsOffsetDeviation'
import {createSolidPreviewWorker} from '../kernel/solidPreviewWorker'
import type {MainSolidWorkerClient} from './mainSolidWorkerClient'
import {useBoundNativeInspection} from './useBoundNativeInspection'
export function useOffsetDeviationInspection(options:Ref<OffsetDeviationOptions|undefined>,active:Ref<boolean>,
 worker:Pick<MainSolidWorkerClient,'run'|'cancel'|'dispose'>=createSolidPreviewWorker()) {
 return useBoundNativeInspection(options,active,o=>({requestIdentity:JSON.stringify(o)}),
  request=>worker.run({kind:'offsetDeviation',options:request}),worker)
}
