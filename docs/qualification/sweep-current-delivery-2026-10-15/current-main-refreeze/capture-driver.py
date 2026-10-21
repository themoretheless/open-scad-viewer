from pathlib import Path
import hashlib,json,os,shutil,time
main=Path('/Users/themoretheless/Documents/Sources/open-scad-viewer');stage=Path('/private/tmp/osv-sweep-current-main-source-20261015');stage.mkdir(exist_ok=False)
sha=lambda b:hashlib.sha256(b).hexdigest()
roots=['src','crates','scripts','tests','examples','tools','integrations','public','benchmarks','docs','.github']
plain=['package.json','package-lock.json','tsconfig.json','tsconfig.mcp.json','vite.config.ts','vitest.config.ts','rust-toolchain.toml','env.d.ts','index.html','LICENSE','README.md','THIRD_PARTY_NOTICES.md','.editorconfig','.gitattributes','.gitignore']
exclude={'target','node_modules','.git','dist','output','tmp','__pycache__','.cache','.rbench','.venv','credentials','credentials.toml'}
def paths():
 result=set(p for p in plain if (main/p).is_file())
 for name in roots:
  if not (main/name).exists():continue
  for folder,dirs,files in os.walk(main/name,followlinks=False):
   rel=Path(folder).relative_to(main)
   dirs[:]=[d for d in dirs if d not in exclude and not (rel==Path('docs') and d=='qualification') and not (rel==Path('src') and d=='generated') and not (Path(folder)/d).is_symlink()]
   for f in files:
    p=Path(folder)/f;local=str(p.relative_to(main))
    if p.is_symlink() or f in exclude or f=='.DS_Store':continue
    if local.startswith('public/wasm/'):continue
    result.add(local)
 return sorted(result)
inputs=paths();ledger={}
for name in inputs:
 p=main/name;data=p.read_bytes();target=stage/name;target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes(data);shutil.copystat(p,target);ledger[name]=sha(data)
# Required external compile-time test data are separate from runtime source.
fixtures={}
for name in ['docs/qualification/cad-roadmap-2026-09-28/parts-history/imported-flange.json','docs/qualification/cad-roadmap-2026-09-28/parts-history/cap-api-fixtures.json']:
 data=(main/name).read_bytes();target=stage/name;target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes(data);fixtures[name]=sha(data)
after=paths();drift=[p for p in inputs if sha((main/p).read_bytes())!=ledger[p]]
added=sorted(set(after)-set(inputs));removed=sorted(set(inputs)-set(after))
receipt={'state':'captured' if not(drift or added or removed) else 'refused-source-drift','passed':not(drift or added or removed),'runtimeInputCount':len(ledger),'sourceLedgerSha256':sha((json.dumps(ledger,indent=2)+'\n').encode()),'changedDuringCapture':drift,'addedDuringCapture':added,'removedDuringCapture':removed,'testOnlyExternalInputs':fixtures,'mainPublicWasmObservedSha256':sha((main/'public/wasm/geometry-kernel.wasm').read_bytes()),'scope':'Fresh actual current main source including untracked new Rust/app files and retained Bernstein optimization. Generated files/old artifacts are unqualified seeds only; no old result reuse or source overwrite.','mainModified':False,'artifactQualified':False,'fullGoalComplete':False}
(stage/'frozen-current-main-source-sha256.json').write_text(json.dumps(ledger,indent=2)+'\n');(stage/'source-capture-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt,indent=2),flush=True)
assert receipt['passed'],'Source changed during capture; keep failed snapshot, do not build it'
assert ledger['crates/nurbs-core/src/distance/surface_distance/bezier_restriction.rs']=='418a891aa6995aedbdd38248c62442f616135f67122129fbb749e247afe6a301'
assert 'const geometryChunkBudget = 3_970_000' in (stage/'scripts/verify-dist.mjs').read_text()
assert 'const totalBudget = 8_080_000' in (stage/'scripts/verify-dist.mjs').read_text()
