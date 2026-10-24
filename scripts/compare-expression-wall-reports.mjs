// Exact regression comparison; agreement does not qualify unresolved geometry.
import assert from 'node:assert/strict'
import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {isDeepStrictEqual} from 'node:util'
const fullReplay=process.argv.includes('--full-replay')
const transitionOutcomes=process.argv.includes('--transition-outcomes')
const reclassify=process.argv.includes('--intrinsic-reclassification')
const allowWorkReduction=process.argv.includes('--work-reduction')||reclassify
const [beforePath,afterPath,outputPath]=process.argv.slice(2).filter(value=>!['--work-reduction','--intrinsic-reclassification','--transition-outcomes','--full-replay'].includes(value))
assert(beforePath&&afterPath,'Provide before and after native wall reports')
const read=path=>{const bytes=readFileSync(path);return {report:JSON.parse(bytes),sha256:createHash('sha256').update(bytes).digest('hex')}}
const before=read(beforePath),after=read(afterPath)
if(fullReplay){
 assert(!allowWorkReduction&&!transitionOutcomes,'Full replay mode requires strict equality')
 const a=before.report,b=after.report
 for(const replay of [a,b]){
  assert(typeof replay.schema==='string'&&replay.schema.length>0,'Missing replay schema')
  assert(typeof replay.requestSha256==='string'&&/^[0-9a-f]{64}$/.test(replay.requestSha256),'Invalid request identity')
  assert(Number.isSafeInteger(replay.requestBytes)&&replay.requestBytes>0,'Invalid source request byte count')
 }
 assert.equal(a.schema,b.schema,'Replay schemas changed')
 assert.equal(a.requestSha256,b.requestSha256,'Source request identity changed')
 assert.equal(a.requestBytes,b.requestBytes,'Source request byte count changed')
 assert(isDeepStrictEqual(a.request,b.request),'Source request changed')
 assert(a.response?.shell?.alignedChordCheck?.report&&b.response?.shell?.alignedChordCheck?.report,
  'Require complete expression inspection replays')
 assert(isDeepStrictEqual(a.response,b.response),'Full inspection response changed outside or inside wall coverage')
 before.report=a.response.shell.alignedChordCheck.report
 after.report=b.response.shell.alignedChordCheck.report
}
for(const {report} of [before,after]){
 assert.equal(report.method,'expression-support-aligned-chord-coverage')
 assert(Array.isArray(report.pending),'Preserve every unresolved UV product')
 for(const field of ['spans','cache_hits','intrinsic_spans','profile_transition_spans'])
  assert(Number.isSafeInteger(report[field])&&report[field]>=0,'Invalid native work counter')
 if(report.span_stages){
  const stages=['position','normal','shared_corner','original_pair','retained_pair','corrected_pair'].map(key=>report.span_stages[key])
  assert(stages.every(value=>Number.isSafeInteger(value)&&value>=0),'Invalid stage counter')
  assert.equal(stages.reduce((a,b)=>a+b,0)+report.intrinsic_spans+report.profile_transition_spans,report.spans,
   'Stage accounting must cover all native span work')
 }

}
if(transitionOutcomes){
 const fields=['profile_transition_inconclusive','profile_transition_unresolved']
 for(const {report} of [before,after])if(fields.some(k=>report[k]!==undefined)){
  assert([...fields,'profile_transition_checked','profile_transition_excluded'].every(k=>Number.isSafeInteger(report[k])&&report[k]>=0),'Invalid transition outcome count')
  assert.equal(report.profile_transition_excluded+report.profile_transition_inconclusive+report.profile_transition_unresolved,
   report.profile_transition_checked,'Every transition query must have exactly one outcome')
 }
 if(fields.some(k=>before.report[k]!==undefined))for(const k of fields)
  assert.equal(after.report[k],before.report[k],'Existing transition outcomes must remain exact')
}
// Preserve region order, UV endpoints, refusal reasons and exclusion counts.
// Strict mode also preserves every work counter; sharing mode allows only
// diagnostic work fields to differ and requires nonincreasing total span work.
const coverage=report=>{
 const {spans,span_stages,cache_hits,intrinsic_spans,profile_transition_spans,...rest}=report
 if(transitionOutcomes){delete rest.profile_transition_inconclusive;delete rest.profile_transition_unresolved}
 if(reclassify){delete rest.angle_excluded;delete rest.intrinsic_excluded}
 return rest
}
if(reclassify){
 for(const {report} of [before,after])for(const field of ['angle_excluded','intrinsic_excluded'])
  assert(Number.isSafeInteger(report[field])&&report[field]>=0,'Invalid exclusion counter')
 assert(after.report.intrinsic_excluded>=before.report.intrinsic_excluded,'Intrinsic proof count must not decrease')
 assert(after.report.angle_excluded<=before.report.angle_excluded,'Later angle proof count must not increase')
 assert.equal(after.report.intrinsic_excluded+after.report.angle_excluded,
  before.report.intrinsic_excluded+before.report.angle_excluded,'Every reclassified proof must preserve the total')
}
if(allowWorkReduction){
 assert(isDeepStrictEqual(coverage(after.report),coverage(before.report)),
  'Native coverage, interval domains or refusal reasons changed')
 assert(Number.isSafeInteger(after.report.spans)&&after.report.spans>=0&&after.report.spans<=before.report.spans,
  'Sharing must not increase total span work')
}else assert(isDeepStrictEqual(after.report,before.report),'Native coverage, interval domains or work accounting changed')
const result={schema:'expression-wall-exact-regression/1',passed:true,
 before:{path:beforePath,sha256:before.sha256},after:{path:afterPath,sha256:after.sha256},
 fullInspectionResponseExact:fullReplay,
 transitionOutcomeSchemaExtension:transitionOutcomes,
 pendingRegions:after.report.pending.length,cells:after.report.cells,spans:after.report.spans,
 comparison:reclassify?'exact-coverage-with-intrinsic-reclassification':allowWorkReduction?'exact-coverage-with-nonincreasing-span-work':'exact-report',
 spanWorkSaved:before.report.spans-after.report.spans,
 ...(reclassify?{intrinsicProofsAdded:after.report.intrinsic_excluded-before.report.intrinsic_excluded,angleProofsReplaced:before.report.angle_excluded-after.report.angle_excluded}:{}),
 geometryQualified:false,scope:'Regression comparison only; unresolved geometry remains unresolved'}
const text=JSON.stringify(result,null,2)+'\n'
if(outputPath)writeFileSync(outputPath,text);else process.stdout.write(text)
