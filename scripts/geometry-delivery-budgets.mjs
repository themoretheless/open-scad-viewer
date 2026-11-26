import profiles231 from './geometry-delivery-profiles/profiles-231.mjs'
import profiles230 from './geometry-delivery-profiles/profiles-230.mjs'
import profiles229 from './geometry-delivery-profiles/profiles-229.mjs'
import profiles228 from './geometry-delivery-profiles/profiles-228.mjs'
import profiles227 from './geometry-delivery-profiles/profiles-227.mjs'
import profiles6 from './geometry-delivery-profiles/profiles-6-54.mjs'
import profiles55 from './geometry-delivery-profiles/profiles-55-102.mjs'
import profiles103 from './geometry-delivery-profiles/profiles-103-150.mjs'
import profiles151 from './geometry-delivery-profiles/profiles-151-198.mjs'
import profiles199 from './geometry-delivery-profiles/profiles-199-226.mjs'

// Packaging policy, not geometry tolerances, proof work, memory or WASM limits.
// A new artifact cannot inherit a larger budget without a reviewed exact pin.
const profiles=Object.freeze({
  ...profiles231,
  ...profiles230,
  ...profiles229,
  ...profiles228,
  ...profiles227,
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
