"""Supplemental source contracts for native G2 companion reports; no geometry in Python."""
from pathlib import Path
import hashlib,json,math,re,sys
MODES=['affine','affine-hollow','varying-affine-hollow','periodic-affine','periodic-varying-affine-hollow','parameter-varying-affine-hollow','parameter-periodic-varying-affine-hollow','periodic-varying-affine-three-holes','parameter-periodic-varying-affine-three-holes','rational-nonuniform-affine-hollow','antipodal-affine-hollow']
def sha(path):
 h=hashlib.sha256()
 with Path(path).open('rb') as f:
  for chunk in iter(lambda:f.read(1024*1024),b''):h.update(chunk)
 return h.hexdigest()
def require(ok,why):
 if not ok:raise ValueError(why)
def number(x):return type(x) in (int,float) and math.isfinite(x)
def expected_contracts(stage):
 text=(stage/'scripts/spatial-rmf-profile-g2-sources.mjs').read_text()
 sources=re.findall(r'"([^"\n]+[.]r)"',text);require(len(sources)==11,'Spatial source inventory must contain exactly eleven companions')
 result={}
 for closed in [False,True]:
  for bits in range(8):
   mode={'closed':closed,'affine':bool(bits&1),'authored':bool(bits&2),'guided':bool(bits&4)}
   laws='-'.join(n for bit,n in [(1,'affine'),(2,'authored'),(4,'guide')] if bits&bit) or 'plain'
   name=f"miter-laws-{'closed' if closed else 'open'}-{laws}-profile-g2"
   result[name+'.step']={'source':name+'.r','budget':10 if closed else 2,'mode':mode,'spatial':False}
 for mode,source in zip(MODES,sources):
  result[f'closed-original-spatial-rmf-holonomy-{mode}-tube-profile-g2.step']={'source':'examples/rush/'+source,'budget':1 if 'antipodal' in mode else .4 if 'rational-nonuniform' in mode else .25,'spatial':True}
 return result

def project(item):
 q={k:item[k] for k in ['file','sourceFile','sourceSha256','requestedSmoothness','explicitCircleCorrection','lawMatrixMode','boundaryError'] if k in item}
 p=item['nativeProfileSmoothness']
 q['nativeProfileSmoothness']={'profileG1Certified':p['profileG1Certified'],'profile':{'exactG1G2Certified':p['profile']['exactG1G2Certified']},'station':{k:p['station'][k] for k in ['stationG1Certified','stationG2Certified']},'stationContinuity':p['stationContinuity'],'fullBoundarySmoothnessCertified':p['fullBoundarySmoothnessCertified']}
 if 'boundaryError' in item:
  owner=item['construction'][item['boundaryError']['nodeId']]['bodyBoundaryEvidence']
  q['owner']={k:owner[k] for k in ['closedPath','profileCorrectionErrorUpper']}
  q['owner']['retainedWalls']={k:owner['retainedWalls'][k] for k in ['certified','faceCoverageCertified','coefficientFamilyCertified']}
 else:
  q['owner']=item['construction']['boundaryCertificate']
  q['constructionFlags']={k:item['construction'][k] for k in ['affineLawsApplied','authoredFramesApplied','orientationGuideApplied']}
 return q

