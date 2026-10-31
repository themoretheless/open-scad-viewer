import {test} from 'node:test'
import assert from 'node:assert/strict'
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
test('refuses unreviewed or malformed artifacts instead of borrowing a larger cap',()=>{
 for(const hash of ['0'.repeat(64),'__proto__','constructor','',null,undefined])
  assert.throws(()=>selectGeometryDeliveryBudget(hash),/no reviewed delivery budget/)
})
