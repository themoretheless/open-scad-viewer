import re,sys
C=sys.argv[1]; S=f'{C}/cad-step/src'
def take(path,name):
    L=open(path).read().split('\n')
    i=next(k for k,l in enumerate(L) if re.match(rf'\s*(pub(\([^)]*\))?\s+)?fn\s+{name}\b',l))
    a=i
    while a>0 and re.match(r'\s*(#\[|///)',L[a-1]): a-=1
    d=0;s=False;j=i
    while True:
        d+=L[j].count('{')-L[j].count('}'); s|='{' in L[j]; j+=1
        if s and d<=0: break
    block='\n'.join(L[a:j]); open(path,'w').write('\n'.join(L[:a]+L[j:])); return block
blocks=[take(f'{S}/nurbs_step_shared.rs','is_uniform_bicubic_positive'),take(f'{S}/nurbs_step_shared.rs','as_uniform_bicubic'),
        take(f'{S}/nurbs_step_solid.rs','freeform_cuboid_solid'),take(f'{S}/nurbs_step_solid.rs','bump_top_surface'),take(f'{S}/nurbs_step_solid.rs','freeform_cuboid_with_bump_face')]
code='\n\n'.join(b.replace('pub(crate) fn','pub fn').replace('brep_core::','crate::') for b in blocks)
hdr='//! Freeform cuboid solids whose faces are uniform bicubic B-splines (weights 1), used by\n//! freeform STEP export and by kernel tests.\nuse crate::Model;\nuse nurbs_core::{Result, surface::Surface};\n\n'
open(f'{C}/brep-core/src/freeform_cuboid.rs','w').write(hdr+code+'\n')
l=open(f'{C}/brep-core/src/lib.rs').read()
l=l.replace('pub mod import_policy;','pub mod import_policy;\npub mod freeform_cuboid;\npub use freeform_cuboid::{freeform_cuboid_solid, freeform_cuboid_with_bump_face};',1)
open(f'{C}/brep-core/src/lib.rs','w').write(l)
sh=open(f'{S}/nurbs_step_shared.rs').read(); sh=sh.replace('\nuse ','\npub(crate) use brep_core::freeform_cuboid::{as_uniform_bicubic, is_uniform_bicubic_positive};\nuse ',1); open(f'{S}/nurbs_step_shared.rs','w').write(sh)
so=open(f'{S}/nurbs_step_solid.rs').read(); so=so.replace('use brep_core::Model;','use brep_core::Model;\npub use brep_core::freeform_cuboid::{freeform_cuboid_solid, freeform_cuboid_with_bump_face};',1); open(f'{S}/nurbs_step_solid.rs','w').write(so)
print('ok')
