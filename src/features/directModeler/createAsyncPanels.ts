import {defineAsyncComponent} from 'svet'

const ExpressionShellCheck=defineAsyncComponent(()=>import('../../components/ExpressionShellCheck.svet'))
const SurfacePairChecks=defineAsyncComponent(()=>import('../../components/SurfacePairChecks.svet'))
const SurfaceDistanceCheck=defineAsyncComponent(()=>import('../../components/SurfaceDistanceCheck.svet'))
const ProfileBridgeCheck=defineAsyncComponent(()=>import('../../components/ProfileBridgeCheck.svet'))
const FrozenGraphBodyPanel=defineAsyncComponent(()=>import('../../components/FrozenGraphBodyPanel.svet'))
const SourceGraphG2Preview=defineAsyncComponent(()=>import('../../components/SourceGraphG2Preview.svet'))
const SourceBodyFailureNotice=defineAsyncComponent(()=>import('../../components/SourceBodyFailureNotice.svet'))
const CadDimensionOverlay=defineAsyncComponent(()=>import('../../components/CadDimensionOverlay.svet'))
const SolidGcodePanel=defineAsyncComponent(()=>import('../GcodePanel.svet'))
const SolidLaserPanel=defineAsyncComponent(()=>import('../LaserCamPanel.svet'))
const VrControls=defineAsyncComponent(()=>import('../../components/VrControls.svet'))
const SceneVirtualList=defineAsyncComponent(()=>import('../../components/SceneVirtualList.svet'))
const SketchDimensionPanel=defineAsyncComponent(()=>import('../../components/SketchDimensionPanel.svet'))
const ProfileIntersectionPresentation=defineAsyncComponent(()=>import('../../components/ProfileIntersectionPresentation.svet'))
const CurvePointTrimControls=defineAsyncComponent(()=>import('../../components/CurvePointTrimControls.svet'))
const CurveOffsetPreview=defineAsyncComponent(()=>import('../../components/CurveOffsetPreview'))
const CurveOffsetConstructionInfo=defineAsyncComponent(()=>import('../../components/CurveOffsetConstructionInfo'))
const CurveOffsetControls=defineAsyncComponent(()=>import('../../components/CurveOffsetControls'))
const CoonsPreparationControls=defineAsyncComponent(()=>import('../../components/CoonsPreparationControls'))
const ShellDistanceSummary=defineAsyncComponent(()=>import('../../components/ShellDistanceSummary.svet'))
const SolidVolumeWitness=defineAsyncComponent(()=>import('../../components/SolidVolumeWitness.svet'))
const SolidVolumeDistance=defineAsyncComponent(()=>import('../../components/SolidVolumeDistance.svet'))
const MaterialPathCheck=defineAsyncComponent(()=>import('../../components/MaterialPathCheck.svet'))

const panels = {
  ExpressionShellCheck, SurfacePairChecks, SurfaceDistanceCheck, ProfileBridgeCheck,
  FrozenGraphBodyPanel, SourceGraphG2Preview, SourceBodyFailureNotice, CadDimensionOverlay,
  SolidGcodePanel, SolidLaserPanel, VrControls, SceneVirtualList,
  SketchDimensionPanel, ProfileIntersectionPresentation, CurvePointTrimControls, CurveOffsetPreview,
  CurveOffsetConstructionInfo, CurveOffsetControls, CoonsPreparationControls, ShellDistanceSummary,
  SolidVolumeWitness, SolidVolumeDistance, MaterialPathCheck
}

export function createAsyncPanels() {
  return panels
}
