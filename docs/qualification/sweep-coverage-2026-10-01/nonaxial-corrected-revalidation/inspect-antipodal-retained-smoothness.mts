import {readFileSync,writeFileSync} from 'node:fs'
import {compileRushFrontend} from '../../../../src/services/rushFrontend'
import {buildOwnNurbsAsync} from '../../../../src/services/rushGraphNurbsKernel'
import {inspectMiterProfileSmoothness} from '../../../../src/services/miterProfileSmoothness'
const source=readFileSync('examples/rush/closed-antipodal-spatial-rmf-affine-hollow-body.r','utf8')
const built=await buildOwnNurbsAsync(compileRushFrontend(source).document,{action:'build',display:{segments:2,subdivisionLevels:0}})
if(!built.nativeGeometry)throw Error('Missing native body')
const model=JSON.parse(built.nativeGeometry.geometryJson).geometry
const proof=inspectMiterProfileSmoothness(model,[])
writeFileSync('docs/qualification/sweep-coverage-2026-10-01/nonaxial-corrected-revalidation/rmf-antipodal-retained-smoothness.json',JSON.stringify(proof,null,2)+'\n')
console.log(JSON.stringify({extractionComplete:proof.extractionComplete,profileG1:proof.profileG1Certified,
 profileG2:proof.profile.exactG1G2Certified,station:proof.stationContinuity,work:proof.totalExactWork,
 profileUnresolved:proof.profile.unresolvedSeams.length,stationUnresolved:proof.station.g2.unresolvedSeams.length}))
