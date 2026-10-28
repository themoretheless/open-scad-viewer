from pathlib import Path
import hashlib,json,re,subprocess,os
ROOT=Path.cwd();OUT=Path(__file__).resolve().parent
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
files=['crates/brep-core/src/sweep_smoothness/complete.rs','crates/brep-core/src/sweep_smoothness/complete/reversal_tests.rs','crates/geometry-bridge/src/cad_sweep_smoothness/complete.rs','crates/geometry-bridge/src/cad_sweep_boundary/smooth_closed.rs','crates/geometry-bridge/tests/complete_multispan_seams.rs']
guard={p:sha(p)for p in files};(OUT/'proof-source-final-sha256.json').write_text(json.dumps(guard,indent=2)+'\n')
s=(OUT/'smoothness-native-r8.log').read_text();assert '16 passed; 0 failed' in s
binary=re.findall(r'Running unittests[^\n]+\(([^\n]+)\)',s)[-1]
record={'state':'running','passed':False,'binarySha256':sha(binary),'source':guard,'phases':[],'limitsIncreased':False,'focus':'No visible browser or native application interaction'}
def save(): (OUT/'proof-regressions.json').write_text(json.dumps(record,indent=2)+'\n')
save()
try:
    for name,selector,count in [('closed-original-retained-g2','sweep_boundary_owned::closed::smooth',14),('miter-cap-boundary','sweep_miter_owned::direct_source_regressions::',16)]:
        phase={'name':name,'selector':selector,'expectedTests':count,'state':'running'};record['phases'].append(phase);save();print(name+': started',flush=True)
        with (OUT/(name+'.log')).open('w') as f:
            result=subprocess.run([binary,selector,'--test-threads=2'],stdout=f,stderr=subprocess.STDOUT)
        text=(OUT/(name+'.log')).read_text();assert result.returncode==0 and f'{count} passed; 0 failed' in text,name
        phase.update(state='passed',logSha256=sha(OUT/(name+'.log')));save();print(name+': passed',flush=True)
    env={**os.environ,'CARGO_TARGET_DIR':'/private/tmp/osv-sweep-remaining-20261028','CARGO_BUILD_JOBS':'2'}
    name='complete-multispan-bridge-final';phase={'name':name,'expectedTests':1,'state':'running'};record['phases'].append(phase);save();print(name+': started',flush=True)
    with (OUT/(name+'.log')).open('w')as f:
        result=subprocess.run(['cargo','test','--manifest-path','crates/Cargo.toml','-p','geometry-bridge','--test','complete_multispan_seams','--locked','--offline','--','--test-threads=2'],env=env,stdout=f,stderr=subprocess.STDOUT)
    assert result.returncode==0 and '1 passed; 0 failed' in (OUT/(name+'.log')).read_text(),name
    phase.update(state='passed',logSha256=sha(OUT/(name+'.log')));save();print(name+': passed',flush=True)
    assert guard=={p:sha(p)for p in files},'Owned proof source changed'
    record.update(state='complete',passed=True,sourceGuardVerified=True);save()
except BaseException as e:
    record.update(state='failed',error=str(e));save();raise
