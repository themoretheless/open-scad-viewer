"""Reuse the qualified Rust basis audit on all newly source-bound surfaces."""
from pathlib import Path
import ast,hashlib,json,subprocess
OUT=Path(__file__).resolve().parent;ROOT=OUT.parents[5];STAGE=Path('/private/tmp/osv-sweep-common-source-v11-20261018')
DEST=OUT/'v11-original-source-interiors37-20261018'
sha=lambda p:hashlib.file_digest(Path(p).open('rb'),'sha256').hexdigest()
assert not DEST.exists(),'Preserve prior results'
followup=OUT/'v11-original-source-step37-occt-20261018.json';gate=json.loads(followup.read_text());assert gate['state']=='complete' and gate['passed'] and gate['cases']==37
manifest_path=Path(gate['manifest']);assert sha(manifest_path)==gate['manifestSha256'];manifest=json.loads(manifest_path.read_text());assert len(manifest['cases'])==37
producer=STAGE/'crates/target/release/examples/certify_retained_surface_interiors';source=STAGE/'crates/brep-core/examples/certify_retained_surface_interiors.rs'
previous=OUT/'frozen-v11r1-common-source-20261018-independent-step-ui-final/retained-interiors/result.json';qualified=json.loads(previous.read_text());assert qualified['passed'];assert sha(producer)==qualified['producerSha256'];assert sha(source)==qualified['producerSourceSha256']
# Reuse the exact existing bounded packet builder, without executing its CLI.
helper=STAGE/'scripts/qualify-retained-surface-interiors.py';text=helper.read_text();tree=ast.parse(text);node=next(n for n in tree.body if isinstance(n,ast.FunctionDef)and n.name=='bounded_batches');code=ast.get_source_segment(text,node);namespace={'json':json};exec(compile(code,str(helper),'exec'),namespace);batches=namespace['bounded_batches']
DEST.mkdir();requests=[]
for case in manifest['cases']:
 owners=[face for face in range(case['faces'])if face not in case['capFaces']];assert len(owners)==len(case['wallSurfaces'])
 assert case['nativeVolume']['solidGeometryCertified'] and case['nativeVolume']['allFacesInjective']
 requests.append({'file':case['file'],'surfaces':[{'face':face,'surface':s}for face,s in zip(owners,case['wallSurfaces'])]})
result={'schema':'sweep-v11-source-interior37/1','passed':False,'fullGoalComplete':False,'manifestSha256':sha(manifest_path),'geometryWasmSha256':gate['geometryWasmSha256'],'stepGateSha256':sha(followup),'producerBinarySha256':sha(producer),'producerSourceSha256':sha(source),'batchHelperSha256':sha(helper),'scriptSha256':sha(__file__),'batches':[],'cases':[],'scope':'Rust basis-C2 on actual rectangular interiors, including radial caps. Existing exact planar-cap mask retained. This primitive does not certify immersion, owned seams, periodic closure, source error or material.'}
for index,(group,encoded)in enumerate(batches({'maxInputItemsPerCase':1000000,'manifestSha256':sha(manifest_path),'geometryWasmSha256':gate['geometryWasmSha256']},requests,31*1024*1024)):
 request=DEST/f'request-{index}.json';output=DEST/f'native-{index}.json';request.write_bytes(encoded)
 subprocess.run([str(producer),str(request),str(output)],check=True)
 native=json.loads(output.read_text());assert native['passed'] and native['requestSha256']==sha(request) and native['producerSha256']==sha(source)
 assert native['continuousBound'] is False and native['solidCertified'] is False
 assert [c['file']for c in native['cases']]==[c['file']for c in group]
 for expected,proof in zip(group,native['cases']):
  assert proof['complete'] and proof['reason']is None and proof['unvisitedFaces']==0 and 0<proof['work']<=proof['maxInputItems']==1000000
  assert [f['face']for f in proof['faces']]==[f['face']for f in expected['surfaces']]
  for f in proof['faces']:
   assert f['parametricC0Certified'] and f['parametricC1Certified'] and f['parametricC2Certified']
   assert f['geometricG1G2Certified']is False and f['surfaceRegularityCertified']is False and f['periodicSeamsCertified']is False
  result['cases'].append(proof)
 result['batches'].append({'requestSha256':sha(request),'outputSha256':sha(output)})
assert len(result['cases'])==37 and sha(manifest_path)==result['manifestSha256'] and sha(producer)==result['producerBinarySha256']
result.update(passed=True,surfaces=sum(len(c['faces'])for c in result['cases']))
(DEST/'result.json').write_text(json.dumps(result,indent=2)+'\n');print('Native37case interior audit passed:',result['surfaces'],'surfaces; geometric/whole-boundary claims remain separate.')
