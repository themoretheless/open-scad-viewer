import re,sys
C=sys.argv[1]; p=f'{C}/brep-core/src/import_policy.rs'; s=open(p).read()
blocks=[]
for name in ['import_step_with_policy','import_iges_v2_with_policy']:
    m=re.search(r'(?:///[^\n]*\n)*pub fn '+name+r'\(.*?\n\}\n',s,re.S); assert m,name
    blocks.append(m.group(0)); s=s.replace(m.group(0),'',1)
s=re.sub(r'(?m)^fn apply_import_policy\(','pub fn apply_import_policy(',s)
s=re.sub(r'(?m)^pub\(crate\) fn apply_import_policy\(','pub fn apply_import_policy(',s)
open(p,'w').write(s)
body='\n'.join(b.replace('crate::step_interchange_v3::','crate::step_interchange_v3::').replace('crate::iges_interchange_v2::','crate::iges_interchange_v2::') for b in blocks)
hdr='//! STEP and IGES import entry points that apply the kernel import policy after parsing.\nuse brep_core::import_policy::{ImportBudget, ImportOutcome, ImportPolicy, ImportResult, apply_import_policy};\n\n'
open(f'{C}/cad-step/src/import_policy.rs','w').write(hdr+body)
l=open(f'{C}/cad-step/src/lib.rs').read(); l=l.replace('pub mod step_interchange_v3;','pub mod step_interchange_v3;\npub mod import_policy;',1); open(f'{C}/cad-step/src/lib.rs','w').write(l)
print('ok')
