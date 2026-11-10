/** Stable public B-rep API; implementations live in their owning domains. */
export * from './brep/core'
export * from '../../services/sweep/construction/brep'
export {readNativeSweepPlanarCapFaces,createDirectMiterOwnedBody,readMiterAffineReplayRouting,type NativeDirectMiterGeometry,createNativeDirectMiterGeometry,type ProgressiveBrepSweepOptions,type ProgressiveAnnularFilling,reconstructCertifiedProgressiveStations,reconstructCertifiedProgressiveStationsAsync} from '../../services/sweep/construction/extendedBrep'
export {createProgressiveBrepProfileBody as proposeProgressiveBrepProfileBody,streamProgressiveBrepProfileBody as streamProposedProgressiveBrepProfileBody,type ProgressiveBrepBody as ProposedProgressiveBrepBody} from '../../services/sweep/construction/extendedBrep'
