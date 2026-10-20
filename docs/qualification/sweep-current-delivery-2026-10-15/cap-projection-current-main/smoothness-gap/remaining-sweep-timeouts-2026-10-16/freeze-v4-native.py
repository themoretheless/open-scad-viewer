from pathlib import Path
import json,hashlib,subprocess
root=Path.cwd();out=Path(__file__).resolve().parent;stage=Path('/private/tmp/osv-sweep-frozen-current-v4-source-20261016');sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();script=(out/'build-main-final-source-refresh-v3-staged.py').read_text();exec(script[script.index('def sources():'):script.index('before=sources();')]);before=sources()
subprocess.run(['python3',str(out/'capture-frozen-v4-runtime.py')],check=True)
after=sources();assert before==after,'Source changed during capture; no build started'
for name,digest in after.items():assert sha(stage/name)==digest,name
(stage/'main-native-source-sha256.json').write_text(json.dumps(after,indent=2)+'\n')
(out/'frozen-v4-capture.json').write_text(json.dumps({'state':'complete','nativeSourceFiles':len(after),'sourceChangedDuringCapture':False,'nativeLedgerSha256':sha(stage/'main-native-source-sha256.json'),'runtimeLedgerSha256':sha(stage/'runtime-source-sha256.json'),'mainWasmModified':False,'fullGoalComplete':False},indent=2)+'\n');print('Whole current source frozen coherently:',len(after),'nativeinputs',flush=True)
