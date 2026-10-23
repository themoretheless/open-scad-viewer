import {expect} from 'vitest'
/** Keep historical fixtures immutable while accepting strictly tighter native
 * enclosures. All remaining fields retain the caller's exact comparison. */
export function normalizeRefinedNativeEvidence(actual:any,reference:any,key=''):any {
 if(key==='contractionUpper'&&typeof reference==='number'&&typeof actual==='number'){
  expect(Number.isFinite(actual)).toBe(true)
  expect(actual).toBeGreaterThanOrEqual(0)
  expect(actual).toBeLessThanOrEqual(reference)
  return reference
 }
 if(['firstUv','secondUv','pointIntervalMm'].includes(key)&&Array.isArray(actual)&&Array.isArray(reference)){
  expect(actual).toHaveLength(reference.length)
  for(let axis=0;axis<reference.length;axis++){
   expect(actual[axis]).toHaveLength(2)
   const [lo,hi]=actual[axis],old=reference[axis]
   expect(Number.isFinite(lo)&&Number.isFinite(hi)).toBe(true)
   expect(lo).toBeLessThanOrEqual(hi)
   expect(lo).toBeGreaterThanOrEqual(old[0])
   expect(hi).toBeLessThanOrEqual(old[1])
  }
  return reference
 }
 if(Array.isArray(actual)&&Array.isArray(reference))return actual.map((value,index)=>normalizeRefinedNativeEvidence(value,reference[index]))
 if(actual&&reference&&typeof actual==='object'&&typeof reference==='object')return Object.fromEntries(Object.entries(actual).map(([name,value])=>[name,normalizeRefinedNativeEvidence(value,reference[name],name)]))
 return actual
}
