"""Replay complete native target coverage and optionally bind current crate sources."""
from pathlib import Path
import argparse,gzip,hashlib,json,re
parser=argparse.ArgumentParser();parser.add_argument('evidence',nargs='?',type=Path,default=Path(__file__).parent);parser.add_argument('--check-source',action='store_true');args=parser.parse_args();root=args.evidence
def read(name):
 p=root/name
 return p.read_text() if p.exists() else gzip.decompress((root/(name+'.gz')).read_bytes()).decode()
def load(name):return json.loads(read(name))
metadata=load('cargo-targets.json');by_source={t['src_path']:t['name'] for t in metadata['targets'] if t['kind']==['test']};assert len(by_source)==285
snapshots=[load('native266-full-'+m+'.source-inputs.json') for m in ['default','no-codec']];assert snapshots[0]==snapshots[1]
if args.check_source:
 actual=sorted(p for p in Path('crates').rglob('*') if p.is_file() and (p.suffix=='.rs' or p.name in ['Cargo.toml','Cargo.lock']))
 assert {str(p) for p in actual}==set(snapshots[0]);assert all(hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in snapshots[0].items())
result={}
for mode in ['default','no-codec']:
 name='native266-full-'+mode;state=load(name+'.state.json');assert state['status']=='finished' and state['exitCode']==0 and state['sourceChanged']==[]
 log=read(name+'.log');assert hashlib.sha256(log.encode()).hexdigest()==state['logSha256']
 sections=re.split(r'^\s*Doc-tests nurbs_core\s*$',log,flags=re.M);assert len(sections)==2
 parts=re.split(r'^\s*Running (.+?) \([^\n]+\)\s*$',sections[0],flags=re.M);targets={}
 for i in range(1,len(parts),2):
  label,body=parts[i:i+2];target='lib' if label=='unittests src/lib.rs' else by_source[label];assert target not in targets
  summaries=re.findall(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;',body,re.M);assert len(summaries)==1
  status,passed,failed,ignored,measured,filtered=summaries[0];assert status=='ok' and failed=='0' and filtered=='0';targets[target]={'passed':int(passed),'ignored':int(ignored)}
 assert set(targets)==set(['lib']+list(by_source.values()))
 ignored=re.findall(r'^test .* \.\.\. ignored.*$',sections[0],re.M);assert ignored==['test normal_alignment::tests::benchmark_closed_rational_restriction ... ignored, native restriction microbenchmark; no production speed claim']
 docs=re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',sections[1],re.M);assert docs==[('2','0','0')]
 result[mode]={'libraryAndIntegrationTargets':len(targets),'integrationTargets':len(by_source),'passed':sum(t['passed'] for t in targets.values()),'docTestsPassed':2,'failed':0,'ignoredMicrobenchmark':sum(t['ignored'] for t in targets.values()),'singleFullCommandExitCode':0}
result['currentSourceChecked']=args.check_source
print(json.dumps(result,indent=2))
