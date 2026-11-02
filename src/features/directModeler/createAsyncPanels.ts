import {defineAsyncComponent} from 'vue'

// Preserve the original per-instance async component wrappers and lazy imports.
export function createAsyncPanels() {
  const ExpressionShellCheck=defineAsyncComponent(()=>import('../../components/ExpressionShellCheck.vue'))
  const SurfacePairChecks=defineAsyncComponent(()=>import('../../components/SurfacePairChecks.vue'))
  const SurfaceDistanceCheck=defineAsyncComponent(()=>import('../../components/SurfaceDistanceCheck.vue'))
  const ProfileBridgeCheck=defineAsyncComponent(()=>import('../../components/ProfileBridgeCheck.vue'))
  const FrozenGraphBodyPanel=defineAsyncComponent(()=>import('../../components/FrozenGraphBodyPanel.vue'))
  const SourceGraphG2Preview=defineAsyncComponent(()=>import('../../components/SourceGraphG2Preview.vue'))
  const SourceBodyFailureNotice=defineAsyncComponent(()=>import('../../components/SourceBodyFailureNotice.vue'))
  const CadDimensionOverlay=defineAsyncComponent(()=>import('../../components/CadDimensionOverlay.vue'))
  const SolidGcodePanel=defineAsyncComponent(()=>import('../GcodePanel.vue'))
  const SolidLaserPanel=defineAsyncComponent(()=>import('../LaserCamPanel.vue'))
  const VrControls=defineAsyncComponent(()=>import('../../components/VrControls.vue'))
  const SceneVirtualList=defineAsyncComponent(()=>import('../../components/SceneVirtualList.vue'))
  const SketchDimensionPanel=defineAsyncComponent(()=>import('../../components/SketchDimensionPanel.vue'))
  const ProfileIntersectionPresentation=defineAsyncComponent(()=>import('../../components/ProfileIntersectionPresentation.vue'))
  const CurvePointTrimControls=defineAsyncComponent(()=>import('../../components/CurvePointTrimControls.vue'))
  const CurveOffsetPreview=defineAsyncComponent(()=>import('../../components/CurveOffsetPreview'))
  const CurveOffsetConstructionInfo=defineAsyncComponent(()=>import('../../components/CurveOffsetConstructionInfo'))
  const CurveOffsetControls=defineAsyncComponent(()=>import('../../components/CurveOffsetControls'))
  const CoonsPreparationControls=defineAsyncComponent(()=>import('../../components/CoonsPreparationControls'))
  const ShellDistanceSummary=defineAsyncComponent(()=>import('../../components/ShellDistanceSummary.vue'))
  const SolidVolumeWitness=defineAsyncComponent(()=>import('../../components/SolidVolumeWitness.vue'))
  const SolidVolumeDistance=defineAsyncComponent(()=>import('../../components/SolidVolumeDistance.vue'))
  const MaterialPathCheck=defineAsyncComponent(()=>import('../../components/MaterialPathCheck.vue'))
  return {
    ExpressionShellCheck, SurfacePairChecks, SurfaceDistanceCheck, ProfileBridgeCheck,
    FrozenGraphBodyPanel, SourceGraphG2Preview, SourceBodyFailureNotice, CadDimensionOverlay,
    SolidGcodePanel, SolidLaserPanel, VrControls, SceneVirtualList,
    SketchDimensionPanel, ProfileIntersectionPresentation, CurvePointTrimControls, CurveOffsetPreview,
    CurveOffsetConstructionInfo, CurveOffsetControls, CoonsPreparationControls, ShellDistanceSummary,
    SolidVolumeWitness, SolidVolumeDistance, MaterialPathCheck
  }
}
