import re,sys
C=sys.argv[1]
MOVED=['step_interchange','step_interchange_v3','step_part21','step_pmi','step_v3_export','nurbs_step_interchange','nurbs_step_shared','nurbs_step_solid','nurbs_step_trimmed','iges_interchange_v2','source_exchange_step']
p=f'{C}/brep-core/src/lib.rs'; s=open(p).read()
vis={}
for m in MOVED:
    mm=re.search(r'(?m)^(pub )?mod '+m+r';\n',s); assert mm,m
    vis[m]='pub ' if mm.group(1) else ''
    s=s[:mm.start()]+s[mm.end():]
    s=re.sub(r'(?ms)^pub use '+m+r'::\{.*?\};\n','',s)
    s=re.sub(r'(?m)^pub use '+m+r'::[^;\n]*;\n','',s)
open(p,'w').write(s)
p=f'{C}/cad-step/src/lib.rs'; s=open(p).read()
for m in MOVED:
    if re.search(r'(?m)^(pub )?mod '+m+r';',s):
        s=re.sub(r'(?m)^(pub )?mod '+m+r';',f'{vis[m]}mod {m};',s)
    else:
        s=s.replace('pub mod source_exchange_step;',f'pub mod source_exchange_step;\n{vis[m]}mod {m};',1)
open(p,'w').write(s)
print(vis)
