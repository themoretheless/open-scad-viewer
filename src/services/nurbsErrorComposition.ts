/** Upper bound for a sum of already certified nonnegative binary64 bounds.
 * One successor encloses round-to-nearest addition. A zero term is exact.
 * Invalid/overflowing input refuses rather than substituting a finite bound. */
export function addCertifiedErrorUpper(a:number,b:number):number|null {
 if(!Number.isFinite(a)||!Number.isFinite(b)||a<0||b<0)return null
 if(a===0)return b
 if(b===0)return a
 const sum=a+b
 if(!Number.isFinite(sum))return null
 const bits=new DataView(new ArrayBuffer(8))
 bits.setFloat64(0,sum,false)
 bits.setBigUint64(0,bits.getBigUint64(0,false)+1n,false)
 const upper=bits.getFloat64(0,false)
 return Number.isFinite(upper)?upper:null
}

/** sqrt(2) times a certified nonnegative bound, rounded outward.
 * The literal is the binary64 number 6369051672525773 / 2^52; its exact
 * square exceeds 2. One successor encloses the product rounding, including
 * subnormals. No assumption about Math.sqrt's rounding is required. */
export function sqrtTwoCertifiedErrorUpper(value:number):number|null {
 if(!Number.isFinite(value)||value<0)return null
 if(value===0)return 0
 const product=value*1.4142135623730951
 if(!Number.isFinite(product))return null
 const bits=new DataView(new ArrayBuffer(8))
 bits.setFloat64(0,product,false)
 bits.setBigUint64(0,bits.getBigUint64(0,false)+1n,false)
 const upper=bits.getFloat64(0,false)
 return Number.isFinite(upper)?upper:null
}

/** One successor encloses a product of certified nonnegative upper bounds. */
export function multiplyCertifiedErrorUpper(a:number,b:number):number|null {
 if(!Number.isFinite(a)||!Number.isFinite(b)||a<0||b<0)return null
 if(a===0||b===0)return 0
 const product=a*b
 if(!Number.isFinite(product))return null
 const bits=new DataView(new ArrayBuffer(8))
 bits.setFloat64(0,product,false)
 bits.setBigUint64(0,bits.getBigUint64(0,false)+1n,false)
 const upper=bits.getFloat64(0,false)
 return Number.isFinite(upper)?upper:null
}
