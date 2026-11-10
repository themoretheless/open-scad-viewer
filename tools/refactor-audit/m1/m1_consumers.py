#!/usr/bin/env python3
"""Point consumers at cad_step for names that moved out of brep-core. usage: m1_consumers.py <crates_dir> <head_brep_lib.rs> <files...>"""
import re,sys
C,head=sys.argv[1],open(sys.argv[2]).read()
MOVED=['step_interchange','step_interchange_v3','step_part21','step_pmi','step_v3_export','nurbs_step_interchange','nurbs_step_shared','nurbs_step_solid','nurbs_step_trimmed','iges_interchange_v2','source_exchange_step','import_policy_facades']
names=set(MOVED)
for m in MOVED:
    for blk in re.findall(r'(?ms)^pub use '+m+r'::\{(.*?)\};',head): names|={x.strip().split(' as ')[-1] for x in blk.split(',') if x.strip()}
names|={'export_complex_step','import_complex_step','export_complex_iges','import_complex_iges','import_step_with_policy','import_iges_v2_with_policy'}
names.discard('')
def split_top(s):
    out=[];d=0;cur=''
    for ch in s:
        d+=(ch=='{')-(ch=='}')
        if ch==',' and d==0: out.append(cur.strip()); cur=''
        else: cur+=ch
    if cur.strip(): out.append(cur.strip())
    return out
def fix(t):
    def rep(m):
        vis,body=m.group(1),m.group(2); a=[];b=[]
        for it in split_top(body):
            head_=re.split(r'::|\s',it)[0]
            (b if head_ in names else a).append(it)
        out=[]
        if a: out.append(f'{vis}use brep_core::{{{", ".join(a)}}};')
        if b: out.append(f'{vis}use cad_step::{{{", ".join(b)}}};')
        return '\n'.join(out)
    t=re.sub(r'((?:pub(?:\([^)]*\))?\s+)?)use\s+brep_core::\{((?:[^{}]|\{[^{}]*\})*)\};',rep,t)
    t=re.sub(r'\bbrep_core::([a-zA-Z_]\w*)',lambda m: ('cad_step::' if m.group(1) in names else 'brep_core::')+m.group(1),t)
    return t
n=0
for p in sys.argv[3:]:
    t=open(p).read(); u=fix(t)
    if u!=t: open(p,'w').write(u); n+=1
print('files changed',n,'names',len(names))
