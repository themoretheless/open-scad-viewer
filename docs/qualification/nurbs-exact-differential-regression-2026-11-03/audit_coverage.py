"""Replay archived target coverage; --check-source additionally binds current crates."""
from pathlib import Path
import argparse,gzip,hashlib,json,re
parser=argparse.ArgumentParser();parser.add_argument('evidence',nargs='?',type=Path,default=Path(__file__).parent);parser.add_argument('--check-source',action='store_true');parser.add_argument('--output',type=Path);args=parser.parse_args();root=args.evidence
def text(name):
 p=root/name
 return p.read_text() if p.exists() else gzip.decompress((root/(name+'.gz')).read_bytes()).decode()
def load(name):return json.loads(text(name))
def targets(name):
 parts=re.split(r'^\s*Running (.+?) \([^\n]+\)\s*$',text(name),flags=re.M);out={}
 for i in range(1,len(parts),2):
  label,body=parts[i:i+2];name='lib' if label=='unittests src/lib.rs' else Path(label).stem
  summaries=re.findall(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;',body,re.M);assert len(summaries)==1,(name,summaries)
  status,p,f,ignored,measured,filtered=summaries[0];assert name not in out;out[name]={'status':status,'passed':int(p),'failed':int(f),'ignored':int(ignored),'filtered':int(filtered)}
 return out
plan=load('native263-resume-plan.json');metadata=load('cargo-targets.json');expected=sorted(t['name'] for t in metadata['targets'] if t['kind']==['test']);assert expected==plan['expectedTargets'] and len(expected)==283
before=load('native259-current-full-source-inputs.json');after=load('native263-resume-default.source-inputs.json');assert after==load('native263-resume-no-codec.source-inputs.json');assert set(before)==set(after)
changed=[p for p,h in before.items() if after[p]!=h];assert changed==['crates/nurbs-core/tests/surface_correspondence.rs']
if args.check_source:assert all(hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in after.items()),'Current source differs from qualified snapshot'
result={}
for mode in ['default','no-codec']:
 initial=targets('native259-current-full-'+mode+'-opt1.log');resume=targets('native263-resume-'+mode+'.log');state=load('native263-resume-'+mode+'.state.json');assert state['exitCode']==0 and state['sourceChanged']==[]
 assert set(initial)==set(['lib']+plan['initialRunTargets']);assert set(resume)==set(plan['resumeTargets']);assert [n for n,r in initial.items() if r['failed']]==['surface_correspondence'];assert initial['surface_correspondence']['failed']==1
 combined={**initial,**resume};assert set(combined)==set(['lib']+expected);assert all(r['status']=='ok' and r['failed']==0 and r['filtered']==0 for r in combined.values());assert sum(r['ignored'] for r in combined.values())==1
 ignored=re.findall(r'^test .* \.\.\. ignored.*$',text('native259-current-full-'+mode+'-opt1.log'),re.M);assert ignored==['test normal_alignment::tests::benchmark_closed_rational_restriction ... ignored, native restriction microbenchmark; no production speed claim']
 docs=re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',text('native263-doc-'+mode+'.log'),re.M);assert docs==[('2','0','0')]
 result[mode]={'libraryAndIntegrationTargets':len(combined),'passed':sum(r['passed'] for r in combined.values()),'failedAfterRepair':0,'ignoredMicrobenchmark':1,'doctestsPassed':2}
result['sourceAudit']={'onlyChangedSource':changed,'currentSourceChecked':args.check_source,'scope':'Complete target union from retained passed prefix and repaired/suffix runs. Initial failed test retained, replacement verified. Not a single-command exit0 full-run claim.'}
output=json.dumps(result,indent=2)+'\n'
if args.output:args.output.write_text(output)
print(output,end='')
