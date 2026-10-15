import {kernelRuntime} from '../../geometry/kernel'
/** Outward rounding and invalid/overflow refusal are owned by Rust
 * (`nurbs_core::numerics::error_upper`); scalar ABI exports avoid a JSON
 * round-trip per composition. The kernel returns NaN on refusal. */
const certified=(upper:number):number|null=>Number.isFinite(upper)?upper:null
export const addCertifiedErrorUpper=(a:number,b:number):number|null=>certified(kernelRuntime().exports.abi_certified_error_add(a,b))
export const multiplyCertifiedErrorUpper=(a:number,b:number):number|null=>certified(kernelRuntime().exports.abi_certified_error_mul(a,b))
export const sqrtTwoCertifiedErrorUpper=(a:number):number|null=>certified(kernelRuntime().exports.abi_certified_error_sqrt_two(a))
