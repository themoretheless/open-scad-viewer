"""Wait for this existing Cargo release; qualify exact raw bytes, never rebuild."""
import hashlib,json,os,re,shutil,subprocess,time
from pathlib import Path
stage=Path('/private/tmp/osv-sweep-auto-caps-wasm-v2-20261014')
native=Path('/private/tmp/osv-sweep-auto-caps-native-20261014')
output=Path('/private/tmp/osv-sweep-auto-caps-v2-raw-20261015')
build_log=Path('/private/tmp/osv-sweep-auto-caps-wasm-v2-build-20261014.log')
python='/Users/themoretheless/.codex/tmp/osv-step-oracle-20261013/bin/python'
raw=stage/'crates/target/wasm32-unknown-unknown/release/geometry_wasm.wasm'
digest=lambda path:hashlib.sha256(path.read_bytes()).hexdigest()
output.mkdir(parents=True,exist_ok=True)
receipt={'scope':'Exact frozen-source raw WASM ABI and independent STEP; not optimized/Rush/UI/main delivery','buildLog':str(build_log),'phases':[],'passed':False,'state':'waiting-existing-cargo-release','rawBeforeReleaseSha256':digest(raw)}
def save():
    tmp=output/'receipt.json.writing'
    tmp.write_text(json.dumps(receipt,indent=2)+'\n')
    tmp.replace(output/'receipt.json')
save()
print('Waiting for the already-running geometry Cargo release (no build/restart)',flush=True)
while True:
    log=build_log.read_text()
    marker=log.rfind('Compiling geometry-wasm ')
    assert marker>=0,'Expected live geometry Cargo phase is missing'
    phase=log[marker:]
    if re.search(r'Finished .*release.*target\(s\)',phase):break
    if re.search(r'^error:',phase,re.M):raise RuntimeError('The existing Cargo build failed; no restart')
    time.sleep(3)
manifest=json.loads((stage/'native-and-app-source-manifest.json').read_text())
count=0
for name,expected in manifest['files'].items():
    if not name.startswith('crates/') or not name.endswith(('.rs','.toml','.lock')):continue
    if name=='crates/Cargo.toml':expected=manifest['privateCargoSha256']
    assert digest(stage/name)==expected,'Frozen native source drift: '+name
    count+=1
receipt.update(state='qualifying-raw',capturedNativeFiles=count,rawSha256=digest(raw),rawByteLength=raw.stat().st_size)
assert receipt['rawSha256']!=receipt['rawBeforeReleaseSha256'],'Cargo did not publish the new raw artifact'
shutil.copy2(raw,output/'qualified-input-raw.wasm')
save()
env={**os.environ,'PYTHONDONTWRITEBYTECODE':'1'}
def run(name,args):
    entry={'name':name,'argv':list(map(str,args)),'state':'running'}
    receipt['phases'].append(entry);save()
    print(name+': started',flush=True)
    start=time.monotonic()
    with (output/(name+'.log')).open('w') as log:
        result=subprocess.run(list(map(str,args)),cwd=stage,env=env,stdout=log,stderr=subprocess.STDOUT)
    entry.update(state='passed' if result.returncode==0 else 'failed',returncode=result.returncode,seconds=time.monotonic()-start)
    assert digest(raw)==receipt['rawSha256'],'Cargo raw artifact changed during qualification'
    save()
    print(name+': '+entry['state'],flush=True)
    if result.returncode:raise RuntimeError(name+' failed; see exact retained log, no retry')
for mode,flags in [('automatic',['--automatic-caps']),('explicit',[])]:
    target=output/mode
    run(mode+'-abi',['node','--import','tsx','scripts/check-staged-smooth-annular-kernel.mts',stage,target/'abi.json',native,'--raw-built',*flags])
    data=json.loads((target/'abi.json').read_text())
    assert data['sha256']==receipt['rawSha256'] and data['automatic']==(mode=='automatic')
    run(mode+'-reference',[python,'scripts/reference-rational-boundary-volume.py',target])
    run(mode+'-occt',[python,'scripts/verify-sweep-step-occt.py',target])
target=output/'legacy'
run('legacy-abi',['node','--import','tsx','scripts/check-staged-annular-miter-kernel.mts',stage,target/'abi.json','--raw-built','--joint-material-step','--unbudgeted-source'])
data=json.loads((target/'abi.json').read_text())
assert data['artifact']['sha256']==receipt['rawSha256']
run('legacy-reference',[python,'scripts/reference-rational-boundary-volume.py',target])
run('legacy-occt',[python,'scripts/verify-sweep-step-occt.py',target])
receipt.update(state='complete',passed=True);save()
print('Exact raw Auto3 + explicit3 + legacy11 ABI/reference/OCCT passed',flush=True)
