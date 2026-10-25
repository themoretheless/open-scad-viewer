import {readFileSync,writeFileSync} from 'node:fs'
import {createSourceBodyRecord,withSourceBodyInspection} from '../src/services/sourceBodyArchive'
import type {ExpressionInspectionOptions} from '../src/services/expressionShellInspection'
import type {SourceBodyOptions} from '../src/services/sourceBody'
const fixture=JSON.parse(readFileSync('docs/qualification/mapped-retained-carrier-2026-10-08/expression-shell-inspection-native-2026-10-09.json','utf8'))
const {op:_,...options}=fixture.request as ExpressionInspectionOptions&{op:string}
if(process.argv.includes('--aligned-chords'))options.alignedChordCheck={
 minimumMm:1,maxSineSquared:1e-4,toleranceUv:0.05,controls:1000,
 cells:process.argv.includes('--aligned-chords-bounded')?10000:0,
 spans:process.argv.includes('--aligned-chords-bounded')?100000:0,
 domainCells:process.argv.includes('--aligned-chords-bounded')?100000:0,
}
if(process.argv.includes('--neighbor-tangency')){
 const fixture=JSON.parse(readFileSync('docs/qualification/mapped-retained-carrier-2026-10-08/expression-neighbor-tangency-dispatch-2026-10-10.json','utf8'))
 options.neighborTangencyCheck=fixture[process.argv.includes('--neighbor-tangency-unproven')?'unproven':'admitted'].request.neighborTangencyCheck
}
if(process.argv.includes('--support-tangency')){
 const fixture=JSON.parse(readFileSync('docs/qualification/mapped-retained-carrier-2026-10-08/expression-support-tangency-dispatch-2026-10-09.json','utf8'))
 options.supportTangencyCheck=fixture[process.argv.includes('--support-tangency-unproven')?'unproven':'admitted'].request.supportTangencyCheck
}
if(process.argv.includes('--radius')){
 const radiusFixture=JSON.parse(readFileSync('docs/qualification/mapped-retained-carrier-2026-10-08/expression-radius-dispatch-2026-10-09.json','utf8'))
 const state=process.argv.includes('--radius-violation')?'violation':process.argv.includes('--radius-unproven')?'unproven':'admitted'
 options.radiusCheck=radiusFixture[state].request.radiusCheck
}
if(process.argv[3]==='--pending'){options.inspectionLimits.contacts.cells=0;options.inspectionLimits.contacts.exactWork=0}
const original=options.proposalDefinition as {originalBody:SourceBodyOptions['definition']}
const source:SourceBodyOptions={definition:original.originalBody,limits:options.sourceLimits,endpointSpans:256,displaySegments:8,
 faceDisplay:{divisions:4,toleranceUv:1e-8,domainCellsPerFace:512}}
const record=withSourceBodyInspection(createSourceBodyRecord('expression-source','Кольцевая деталь — переход',source),options)
if(!process.argv[2])throw new Error('Provide UI fixture output path')
writeFileSync(process.argv[2],JSON.stringify({version:1,sketches:[],bodies:[],sourceBodies:[record]})+'\n')
