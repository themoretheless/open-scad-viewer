export interface CurveOffsetDiagnostics {
 scope:'represented-offset-chain'
 method:'outward-line-pair-interval/1'|'outward-line-pair-interval-exact/2'
 crossings:[number,number][];contacts:[number,number][];uncertain:[number,number][];degenerate:number[]
 complete:boolean;checks:number;totalPairs:number;enumerationComplete:boolean;predicatesComplete:boolean;simple:boolean
 originalOffsetTopologyCertified:false
}
/** Validate diagnostics of represented chords without certifying the original offset. */
export function validCurveOffsetDiagnostics(value:unknown,segments:number):value is CurveOffsetDiagnostics {
 if(!value||typeof value!=='object'||!Number.isInteger(segments)||segments<1||segments>65536)return false
 const d=value as CurveOffsetDiagnostics,total=segments*(segments-1)/2
 if(d.scope!=='represented-offset-chain'||!['outward-line-pair-interval/1','outward-line-pair-interval-exact/2'].includes(d.method)||d.originalOffsetTopologyCertified!==false
  ||d.totalPairs!==total||!Number.isInteger(d.checks)||d.checks<0||d.checks>Math.min(total,1000000))return false
 const index=(n:unknown):n is number=>Number.isInteger(n)&&Number(n)>=0&&Number(n)<segments
 const seen=new Set<string>()
 for(const pairs of [d.crossings,d.contacts,d.uncertain]){
  if(!Array.isArray(pairs)||pairs.length>d.checks)return false
  for(const pair of pairs){
   if(!Array.isArray(pair)||pair.length!==2||!index(pair[0])||!index(pair[1])||pair[0]>=pair[1])return false
   const key=pair.join(':');if(seen.has(key))return false;seen.add(key)
  }
 }
 if(seen.size>d.checks||!Array.isArray(d.degenerate)||d.degenerate.length>segments
  ||!d.degenerate.every(index)||new Set(d.degenerate).size!==d.degenerate.length)return false
 return d.enumerationComplete===(d.checks===total)&&d.predicatesComplete===(d.uncertain.length===0)
  &&d.complete===(d.enumerationComplete&&d.predicatesComplete)
  &&d.simple===(d.complete&&!d.crossings.length&&!d.contacts.length&&!d.degenerate.length)
}
