import type {Ref} from 'svet'
import {createSolidPreviewWorker} from '../kernel/solidPreviewWorker'
import {profileBridgeChordExpectation,type ProfileBridgeChordOptions} from '../kernel/nurbsProfileBridgeChords'
import type {MainSolidWorkerClient} from './mainSolidWorkerClient'
import {useBoundNativeInspection} from './useBoundNativeInspection'
export function useProfileBridgeInspection(options:Ref<ProfileBridgeChordOptions|undefined>,active:Ref<boolean>,worker:Pick<MainSolidWorkerClient,'run'|'cancel'|'dispose'>=createSolidPreviewWorker()) {
 return useBoundNativeInspection(options,active,profileBridgeChordExpectation,
  request=>worker.run({kind:'profileBridgeChords',options:request}),worker)
}
