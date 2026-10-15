import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFileSync,writeFileSync} from 'node:fs'
import {readSweepManifest} from './sweep-manifest-reader.mjs'
import {smoothCapGapCompanions} from './sweep-automatic-cap-contracts.mjs'

const [manifestPath,baselinePath,output]=process.argv.slice(2)
assert.ok(manifestPath&&baselinePath&&output)
const sha=b=>createHash('sha256').update(b).digest('hex')
const bytes=readFileSync(manifestPath),baseline=JSON.parse(readFileSync(baselinePath))
assert.equal(sha(bytes),baseline.sourceManifestSha256,'Historical source manifest differs from the independently captured157-case baseline')
const manifest=readSweepManifest(manifestPath,bytes)
const cases=[...smoothCapGapCompanions.keys()].map(file=>{
 const item=manifest.cases.find(c=>c.file===file)
 assert.ok(item);assert.equal(item.capFaces.length,2)
 assert.equal(item.wallSurfaces.length,item.faces-2)
 return {file,faces:item.faces,edges:item.edges,wallSurfaces:item.wallSurfaces,
  faceLoops:item.faceLoops,wallCoedges:item.wallCoedges,wallFaceReversed:item.wallFaceReversed,
  edgeCurves:item.edgeCurves.map(e=>e.curve)}
})
writeFileSync(output,JSON.stringify({schema:'sweep-original-annular-walls/1',sourceManifestSha256:sha(bytes),
 scope:'Exact historical retained walls, boundary edges and topology for three explicitly upgraded native cap fixtures; not historical proof outcomes',cases},null,2)+'\n')
console.log('Captured exact original walls/edges for three explicit native cap upgrades')
