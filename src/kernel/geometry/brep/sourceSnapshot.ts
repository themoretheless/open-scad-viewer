import type {NurbsBrep} from './core'

/** Rust serde defaults are part of the wire schema. Preserve all authored data,
 * while giving absent and explicitly empty optional topology metadata one form. */
export function brepSourceSnapshot(model:NurbsBrep):string {
 if(!model)return JSON.stringify(model)
 const ids=model.topologyIds
 const normalized=ids?{...model,topologyIds:{...ids,localTolerancesMm:ids.localTolerancesMm===undefined?{}:ids.localTolerancesMm,sewedEdges:ids.sewedEdges===undefined?[]:ids.sewedEdges}}:model
 return JSON.stringify(normalized,(_,v)=>v&&typeof v==='object'&&!Array.isArray(v)?Object.fromEntries(Object.keys(v).sort().map(k=>[k,v[k]])):v)
}
