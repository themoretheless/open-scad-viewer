import os,re,sys
S=sys.argv[1]
MOVED={'step_interchange','step_interchange_v3','step_part21','step_pmi','step_v3_export','nurbs_step_interchange','nurbs_step_shared','nurbs_step_solid','nurbs_step_trimmed','iges_interchange_v2','source_exchange_step','step_tessellated','step_presentation','step_schema','step_xde','import_policy','close_topology_interchange','iges_interchange_v1'}
def split_top(s):
    out=[];d=0;cur=''
    for ch in s:
        if ch=='{': d+=1
        if ch=='}': d-=1
        if ch==',' and d==0: out.append(cur.strip()); cur=''
        else: cur+=ch
    if cur.strip(): out.append(cur.strip())
    return out
def fix(text):
    def rep(m):
        vis,body=m.group(1),m.group(2)
        own=[];ext=[]
        for it in split_top(body):
            head=re.split(r'::|\s',it)[0]
            (own if head in MOVED or head in ('self',) else ext).append(it)
        lines=[]
        if own: lines.append(f'{vis}use crate::{{{", ".join(own)}}};')
        if ext: lines.append(f'{vis}use brep_core::{{{", ".join(ext)}}};')
        return '\n'.join(lines)
    return re.sub(r'((?:pub(?:\([^)]*\))?\s+)?)use\s+crate::\{((?:[^{}]|\{[^{}]*\})*)\};',rep,text)
n=0
for dp,dn,fn in os.walk(S):
    for f in fn:
        if not f.endswith('.rs') or f=='lib.rs': continue
        p=os.path.join(dp,f); t=open(p).read(); u=fix(t)
        if u!=t: open(p,'w').write(u); n+=1
print('files',n)
