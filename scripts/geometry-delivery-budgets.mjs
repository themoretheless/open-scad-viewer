import profiles6 from './geometry-delivery-profiles/profiles-6-54.mjs'
import profiles55 from './geometry-delivery-profiles/profiles-55-102.mjs'
import profiles103 from './geometry-delivery-profiles/profiles-103-150.mjs'
import profiles151 from './geometry-delivery-profiles/profiles-151-198.mjs'
import profiles199 from './geometry-delivery-profiles/profiles-199-226.mjs'

// Packaging policy, not geometry tolerances, proof work, memory or WASM limits.
// A new artifact cannot inherit a larger budget without a reviewed exact pin.
const profiles=Object.freeze({
  ...profiles6,
  ...profiles55,
  ...profiles103,
  ...profiles151,
  ...profiles199,
})

export function selectGeometryDeliveryBudget(wasmSha256){
  if(typeof wasmSha256!=='string'||!Object.hasOwn(profiles,wasmSha256))
    throw new Error('Geometry artifact has no reviewed delivery budget profile')
  return profiles[wasmSha256]
}
