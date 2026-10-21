"""Wait for this existing Cargo release; qualify exact raw bytes, never rebuild."""
import hashlib,json,os,re,shutil,subprocess,time
from pathlib import Path
stage=Path('/private/tmp/osv-sweep-current-wasm-20261015')
native=Path('/private/tmp/osv-sweep-auto-caps-native-20261014')
output=Path('/private/tmp/osv-sweep-current-final-packed-qualified-20261015')
build_log=Path('/private/tmp/osv-sweep-auto-caps-wasm-v2-build-20261014.log')
python='/Users/themoretheless/.codex/tmp/osv-step-oracle-20261013/bin/python'
raw=stage/'crates/target/wasm32-unknown-unknown/release/geometry_wasm.wasm'
digest=lambda path:hashlib.sha256(path.read_bytes()).hexdigest()
output.mkdir(parents=True,exist_ok=True)
receipt={'scope':'Current frozen full native source, brepS/bridgeZ/nurbsZ/predicatesS; exact packed ABI/reference/OCP, not full mode or UI/main delivery','phases':[],'passed':False,'state':'qualifying-packed'}
output.mkdir(parents=True,exist_ok=True)
def save():
    tmp=output/'receipt.json.writing'
    tmp.write_text(json.dumps(receipt,indent=2)+'\n')
    tmp.replace(output/'receipt.json')
raw=stage/'public/wasm/geometry-kernel.wasm'
receipt.update(rawSha256=digest(raw),rawByteLength=raw.stat().st_size)
assert digest(stage/'src/generated/geometry-kernels/kernel_bg.wasm')==receipt['rawSha256']
assert receipt['rawSha256'] in (stage/'src/generated/geometry-kernels/identity.ts').read_text()
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
run('station-abi',['node','--import','tsx','scripts/check-staged-auto-station-caps.mts',stage,output/'station'/'receipt.json',native])
for mode,flags in [('automatic',['--automatic-caps']),('explicit',[])]:
    target=output/mode
    run(mode+'-abi',['node','--import','tsx','scripts/check-staged-smooth-annular-kernel.mts',stage,target/'abi.json',native,*flags])
    data=json.loads((target/'abi.json').read_text())
    assert data['sha256']==receipt['rawSha256'] and data['automatic']==(mode=='automatic')
    run(mode+'-reference',[python,'scripts/reference-rational-boundary-volume.py',target])
    run(mode+'-occt',[python,'scripts/verify-sweep-step-occt.py',target])
target=output/'legacy'
run('legacy-abi',['node','--import','tsx','scripts/check-staged-annular-miter-kernel.mts',stage,target/'abi.json','--joint-material-step','--unbudgeted-source'])
data=json.loads((target/'abi.json').read_text())
assert data['artifact']['sha256']==receipt['rawSha256']
run('legacy-reference',[python,'scripts/reference-rational-boundary-volume.py',target])
run('legacy-occt',[python,'scripts/verify-sweep-step-occt.py',target])
receipt.update(state='complete',passed=True);save()
print('Final packed station + Auto3/explicit3/legacy11 ABI/reference/OCCT passed',flush=True)
