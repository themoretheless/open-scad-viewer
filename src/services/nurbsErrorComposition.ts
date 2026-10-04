import {callNurbsRust} from './geometry/nurbs'
const compose=(kind:'add'|'multiply'|'sqrt-two',a:number,b?:number):number|null=>{
 try{return callNurbsRust<{errorUpper:number|null}>('sweep_error_upper_compose',{kind,a,b:b??null}).errorUpper}catch{return null}
}
/** Outward rounding and invalid/overflow refusal are owned by Rust. */
export const addCertifiedErrorUpper=(a:number,b:number):number|null=>compose('add',a,b)
export const multiplyCertifiedErrorUpper=(a:number,b:number):number|null=>compose('multiply',a,b)
export const sqrtTwoCertifiedErrorUpper=(a:number):number|null=>compose('sqrt-two',a)
