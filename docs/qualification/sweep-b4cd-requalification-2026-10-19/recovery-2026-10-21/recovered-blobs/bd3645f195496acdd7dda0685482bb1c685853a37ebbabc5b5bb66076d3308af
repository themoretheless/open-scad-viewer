import {kernelRuntime} from './geometry/kernel'

/** Upper bound for a sum of already certified nonnegative binary64 bounds.
 * One successor encloses round-to-nearest addition. A zero term is exact.
 * Invalid/overflowing input refuses rather than substituting a finite bound. */
export function addCertifiedErrorUpper(a:number,b:number):number|null {
 const upper=kernelRuntime().exports.abi_certified_error_add(a,b)
 return Number.isFinite(upper)?upper:null
}

/** sqrt(2) times a certified nonnegative bound, rounded outward.
 * The literal is the binary64 number 6369051672525773 / 2^52; its exact
 * square exceeds 2. One successor encloses the product rounding, including
 * subnormals. No assumption about Math.sqrt's rounding is required. */
export function sqrtTwoCertifiedErrorUpper(value:number):number|null {
 const upper=kernelRuntime().exports.abi_certified_error_sqrt_two(value)
 return Number.isFinite(upper)?upper:null
}

/** One successor encloses a product of certified nonnegative upper bounds. */
export function multiplyCertifiedErrorUpper(a:number,b:number):number|null {
 const upper=kernelRuntime().exports.abi_certified_error_mul(a,b)
 return Number.isFinite(upper)?upper:null
}

