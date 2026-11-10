#!/usr/bin/env python3
"""M1: move brep-core's STEP/IGES modules into cad-step, replacing cad-step's diverged copies.
usage: m1_move.py <crates_dir>"""
import os,re,shutil,sys
C=sys.argv[1]; B=f'{C}/brep-core/src'; S=f'{C}/cad-step/src'
MOVED=['step_interchange','step_interchange_v3','step_part21','step_pmi','step_v3_export','nurbs_step_interchange','nurbs_step_shared','nurbs_step_solid','nurbs_step_trimmed','iges_interchange_v2','source_exchange_step']
EXTRA_FILES=['step_tessellated','step_presentation','step_schema','step_xde']   # #[path] children living next to their parents
# 1. drop cad-step's diverged copies of the same modules
for m in MOVED:
    for p in (f'{S}/{m}.rs',f'{S}/{m}'):
        if os.path.isdir(p): shutil.rmtree(p)
        elif os.path.exists(p): os.remove(p)
# 2. move brep-core files and directories
for m in MOVED+EXTRA_FILES:
    for suf in ('.rs',''):
        src=f'{B}/{m}{suf}'
        if os.path.exists(src): shutil.move(src,f'{S}/{m}{suf}')
# brep-core STEP tests
for t in ['step_interchange_v3','step_semantic_metadata','prism_imported_caps']:
    src=f'{B}/tests/{t}.rs'
    if os.path.exists(src):
        os.makedirs(f'{S}/tests/from_brep_core',exist_ok=True); shutil.move(src,f'{S}/tests/from_brep_core/{t}.rs')
moved_names=set(MOVED+EXTRA_FILES)
pat=re.compile(r'\bcrate::([a-z_][a-z0-9_]*)')
def rewrite(text):
    return pat.sub(lambda m: m.group(0) if m.group(1) in moved_names else 'brep_core::'+m.group(1), text)
n=0
for dp,dn,fn in os.walk(S):
    for f in fn:
        if not f.endswith('.rs'): continue
        p=os.path.join(dp,f)
        if p.endswith('/lib.rs'): continue
        rel=os.path.relpath(p,S); top=rel.split('/')[0].replace('.rs','')
        if top not in moved_names and not rel.startswith('tests/from_brep_core'): continue
        t=open(p).read(); u=rewrite(t)
        if u!=t: open(p,'w').write(u); n+=1
print('rewritten files',n)
