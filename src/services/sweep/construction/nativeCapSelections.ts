import type {NurbsBrep} from '../../geometry/brep/core'
// Native-derived face selections are optimization hints, not proof premises.
// Affine transforms preserve the face indices; every geometry audit stays fresh.
export const nativeSweepPlanarCapSelections=new WeakMap<NurbsBrep,number[]>()
export const readNativeSweepPlanarCapFaces=(model:NurbsBrep):number[]|undefined=>nativeSweepPlanarCapSelections.get(model)?.slice()
export function transferNativeCapSelection(source:NurbsBrep,target:NurbsBrep):NurbsBrep {
 const selection=nativeSweepPlanarCapSelections.get(source)
 if(selection)nativeSweepPlanarCapSelections.set(target,selection.slice())
 return target
}
