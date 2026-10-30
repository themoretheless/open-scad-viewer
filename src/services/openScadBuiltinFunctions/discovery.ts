import {OPENSCAD_2021_01_BUILTIN_FUNCTION_NAMES} from '../openScadBuiltinFunctions'
import {OPENSCAD_BUILTIN_MODULE_NAMES} from './moduleNames'

let inventory:ReadonlySet<string>|undefined
function builtinNames():ReadonlySet<string> {
 return inventory??=new Set([...OPENSCAD_2021_01_BUILTIN_FUNCTION_NAMES,...OPENSCAD_BUILTIN_MODULE_NAMES])
}
/** Compatibility discovery API includes callable modules as well as functions. */
export function isOpenScadBuiltinFunction(name:string):boolean {return builtinNames().has(name)}

export function findClosestOpenScadBuiltinFunction(name:string,maxDistance=3):string|null {
 if(!Number.isSafeInteger(maxDistance)||maxDistance<0||!name)return null
 let best:string|null=null,bound=maxDistance+1
 for(const candidate of builtinNames()){
  if(Math.abs(candidate.length-name.length)>=bound)continue
  let row=Array.from({length:candidate.length+1},(_,i)=>i)
  for(let i=0;i<name.length;i++){
   const next=[i+1]
   for(let j=0;j<candidate.length;j++)next.push(Math.min(next[j]+1,row[j+1]+1,row[j]+Number(name[i]!==candidate[j])))
   row=next
  }
  const distance=row[candidate.length]
  if(distance<bound){bound=distance;best=candidate}
 }
 return best
}
