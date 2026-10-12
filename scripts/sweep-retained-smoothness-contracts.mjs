import assert from 'node:assert/strict'

// Requirements come from explicit source-fixture assertions in the STEP exporter,
// not observed certificate flags. Unlisted cases still receive the full scoped
// audit; absence here does not mean that G1/G2 is inapplicable.
export const retainedSmoothnessContracts=Object.freeze({
 'rush-circle-corrected-scale-twist.step':{profile:'G2'},
 'rush-circle-corrected-spatial.step':{profile:'G2'},
 'rush-circle-corrected-affine.step':{profile:'G2'},
 'rush-circle-corrected-oblique.step':{profile:'G2'},
 'rush-rational-curved-guide-frame-affine.step':{profile:'G2'},
 'rush-reconstructed-rational-profile.step':{profile:'G1',station:'G2'},
 'rush-reconstructed-periodic-profile.step':{profile:'G2',station:'G2'},
 'rush-g1-profile-frame-guide-affine.step':{profile:'G1',profileG2:false},
 'rush-station-g2-hollow.step':{station:'G2'},
 'rush-nonuniform-station-g2-hollow-miter.step':{station:'G2'},
 'rush-sharp-profile-nonuniform-station-g2-miter.step':{profile:'C0',station:'G2'},
 'rush-progressive-miter-reconstructed-conic-half.step':{profile:'G2',station:'G2'},
 'rush-progressive-miter-reconstructed-conic-one.step':{profile:'G2',station:'G2'},
 'rush-progressive-miter-reconstructed-conic-two.step':{profile:'G2',station:'G2'},
 'rush-progressive-miter-reconstructed-stations.step':{profile:'G2',station:'G2'},
 'rush-progressive-miter-reconstructed-sharp.step':{profile:'G2',station:'C0'},
 'rush-progressive-miter-reconstructed-guide-frame-affine-hollow.step':{profile:'G2',station:'G2'},
 'rush-progressive-miter-reconstructed-moving-frame-guide-affine-hollow.step':{profile:'G2',station:'G2'},
 'rush-closed-progressive-miter-reconstructed-frame-guide-affine-hollow.step':{profile:'G2',station:'C0'},
})

export function checkRetainedSmoothnessContract(item){
 const contract=retainedSmoothnessContracts[item.file]
 if(!contract)return null
 const p=item.nativeProfileSmoothness,label=`${item.file}: declared retained smoothness`
 if(contract.profile==='G2')assert.equal(p.profile.exactG1G2Certified,true,`${label} profile G2`)
 if(contract.profile==='G1')assert.equal(p.profileG1Certified,true,`${label} profile G1`)
 if(contract.profile==='C0'){
  assert.equal(p.profileG1Certified,false,`${label} sharp profile must not promote G1`)
  assert.equal(p.profile.exactG1G2Certified,false,`${label} sharp profile must not promote G2`)
 }
 if(contract.profileG2===false)assert.equal(p.profile.exactG1G2Certified,false,`${label} G1/G2 distinction`)
 if(contract.station==='G2')assert.equal(p.station.stationG2Certified,true,`${label} station G2`)
 if(contract.station==='C0'){
  assert.equal(p.station.stationG1Certified,false,`${label} sharp station must not promote G1`)
  assert.equal(p.station.stationG2Certified,false,`${label} sharp station must not promote G2`)
 }
 return contract
}

export function checkRetainedSmoothnessInventory(cases){
 for(const file of Object.keys(retainedSmoothnessContracts)){
  const count=cases.filter(item=>item.file===file).length
  assert.ok(count>0,`Missing declared retained smoothness fixture: ${file}`)
  assert.equal(count,1,`Duplicate declared retained smoothness fixture: ${file}`)
 }
}
