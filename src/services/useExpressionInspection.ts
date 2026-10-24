import type {Ref} from 'vue'
import {createSolidPreviewWorker} from './solidPreviewWorker'
import {expressionInspectionExpectation,type ExpressionInspectionOptions} from './expressionShellInspection'
import type {MainSolidWorkerClient} from './mainSolidWorkerClient'
import {useBoundNativeInspection} from './useBoundNativeInspection'
/** Independent inspection realm; recipe edits invalidate every pending result. */
export function useExpressionInspection(options:Ref<ExpressionInspectionOptions|undefined>,active:Ref<boolean>,worker:Pick<MainSolidWorkerClient,'run'|'cancel'|'dispose'>=createSolidPreviewWorker()) {
 return useBoundNativeInspection(options,active,expressionInspectionExpectation,
  request=>worker.run({kind:'expressionShellInspect',options:request}),worker)
}
