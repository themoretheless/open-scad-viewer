"""Verify that transport bodies moved intact, excluding codec attributes/import."""
from pathlib import Path
import subprocess,json,hashlib
base='4ad06539c9b6cef2032204363b9ac659d589e796'
root='crates/brep-core/src/minkowski3d/curved/'
rows=[]
for name in ['atlas','atlas_legacy','prismatic','shear','topology']:
 s=subprocess.check_output(['git','show',f'{base}:{root}{name}.rs'],text=True)
 start=s.index('impl Serialize');end=s.find('#[cfg(test)]',start);end=len(s) if end<0 else end
 original=s[start:end].strip();path=root+name+'/codec.rs';actual=Path(path).read_text();body=actual.removeprefix('use super::*;\n').replace('#[cfg(feature = "codec")]\n','').strip()
 assert original==body,path
 rows.append({'path':path,'sha256':hashlib.sha256(actual.encode()).hexdigest(),'unchangedTransportBodies':True})
print(json.dumps({'baseCommit':base,'passed':len(rows),'checks':rows},indent=2))
