import assert from 'node:assert/strict'
import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {dirname,resolve} from 'node:path'
import {readSweepManifest,sweepManifestReaderProvenance} from './sweep-manifest-reader.mjs'
import {stationFrameSources} from './station-frame-matrix-sources.mjs'
import {spatialRmfProfileG2BodySources} from './spatial-rmf-profile-g2-sources.mjs'
const [manifestPath,originalInventoryPath,stageRoot,output]=process.argv.slice(2)
assert.ok(manifestPath&&originalInventoryPath&&stageRoot&&output)
const hash=b=>createHash('sha256').update(b).digest('hex')
const bytes=readFileSync(manifestPath),manifest=readSweepManifest(manifestPath,bytes)
const originalBytes=readFileSync(originalInventoryPath),original=JSON.parse(originalBytes)
assert.equal(original.cases.length,157)
const contracts=new Map()
const station=[...stationFrameSources,'straight-progressive-body-station-g2.r','closed-spatial-rmf-varying-affine-hollow-body-station-g2.r','arc-length-periodic-authored-station-g2.r',...['circle-solid','circle-hollow','periodic-solid','periodic-hollow'].map(n=>`closed-frenet-arc-${n}-station-g2.r`)]
for(const source of station)contracts.set(source.replace(/\.r$/,'.step'),{source:`examples/rush/${source}`,station:'G2'})
for(const closed of [false,true])for(let bits=0;bits<8;bits++){
 const laws=[bits&1?'affine':null,bits&2?'authored':null,bits&4?'guide':null].filter(Boolean).join('-')||'plain'
 const name=`miter-laws-${closed?'closed':'open'}-${laws}-profile-g2`
 contracts.set(`${name}.step`,{source:`${name}.r`,station:'C0'})
}
const modes=['affine','affine-hollow','varying-affine-hollow','periodic-affine','periodic-varying-affine-hollow','parameter-varying-affine-hollow','parameter-periodic-varying-affine-hollow','periodic-varying-affine-three-holes','parameter-periodic-varying-affine-three-holes','rational-nonuniform-affine-hollow','antipodal-affine-hollow']
for(const [i,mode] of modes.entries())contracts.set(`closed-original-spatial-rmf-holonomy-${mode}-tube-profile-g2.step`,{source:`examples/rush/${spatialRmfProfileG2BodySources[i]}`,station:'C0'})
assert.equal(contracts.size,46)
const expected=[...original.cases.map(c=>c.file),...contracts.keys()]
assert.equal(new Set(expected).size,203)
assert.deepEqual(manifest.cases.map(c=>c.file).sort(),expected.sort(),'Complete current matrix must preserve all original cases and all explicit companions')
const checked=[]
for(const c of manifest.cases){
 const contract=contracts.get(c.file);if(!contract)continue
 assert.equal(c.sourceFile,contract.source)
 const sourceRoot=c.sourceFile.startsWith('examples/')?stageRoot:dirname(manifestPath)
 assert.equal(hash(readFileSync(resolve(sourceRoot,c.sourceFile))),c.sourceSha256,'Original companion source changed')
 const p=c.nativeProfileSmoothness,s=p.station
 assert.equal(p.profileG1Certified,true);assert.equal(p.profile.exactG1G2Certified,true)
 assert.equal(p.stationContinuity,contract.station)
 assert.equal(s.stationG1Certified,contract.station==='G2')
 assert.equal(s.stationG2Certified,contract.station==='G2')
 assert.equal(p.fullBoundarySmoothnessCertified,false)
 assert.equal(c.nativeVolume.solidGeometryCertified,true)
 const b=c.boundaryError??c.construction.boundaryCertificate
 assert.equal(b.continuousBound,true);assert.equal(b.withinBudget,true)
 assert.ok(Number.isFinite(b.errorUpper)&&b.errorUpper>=0&&b.errorUpper<=b.budget)
 checked.push({file:c.file,sourceSha256:c.sourceSha256,profile:'G2',station:contract.station})
}
assert.equal(checked.length,46)
writeFileSync(output,JSON.stringify({schema:'sweep-current-inventory/1',passed:true,manifestReaderProvenance:sweepManifestReaderProvenance,scope:'Exact203-case inventory and46 explicit companion requirements; must accompany independent STEP, complete boundary, global and seam audits',manifestSha256:hash(bytes),originalInventorySha256:hash(originalBytes),verifierSha256:hash(readFileSync(new URL(import.meta.url))),cases:203,companions:checked},null,2)+'\n')
console.log('203-case inventory and46 explicit smoothness companion requirements verified')
