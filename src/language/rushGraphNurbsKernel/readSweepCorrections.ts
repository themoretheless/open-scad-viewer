type SweepCorrections={circleCorrection?:{tolerance:number;quantum:number;maxWork:number};capCorrection?:{tolerance:number;quantum:number;maxWork:number;authoredFrame?:boolean}}
export function readSweepCorrections(node:any,miter=false,authored=false):SweepCorrections {
 const out:SweepCorrections={}
 for(const kind of ['circle','cap'] as const){
  const tolerance=node[`${kind}_correction_tolerance`],quantum=node[`${kind}_correction_quantum`],work=node[`${kind}_correction_max_work`]
  if(tolerance===undefined){
   if(quantum!==undefined||work!==undefined||kind==='cap'&&authored&&node.cap_correction_authored_frame===true)
    throw new Error(`${miter?'Miter '+kind:kind==='circle'?'Profile circle':'Cap'} correction requires a displacement tolerance`)
  }else out[`${kind}Correction`]={tolerance,quantum:quantum??2**-40,maxWork:work??1000000,...(kind==='cap'&&authored?{authoredFrame:node.cap_correction_authored_frame}:{})}
 }
 return out
}
