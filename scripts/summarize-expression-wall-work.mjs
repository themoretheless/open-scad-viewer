// Diagnostic accounting only; this script grants no geometric qualification.
import {readFileSync,writeFileSync} from 'node:fs'
import assert from 'node:assert/strict'
const [input,output]=process.argv.slice(2)
assert(input,'Provide a native expression wall report JSON path')
const report=JSON.parse(readFileSync(input,'utf8'))
assert.equal(report.method,'expression-support-aligned-chord-coverage')
const count=value=>Number.isSafeInteger(value)&&value>=0
assert(count(report.spans)&&Array.isArray(report.pending))
const stages={intrinsic:report.intrinsic_spans,profile_transition:report.profile_transition_spans}
const detailed=report.span_stages!==undefined
if(detailed){
 for(const key of ['position','normal','shared_corner','original_pair','retained_pair','corrected_pair'])stages[key]=report.span_stages[key]
}
assert(Object.values(stages).every(count),'Each work stage must be a nonnegative integer')
const attributedSpans=Object.values(stages).reduce((a,b)=>a+b,0)
assert(attributedSpans<=report.spans,'Stages cannot exceed total work')
if(detailed)assert.equal(attributedSpans,report.spans,'Every native span must be attributed exactly once')
const pairs=new Map(),reasons={budget:0,enclosure:0,resolution:0}
for(const region of report.pending){
 assert(Array.isArray(region.faces)&&region.faces.length===2&&region.faces.every(count))
 assert(Object.hasOwn(reasons,region.pending_reason))
 reasons[region.pending_reason]++
 const key=region.faces.join(':');pairs.set(key,(pairs.get(key)??0)+1)
}
for(const key of Object.keys(reasons))assert.equal(reasons[key],report[`${key}_pending`])
const summary={
 schema:'expression-wall-work-summary/1',source:input,
 geometryQualified:false,scope:'Work accounting and pending-region counts only; no whole-body thickness admission',
 complete:report.complete,cells:report.cells,spans:report.spans,domainCells:report.domain_cells,
 initialPairsChecked:report.initial_pairs_checked,totalPairs:report.total_pairs,
 pendingRegions:report.pending.length,pendingFacePairs:pairs.size,pendingReasons:reasons,
 detailedStages:detailed,unattributedSpans:report.spans-attributedSpans,
 stages:Object.entries(stages).sort((a,b)=>b[1]-a[1]).map(([stage,spans])=>({stage,spans,fractionOfSpanWork:report.spans?spans/report.spans:0})),
 pendingPairs:[...pairs].sort((a,b)=>b[1]-a[1]).map(([pair,regions])=>({faces:pair.split(':').map(Number),regions})),
}
const text=JSON.stringify(summary,null,2)+'\n'
if(output)writeFileSync(output,text)
else process.stdout.write(text)
