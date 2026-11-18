import re,sys
C=sys.argv[1]
p=f'{C}/brep-core/src/close_topology/interchange.rs'; s=open(p).read()
for n in ['export_complex_step','import_complex_step','export_complex_iges','import_complex_iges']:
    m=re.search(r'(?:///[^\n]*\n)*pub fn '+n+r'\(.*?\n\}\n',s,re.S); assert m,n; s=s.replace(m.group(0),'',1)
open(p,'w').write(s)
for p in [f'{C}/brep-core/src/lib.rs',f'{C}/brep-core/src/close_topology/mod.rs']:
    try: s=open(p).read()
    except FileNotFoundError: continue
    t=re.sub(r'\b(export_complex_iges|export_complex_step|import_complex_iges|import_complex_step),\s*','',s)
    t=re.sub(r',\s*(export_complex_iges|export_complex_step|import_complex_iges|import_complex_step)\b','',t)
    if t!=s: open(p,'w').write(t); print('edited',p)
