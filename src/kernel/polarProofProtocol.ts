/** Transport checks for native interval certificates; geometry is proved in Rust. */
export interface PolarProofEvidence {
 coordinates:number[][];jacobian:number[][][];rowSigns:number[];diagonalLower:number[];determinantLower:number
}
export function validPolarProofEvidence(value:unknown):value is PolarProofEvidence {
 const p=value as PolarProofEvidence
 const finiteRow=(v:unknown,n:number):v is number[]=>Array.isArray(v)&&v.length===n&&v.every(Number.isFinite)
 const shape=!!p&&Array.isArray(p.coordinates)&&p.coordinates.length===3&&p.coordinates.every(r=>finiteRow(r,4))
  &&Array.isArray(p.jacobian)&&p.jacobian.length===2&&p.jacobian.every(r=>Array.isArray(r)&&r.length===2&&r.every(i=>finiteRow(i,2)&&i[0]<=i[1]))
  &&finiteRow(p.rowSigns,2)&&p.rowSigns.every(s=>s===1||s===-1)
  &&finiteRow(p.diagonalLower,2)&&p.diagonalLower.every((d,i)=>d>0&&d<=(p.rowSigns[i]===1?p.jacobian[i][i][0]:-p.jacobian[i][i][1]))
  &&typeof p.determinantLower==='number'&&Number.isFinite(p.determinantLower)&&p.determinantLower>0
 if(!shape)return false
 const a=p.jacobian[0][1].map(v=>v*p.rowSigns[0]),b=p.jacobian[1][0].map(v=>v*p.rowSigns[1])
 const upperCross=Math.max(...a.flatMap(x=>b.map(y=>x*y)))
 return p.determinantLower<=p.diagonalLower[0]*p.diagonalLower[1]-upperCross
}
export const axisPolarFrames=[0,1,2].flatMap(axis=>[1,-1].flatMap(sign=>[1,-1].map(zsign=>{
 const frame=Array.from({length:3},()=>[0,0,0,0]);frame[0][axis]=sign;frame[1][(axis+1)%3]=sign;frame[2][(axis+2)%3]=zsign;return frame
})))
export function sameFrame(a:unknown,b:unknown):boolean{return Array.isArray(a)&&Array.isArray(b)&&a.length===b.length&&a.every((r,i)=>Array.isArray(r)&&Array.isArray(b[i])&&r.length===b[i].length&&r.every((x,k)=>typeof x==='number'&&Number.isFinite(x)&&x===b[i][k]))}
