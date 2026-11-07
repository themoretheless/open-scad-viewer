import {defineAsyncComponent} from 'svet'

const ExpressionShellCheck=defineAsyncComponent(()=>import('../../components/ExpressionShellCheck.svt'))
const SurfacePairChecks=defineAsyncComponent(()=>import('../../components/SurfacePairChecks.svt'))
const SurfaceDistanceCheck=defineAsyncComponent(()=>import('../../components/SurfaceDistanceCheck.svt'))
const ProfileBridgeCheck=defineAsyncComponent(()=>import('../../components/ProfileBridgeCheck.svt'))
const FrozenGraphBodyPanel=defineAsyncComponent(()=>import('../../components/FrozenGraphBodyPanel.svt'))
const SourceGraphG2Preview=defineAsyncComponent(()=>import('../../components/SourceGraphG2Preview.svt'))
const SourceBodyFailureNotice=defineAsyncComponent(()=>import('../../components/SourceBodyFailureNotice.svt'))
const CadDimensionOverlay=defineAsyncComponent(()=>import('../../components/CadDimensionOverlay.svt'))
const SolidGcodePanel=defineAsyncComponent(()=>import('../GcodePanel.svt'))
const SolidLaserPanel=defineAsyncComponent(()=>import('../LaserCamPanel.svt'))
const VrControls=defineAsyncComponent(()=>import('../../components/VrControls.svt'))
const SceneVirtualList=defineAsyncComponent(()=>import('../../components/SceneVirtualList.svt'))
const SketchDimensionPanel=defineAsyncComponent(()=>import('../../components/SketchDimensionPanel.svt'))
const ProfileIntersectionPresentation=defineAsyncComponent(()=>import('../../components/ProfileIntersectionPresentation.svt'))
const CurvePointTrimControls=defineAsyncComponent(()=>import('../../components/CurvePointTrimControls.svt'))
const CurveOffsetPreview=defineAsyncComponent(()=>import('../../components/CurveOffsetPreview'))
const CurveOffsetConstructionInfo=defineAsyncComponent(()=>import('../../components/CurveOffsetConstructionInfo'))
const CurveOffsetControls=defineAsyncComponent(()=>import('../../components/CurveOffsetControls'))
const CoonsPreparationControls=defineAsyncComponent(()=>import('../../components/CoonsPreparationControls'))
const ShellDistanceSummary=defineAsyncComponent(()=>import('../../components/ShellDistanceSummary.svt'))
const SolidVolumeWitness=defineAsyncComponent(()=>import('../../components/SolidVolumeWitness.svt'))
const SolidVolumeDistance=defineAsyncComponent(()=>import('../../components/SolidVolumeDistance.svt'))
const MaterialPathCheck=defineAsyncComponent(()=>import('../../components/MaterialPathCheck.svt'))

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
