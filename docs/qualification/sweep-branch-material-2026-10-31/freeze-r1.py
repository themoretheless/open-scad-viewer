from pathlib import Path
import hashlib,json,shutil,tarfile
ROOT=Path(__file__).resolve().parents[3]
OUT=Path(__file__).resolve().parent
BASE=Path('/private/tmp/osv-sweep-window-crossings-20261031-r1')
STAGE=Path('/private/tmp/osv-sweep-branch-material-20261031-r1')
assert not STAGE.exists()
old=json.loads((ROOT/'docs/qualification/sweep-window-crossings-2026-10-31/source-r1.json').read_text())
owned=json.loads((OUT/'owned-paths.json').read_text())
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
for n,h in old.items():
 assert sha(BASE/n)==h, n
 (STAGE/n).parent.mkdir(parents=True,exist_ok=True)
 shutil.copy2(BASE/n,STAGE/n)
for n in owned:
 (STAGE/n).parent.mkdir(parents=True,exist_ok=True)
 shutil.copy2(ROOT/n,STAGE/n)
ledger={n:sha(STAGE/n)for n in sorted(set(old)|set(owned))}
(OUT/'source-r1.json').write_text(json.dumps(ledger,indent=2)+'\n')
(OUT/'owned-r1.json').write_text(json.dumps({n:ledger[n]for n in owned},indent=2)+'\n')
with tarfile.open(OUT/'inputs-r1.tar.gz','w:gz',compresslevel=3)as t:
 for n in ledger:t.add(STAGE/n,arcname=n,recursive=False)
print(str(STAGE),len(ledger),len(owned),flush=True)