def validate_case(q,contract,source_bytes,original_bytes):
 label=q['file'];fail=lambda ok,msg:require(ok,label+': '+msg)
 fail(q['sourceFile']==contract['source'],'wrong declared source')
 fail(source_bytes==original_bytes,'captured source differs from unchanged input')
 fail(hashlib.sha256(source_bytes).hexdigest()==q['sourceSha256'],'source hash differs')
 fail(re.search(rb'circle_correction_tolerance: 0[.]000000001mm',source_bytes) is not None,'original circle correction tolerance changed')
 fail(q['requestedSmoothness']=={'profile':'G2','station':'C0'},'wrong requested smoothness')
 fail(q['explicitCircleCorrection'] is True,'correction is not explicit')
 p=q['nativeProfileSmoothness']
 fail(p['profileG1Certified'] is True and p['profile']['exactG1G2Certified'] is True,'actual retained profile is not G2')
 fail(p['station']['stationG1Certified'] is False and p['station']['stationG2Certified'] is False and p['stationContinuity']=='C0','sharp station was promoted')
 fail(p['fullBoundarySmoothnessCertified'] is False,'profile certificate was promoted to whole boundary')
 b=q['boundaryError'] if contract['spatial'] else q['owner']
 fail(number(b['budget']) and b['budget']==contract['budget'],'original deviation budget changed')
 fail(b['continuousBound'] is True and b['withinBudget'] is True,'whole original error is unproved')
 fail(number(b['errorUpper']) and 0<=b['errorUpper']<=b['budget'],'invalid original error bound')
 owner=q['owner']
 if contract['spatial']:
  fail(owner['closedPath'] is True,'spatial closed source was opened')
  error=owner['profileCorrectionErrorUpper']
  fail(number(error) and 0<error<=1e-9,'missing or out-of-budget correction contribution')
  fail(all(owner['retainedWalls'][key] is True for key in ['certified','faceCoverageCertified','coefficientFamilyCertified']),'unproved retained coefficient family')
  fail(q['boundaryError']['profileG2Certified'] is True and q['boundaryError']['stationContinuity']=='C0','viewport smoothness disagrees')
 else:
  mode=contract['mode']
  fail(set(q['lawMatrixMode'])==set(mode) and all(type(q['lawMatrixMode'][k]) is bool and q['lawMatrixMode'][k]==v for k,v in mode.items()),'joint-law combination differs')
  for flag,key in [('affineLawsApplied','affine'),('authoredFramesApplied','authored'),('orientationGuideApplied','guided')]:fail(q['constructionFlags'][flag] is mode[key],'native law provenance differs')
  fail(owner['scope']=='boundary-set-hausdorff' and owner['method']=='retained-sweep-boundary-union' and owner['reason'] is None,'wrong boundary certificate owner')
  fail(owner['closed'] is mode['closed'],'boundary closure differs')
  fail(number(owner['wallErrorUpper']) and 0<=owner['wallErrorUpper']<=owner['errorUpper'],'wall exceeds complete boundary bound')
  caps=owner['filledCapErrorUpper']
  if mode['closed']:fail(caps is None,'closed boundary fabricated filled caps')
  else:fail(type(caps) is list and len(caps)==2 and all(number(e) and 0<=e<=owner['errorUpper'] for e in caps),'missing complete two-cap contribution')
 return {'file':label,'sourceSha256':q['sourceSha256'],'budget':b['budget'],'errorUpper':b['errorUpper'],'scope':'spatial-profile-G2' if contract['spatial'] else 'joint-law-profile-G2','station':'C0'}

def check_inventory(cases,contracts):
 names=[c['file'] for c in cases]
 require(len(names)==len(set(names))==27 and set(names)==set(contracts),'Incomplete or duplicated27-companion inventory')

def main(canonical,stage,expected_wasm,output):
 parent_path=canonical/'receipt.json';parent=json.loads(parent_path.read_text())
 require(parent['state']=='complete' and parent['passed'] is True,'Fresh canonical STEP/UI parent is incomplete')
 require(parent['geometryWasmSha256']==expected_wasm,'Canonical artifact changed')
 manifest_path=canonical/'step207/manifest.json';manifest_sha=sha(manifest_path)
 require(parent['finalManifestSha256']==manifest_sha,'Canonical manifest changed')
 manifest=json.loads(manifest_path.read_text());require(len(manifest['cases'])==207,'Not the complete current matrix')
 require(manifest['artifactProvenance']['geometryWasmSha256']==expected_wasm,'Captured native reports belong to another WASM')
 oracle_path=canonical/'step207/opencascade-sweep.json';oracle=json.loads(oracle_path.read_text())
 require(oracle['passed'] is True and oracle['manifestSha256']==manifest_sha,'Independent STEP oracle is stale')
 require(oracle['artifactProvenance']==manifest['artifactProvenance'],'Independent/native provenance differs')
 imported={c['file']:c for c in oracle['cases']};require(len(imported)==207,'Incomplete independent import inventory')
 contracts=expected_contracts(stage);selected=[c for c in manifest['cases'] if c['file'] in contracts];check_inventory(selected,contracts)
 original_parent_sha=sha(parent_path);checked=[]
 for item in selected:
  contract=contracts[item['file']];source=canonical/'step207'/item['sourceFile']
  original=stage/contract['source'] if contract['spatial'] else stage/'qualification-law-sources'/contract['source']
  step_path=canonical/'step207'/item['file'];step_sha=sha(step_path)
  require(imported[item['file']]['passed'] is True and step_sha==item['sha256']==imported[item['file']]['stepSha256'],'Independent STEP bytes changed')
  checked.append(validate_case(project(item),contract,source.read_bytes(),original.read_bytes()))
 require(sha(parent_path)==original_parent_sha and sha(manifest_path)==manifest_sha,'Captured parent changed during audit')
 result={'schema':'sweep-companion27-source-contracts/1','passed':True,'geometryWasmSha256':expected_wasm,'canonicalReceiptSha256':original_parent_sha,'manifestSha256':manifest_sha,'oracleSha256':sha(oracle_path),'checkerSha256':sha(Path(__file__)),'sourceInventorySha256':sha(stage/'scripts/spatial-rmf-profile-g2-sources.mjs'),'cases':checked,'scope':'Supplemental native-report/source/deviation/correction contracts for all16joint-law and11spatial G2 companions. No geometry recomputation, new STEP/UI run or full objective completion.','fullGoalComplete':False}
 output.write_text(json.dumps(result,indent=2)+'\n')
if __name__=='__main__':main(Path(sys.argv[1]),Path(sys.argv[2]),sys.argv[3],Path(sys.argv[4]))
