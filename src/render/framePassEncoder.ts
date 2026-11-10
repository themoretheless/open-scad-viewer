// Public entry point; implementations retain their original contracts.
export { groupByShadingModel, setObjectStyleImmediate, advanceSceneGeometryAnimation } from './framePassEncoder/groupByShadingModel'
export type { AdvanceGeometryAnimationContext, EncodeFramePassContext } from './framePassEncoder/groupByShadingModel'
export { encodeAndSubmitFrame } from './framePassEncoder/encodeAndSubmitFrame'
