"""Publish only a qualified, source-matching staged kernel; preserve rollback bytes."""
from pathlib import Path
import json,hashlib,os
root=Path.cwd();out=Path(__file__).resolve().parent;stage=Path('/private/tmp/osv-sweep-main-shared-boundary-integer-norm-delivery-20261016');sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
q=json.loads((out/'current-main-finalnorm-qualification-v2.json').read_text());assert q['state']=='complete' and q['passed'] and q['deadlineClosureConfirmed'];assert all(v['status']=='passed' and v['durationMs']<30000 for v in q['originalFive'].values())
build=json.loads((out/'main-shared-boundary-integer-norm-staged-build.json').read_text());assert build['state']=='complete' and not build['sourceChangedDuringBuild'];native=json.loads((stage/'main-native-source-sha256.json').read_text())
script=(out/'build-main-shared-boundary-integer-norm-staged.py').read_text();exec(script[script.index('def sources():'):script.index('before=sources();')]);current=sources();drift=[x for x in set(native)|set(current) if native.get(x)!=current.get(x)];assert not drift,('Native source changed after qualification',drift)
ledger=json.loads((stage/'runtime-source-sha256.json').read_text());critical=[]
for name,digest in ledger.items():
 assert sha(stage/name)==digest,name
 if name.startswith(('src/','integrations/','tools/','scripts/')) or '/' not in name:
  if not (root/name).is_file() or sha(root/name)!=digest:critical.append(name)
assert not critical,('Current runtime source differs from qualified snapshot',critical)
assert sha(stage/'public/wasm/geometry-kernel.wasm')==q['packedSha256']
assert sha(root/'public/wasm/geometry-kernel.wasm')==build['mainWasmAfter'],'A peer published a different kernel; inspect rather than overwrite'
assert sha(root/'src/generated/geometry-kernels/kernel_bg.wasm')==build['mainWasmAfter']
files=['public/wasm/geometry-kernel.wasm']
for directory in ['src/generated/geometry-kernels','src/generated/wasm-brotli']:
 files += [str(p.relative_to(stage)) for p in (stage/directory).rglob('*') if p.is_file() and not p.name.startswith('.')]
backup=out/'actual-main-generated-before-finalnorm';assert not backup.exists();backup.mkdir();before={}
for name in files:
 destination=root/name;before[name]=sha(destination) if destination.exists() else None
 if destination.exists():p=backup/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(destination.read_bytes())
prepared={name:(stage/name).read_bytes() for name in files}
for name,previous in before.items():assert (sha(root/name) if (root/name).exists() else None)==previous,name
for name,data in prepared.items():
 destination=root/name;destination.parent.mkdir(parents=True,exist_ok=True);temporary=destination.with_name(destination.name+'.qualified-temp');assert not temporary.exists();temporary.write_bytes(data);os.replace(temporary,destination)
for name in files:assert sha(root/name)==sha(stage/name),name
assert q['packedSha256'] in (root/'src/generated/geometry-kernels/identity.ts').read_text()
r={'state':'complete','published':True,'packedSha256':q['packedSha256'],'qualificationReceiptSha256':sha(out/'current-main-finalnorm-qualification-v2.json'),'originalFive':q['originalFive'],'publicCurrentTests':q['actualCurrentPublicTests'],'nativeInputs':len(native),'mainSourceMatchedQualifiedStage':True,'files':{name:sha(root/name) for name in files},'backup':str(backup),'fullGoalComplete':False};(out/'current-main-finalnorm-publication.json').write_text(json.dumps(r,indent=2)+'\n');print('Qualified current-main kernel published with rollback bytes preserved.',flush=True)
