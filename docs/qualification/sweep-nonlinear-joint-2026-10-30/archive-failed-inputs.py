from pathlib import Path
import hashlib,json,tarfile,io
OUT=Path(__file__).resolve().parent
BASE=Path('/private/tmp/osv-sweep-nonlinear-joint-20261030-r9')
base=json.loads((OUT/'source-r9.json').read_text())
sha=lambda b:hashlib.sha256(b).hexdigest()
records=[]
with tarfile.open(OUT/'failed-input-deltas.tar.gz','w:gz',compresslevel=3)as tar:
 for version in [1,2,4,6,8]:
  ledger=json.loads((OUT/f'source-r{version}.json').read_text())
  stage=Path(f'/private/tmp/osv-sweep-nonlinear-joint-20261030-r{version}')
  changes={}
  for n,h in ledger.items():
   if h==base.get(n):continue
   p=stage/n
   if version==6 and n=='crates/brep-core/src/sweep_station_reconstruction.rs':p=OUT/'diagnostic-r6-original.rs.txt'
   data=p.read_bytes();assert sha(data)==h,(version,n)
   name=f'r{version}/'+n;item=tarfile.TarInfo(name);item.size=len(data);tar.addfile(item,io.BytesIO(data));changes[n]=h
  records.append({'version':version,'sourceIndex':f'source-r{version}.json','sourceIndexSha256':sha((OUT/f'source-r{version}.json').read_bytes()),'changedInputs':changes,'removeFromBase':sorted(set(base)-set(ledger))})
(OUT/'failed-input-deltas.json').write_text(json.dumps({'baseSourceIndex':'source-r9.json','baseSourceIndexSha256':sha((OUT/'source-r9.json').read_bytes()),'archiveSha256':sha((OUT/'failed-input-deltas.tar.gz').read_bytes()),'reconstruction':'Select only the target source-index paths from qualified-source-r9.tar.gz; overlay the matching rN/ entries; verify every target hash.','attempts':records},indent=2)+'\n')
print('Preserved frozen failed input deltas:',len(records))
