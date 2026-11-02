import {test} from 'node:test'
import assert from 'node:assert/strict'
import {readFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {selectGeometryDeliveryBudget} from '../scripts/geometry-delivery-budgets.mjs'

test('preserves the exact runtime v6 delivery policy',()=>{
 assert.deepEqual(selectGeometryDeliveryBudget('2ff06f662379e92193ff80849ce03bac9ab28c82d00223ff9a48e4d0307ca058'),
  {version:6,geometryChunkBytes:4680000,totalAssetBytes:9090000})
})
test('pins the measured runtime v7 assets with bounded delivery margins',()=>{
 const profile=selectGeometryDeliveryBudget('91521ffbc32570cddc99b4555dcdd3f55fdaa902a806fccc215bf7afefab129f')
 assert.equal(profile.version,7)
 assert.ok(profile.geometryChunkBytes>=5360574&&profile.geometryChunkBytes-5360574<=10000)
 assert.ok(profile.totalAssetBytes>=10066845&&profile.totalAssetBytes-10066845<=15000)
 assert.ok(Object.isFrozen(profile))
 assert.throws(()=>{profile.geometryChunkBytes=Infinity},TypeError)
})
test('binds the measured v8 artifact to the unchanged v7 size limits',()=>{
 assert.deepEqual(selectGeometryDeliveryBudget('442d2703d243fe29ed8434dcbce04e7cf06c2be36be149af07b51b00e45160f6'),
  {version:8,geometryChunkBytes:5370000,totalAssetBytes:10080000})
})
test('refuses unreviewed or malformed artifacts instead of borrowing a larger cap',()=>{
 for(const hash of ['0'.repeat(64),'__proto__','constructor','',null,undefined])
  assert.throws(()=>selectGeometryDeliveryBudget(hash),/no reviewed delivery budget/)
})

test('admits only the archived390 bytes within unchanged caps and measured margins',()=>{
 const review=JSON.parse(readFileSync(new URL('../docs/qualification/geometry-delivery-390fb9-2026-11-02/review.json',import.meta.url),'utf8'))
 const bytes=readFileSync(new URL('../'+review.artifact,import.meta.url))
 assert.equal(createHash('sha256').update(bytes).digest('hex'),review.wasmSha256)
 assert.equal(bytes.length,review.artifactBytes)
 const profile=selectGeometryDeliveryBudget(review.wasmSha256)
 const old=selectGeometryDeliveryBudget('442d2703d243fe29ed8434dcbce04e7cf06c2be36be149af07b51b00e45160f6')
 assert.equal(profile.version,9)
 for(const key of ['geometryChunkBytes','totalAssetBytes']) {
  assert.equal(profile[key],old[key])
  assert.ok(review.measurement[key]<=profile[key])
 }
 const changed=Buffer.from(bytes);changed[changed.length-1]^=1
 assert.throws(()=>selectGeometryDeliveryBudget(createHash('sha256').update(changed).digest('hex')),/no reviewed delivery budget/)
})

test('binds the reviewed baf artifact to unchanged caps after sept payload verification',()=>{
 const review=JSON.parse(readFileSync(new URL('../docs/qualification/geometry-delivery-baf58e-2026-11-02/review.json',import.meta.url),'utf8'))
 const bytes=readFileSync(new URL('../'+review.artifact,import.meta.url))
 assert.equal(createHash('sha256').update(bytes).digest('hex'),review.wasmSha256)
 const profile=selectGeometryDeliveryBudget(review.wasmSha256)
 const prior=selectGeometryDeliveryBudget('390fb9f1e731a8de56228e7bcb6454eae96f28f06df621a9d1ca8f489b2a0201')
 assert.equal(profile.version,10)
 assert.equal(profile.geometryChunkBytes,prior.geometryChunkBytes)
 assert.equal(profile.totalAssetBytes,prior.totalAssetBytes)
 assert.ok(review.measurement<=profile.geometryChunkBytes)
 assert.ok(review.privateViteAssetBytes<=profile.totalAssetBytes)
 const altered=Buffer.from(bytes);altered[altered.length-1]^=1
 assert.throws(()=>selectGeometryDeliveryBudget(createHash('sha256').update(altered).digest('hex')),/no reviewed delivery budget/)
})
