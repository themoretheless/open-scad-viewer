from pathlib import Path
import hashlib,json,tarfile,shutil
ROOT=Path(__file__).resolve().parents[3]
OUT=Path(__file__).resolve().parent
OLD=ROOT/'docs/qualification/sweep-algebraic-graph-2026-10-31'
STAGE=Path('/private/tmp/osv-sweep-branch-moving-20261101-r7')
assert not STAGE.exists()
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
archive=OLD/'qualified-source-r4.tar.gz'
assert sha(archive)==json.loads((OLD/'result.json').read_text())['sourceArchiveSha256']
ledger=json.loads((OLD/'source-r4.json').read_text())
owned=list(json.loads((OUT/'owned-r7.json').read_text()))
borrowed=list(json.loads((OUT/'borrowed-r7.json').read_text()))
with tarfile.open(archive)as tar:
 for n,h in ledger.items():
  assert not Path(n).is_absolute() and '..' not in Path(n).parts
  data=tar.extractfile(n).read();assert hashlib.sha256(data).hexdigest()==h,n
  (STAGE/n).parent.mkdir(parents=True,exist_ok=True);(STAGE/n).write_bytes(data)
for n in owned+borrowed:
 (STAGE/n).parent.mkdir(parents=True,exist_ok=True);shutil.copy2(ROOT/n,STAGE/n)
result={n:sha(STAGE/n)for n in sorted(set(ledger)|set(owned)|set(borrowed))}
assert result==json.loads((OUT/'source-r7.json').read_text())
print('Restored exact final source snapshot:',len(result))
