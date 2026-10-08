import {expect,it} from 'vitest'
import {addCertifiedErrorUpper,sqrtTwoCertifiedErrorUpper} from '../src/services/nurbsErrorComposition'
it('encloses a lost half-ulp term and preserves exact zero contributions',()=>{
 expect(addCertifiedErrorUpper(1,2**-53)).toBe(1+2**-52)
 expect(addCertifiedErrorUpper(0,Number.MIN_VALUE)).toBe(Number.MIN_VALUE)
 expect(addCertifiedErrorUpper(Number.MIN_VALUE,Number.MIN_VALUE)).toBe(3*Number.MIN_VALUE)
 expect(addCertifiedErrorUpper(Number.MAX_VALUE,0)).toBe(Number.MAX_VALUE)
 expect(addCertifiedErrorUpper(Number.MAX_VALUE,Number.MAX_VALUE)).toBeNull()
 expect(addCertifiedErrorUpper(-1,2)).toBeNull()
 expect(addCertifiedErrorUpper(NaN,2)).toBeNull()
})

it('encloses sqrt(2) scaling using an exact rational oracle over normal/subnormal inputs',()=>{
 const coefficient=6369051672525773n,denominator=1n<<52n
 expect(coefficient*coefficient>2n*denominator*denominator).toBe(true)
 // Decode the actual binary64 output independently; check its square >=2*x^2
 // using integers, rather than trusting floating-point sqrt/product results.
 const dyadic=(x:number):[bigint,number]=>{
  const data=new DataView(new ArrayBuffer(8));data.setFloat64(0,x,false)
  const bits=data.getBigUint64(0,false),exponent=Number((bits>>52n)&2047n)
  return [exponent===0?bits&((1n<<52n)-1n):(1n<<52n)|(bits&((1n<<52n)-1n)),exponent===0?-1074:exponent-1075]
 }
 for(const input of [Number.MIN_VALUE,3*Number.MIN_VALUE,2**-1022,.25,1,1+2**-52,2**500,Number.MAX_VALUE/2]){
  const bound=sqrtTwoCertifiedErrorUpper(input)!
  expect(Number.isFinite(bound)).toBe(true)
  const [a,e]=dyadic(input),[b,f]=dyadic(bound),shift=2*(f-e)
  expect(shift>=0?(b*b<<BigInt(shift))>=2n*a*a:b*b>=(2n*a*a<<BigInt(-shift))).toBe(true)
 }
 expect(sqrtTwoCertifiedErrorUpper(0)).toBe(0)
 expect(sqrtTwoCertifiedErrorUpper(Number.MAX_VALUE)).toBeNull()
 expect(sqrtTwoCertifiedErrorUpper(-1)).toBeNull()
 expect(sqrtTwoCertifiedErrorUpper(Infinity)).toBeNull()
})
