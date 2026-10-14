import {languageRequest} from './languages/kernel'
export type DegreeFunction = 'sin'|'cos'|'tan'|'asin'|'acos'|'atan'|'atan2'
export interface DegreeCall {readonly function:DegreeFunction;readonly args:readonly number[]}
const encode=(value:number)=>Object.is(value,-0)?'-0':Number.isFinite(value)?value:String(value)
/** Preserve order while bounding each ABI upload to 1024 numeric calls. */
export function evaluateDegreeBatch(calls:readonly DegreeCall[]):number[] {
 const values:number[]=[]
 for(let start=0;start<calls.length;start+=1024){
  const response=languageRequest(35,{calls:calls.slice(start,start+1024).map(call=>({function:call.function,args:call.args.map(encode)}))}) as {ok:boolean;value:(number|string)[];error?:{message:string}}
  if(!response.ok)throw new Error(response.error?.message??'Native degree math failed')
  values.push(...response.value.map(Number))
 }
 return values
}
// Pure degree rules allow reuse across evaluations. Bound retained angles explicitly.
const pairs=new Map<number|string,[number|undefined,number|undefined]>()
let previous:{name:'sin'|'cos';key:number|string}|undefined
export function evaluateDegreeScalar(name:DegreeFunction,args:readonly number[]):number {
 if(name==='sin'||name==='cos'){
  const angle=args[0],key=encode(angle),slot=name==='sin'?0:1
  let pair=pairs.get(key)
  if(pair?.[slot]===undefined){
   // Pair only after observing alternating functions across angles. A sin-only
   // workload continues to request one value rather than compute an unused cos.
   const paired=previous&&previous.name!==name&&previous.key!==key
   if(paired){
    const values=evaluateDegreeBatch([{function:'sin',args:[angle]},{function:'cos',args:[angle]}])
    pair=[values[0],values[1]]
   }else{
    pair=pair??[undefined,undefined]
    pair[slot]=evaluateDegreeBatch([{function:name,args:[angle]}])[0]
   }
   if(!pairs.has(key)&&pairs.size>=256)pairs.delete(pairs.keys().next().value!)
   pairs.set(key,pair)
  }
  previous={name,key}
  return pair![slot]!
 }
 previous=undefined
 return evaluateDegreeBatch([{function:name,args}])[0]
}
