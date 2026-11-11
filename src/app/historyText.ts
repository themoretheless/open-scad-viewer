/** Immutable text split at JSON token boundaries, sharing unchanged tokens across snapshots.
 * The owner supplies the pool and releases unreachable entries after stack changes.
 */
export interface HistoryText { readonly chunks: readonly string[]; readonly characters: number }
export function packHistoryText(text: string, pool: Map<string,string>): HistoryText {
 const chunks:string[]=[]
 let start=0,quoted=false,escaped=false
 for(let i=0;i<text.length;i++) {
  const c=text[i]
  if(quoted){if(escaped)escaped=false;else if(c==='\\')escaped=true;else if(c==='"')quoted=false}
  else if(c==='"')quoted=true
  if(!quoted&&(c===','||c==='}'||c===']')&&i-start>=256) {
   const part=text.slice(start,i+1);let shared=pool.get(part)
   if(shared===undefined){shared=part;pool.set(part,shared)}chunks.push(shared);start=i+1
  }
 }
 if(start<text.length){const part=text.slice(start);let shared=pool.get(part);if(shared===undefined){shared=part;pool.set(part,shared)}chunks.push(shared)}
 return {chunks,characters:text.length}
}
export function unpackHistoryText(snapshot:HistoryText):string{return snapshot.chunks.join('')}
export function sweepHistoryTextPool(pool:Map<string,string>, snapshots:readonly HistoryText[]):number {
 const live=new Set(snapshots.flatMap(snapshot=>snapshot.chunks))
 let retained=0
 for(const chunk of pool.keys()){if(live.has(chunk))retained+=chunk.length;else pool.delete(chunk)}
 return retained
}
