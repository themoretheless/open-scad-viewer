"""Copy complete current runtime inputs without touching build-owned generated assets."""
from pathlib import Path
import subprocess,hashlib,json,os
root=Path.cwd();out=Path(__file__).resolve().parent;stage=Path('/private/tmp/osv-sweep-main-shared-boundary-bernstein-delivery-20261016');assert stage.exists()
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();assert not (stage/'runtime-source-sha256.json').exists()
for name in ['src','tests','examples','integrations','tools','scripts','public','crates','docs']:
 if not (root/name).exists():continue
 excludes=[]
 if name=='src':excludes=['--exclude=generated/geometry-kernels/','--exclude=generated/wasm-brotli/']
 if name=='public':excludes=['--exclude=wasm/geometry-kernel.wasm']
 if name=='crates':excludes=['--exclude=target/']
 if name=='docs':excludes=['--exclude=qualification/']
 (stage/name).mkdir(exist_ok=True)
 subprocess.run(['rsync','-a',*excludes,str(root/name)+'/',str(stage/name)+'/'],check=True)
for p in root.iterdir():
 if p.is_file() and (p.suffix in ['.json','.toml','.ts','.js','.mjs','.html','.css'] or p.name in ['.npmrc','.node-version','.nvmrc']):
  subprocess.run(['cp','-c',str(p),str(stage/p.name)],check=True)
old=json.loads(Path('/private/tmp/osv-sweep-main-trim-probes-delivery-20261016/runtime-source-sha256.json').read_text())
for name in old:
 if not name.startswith('docs/qualification/'):continue
 source=root/name;assert source.is_file(),name;destination=stage/name;destination.parent.mkdir(parents=True,exist_ok=True);subprocess.run(['cp','-c',str(source),str(destination)],check=True)
if not (stage/'node_modules').exists():os.symlink(root/'node_modules',stage/'node_modules')
ledger={}
for name in ['src','tests','examples','integrations','tools','scripts','public','crates','docs']:
 for p in (stage/name).rglob('*'):
  if not p.is_file():continue
  rel=str(p.relative_to(stage))
  if rel.startswith(('src/generated/geometry-kernels/','src/generated/wasm-brotli/')) or rel=='public/wasm/geometry-kernel.wasm':continue
  ledger[rel]=sha(p)
for p in stage.iterdir():
 if p.is_file() and (p.suffix in ['.json','.toml','.ts','.js','.mjs','.html','.css'] or p.name in ['.npmrc','.node-version','.nvmrc']) and p.name not in ['runtime-source-sha256.json','main-native-source-sha256.json']:
  ledger[p.name]=sha(p)
(stage/'runtime-source-sha256.json').write_text(json.dumps(ledger,indent=2)+'\n')
print('Complete protected runtime snapshot:',len(ledger),'files',flush=True)
