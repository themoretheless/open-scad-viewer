"""Compare relocated implementation/test text against the immutable base commit."""
import hashlib,json,subprocess
from pathlib import Path
BASE='419089d79db7bd3b50c67b297e113d9bfc6694a6'
checks=[]
def original(path): return subprocess.check_output(['git','show',f'{BASE}:{path}'],text=True)
def segment(path,start,end=None):
 s=original(path);a=s.index(start);b=s.index(end,a) if end else len(s);return s[a:b]
def verify(path,expected):
 actual=Path(path).read_text();assert actual.strip()==expected.strip(),path
 checks.append({'path':path,'sha256':hashlib.sha256(actual.encode()).hexdigest(),'textPreserved':True})
b='crates/brep-core/src/'
for path,start,end,child in [
 ('minkowski3d/curved.rs','impl Serialize for Source {','#[cfg(test)]\nfn save_fixture','minkowski3d/curved/codec.rs'),
 ('nurbs_step_shared.rs','fn refs(token: &str)','pub(crate) fn resolve_cartesian(','nurbs_step_shared/linked_graph.rs'),
 ('region_replacement/six_edge_corner.rs','pub struct SixEdgeGraphProof {','fn radius_intervals(','region_replacement/six_edge_corner/graph_proof.rs'),
 ('source_face_contacts.rs','/// Inspect every face pair with exact original hull separation','#[cfg(test)]','source_face_contacts/projective.rs')]:
 verify(b+child,'use super::*;\n'+segment(b+path,start,end))
verify(b+'source_shell_incidence/tests/projective_qualification.rs',original(b+'source_shell_incidence/projective_qualification.rs'))
b='crates/nurbs-core/src/analysis/curve_surface_composition/'
for path,name,end,child in [('graph_trace.rs','tests',None,'graph_trace/tests/mod.rs'),('exact_trace.rs','tests','#[cfg(test)]\nmod tensor_contraction_tests','exact_trace/tests/mod.rs'),('exact_trace.rs','tensor_contraction_tests',None,'exact_trace/tests/tensor_contraction.rs')]:
 s=segment(b+path,'#[cfg(test)]\nmod '+name+' {',end);verify(b+child,s[s.index('{')+1:s.rfind('}')])
s=segment(b+'exact_trace.rs','pub(super) fn extraction(','#[cfg(test)]\nmod tests {').replace('pub(super) fn extraction','pub(in super::super) fn extraction',1)
t=segment(b+'exact_trace.rs','fn mix(','/// Restrict an original clamped pcurve').replace('fn mix(','pub(super) fn mix(',1).replace('fn blossom(','pub(super) fn blossom(',1)
verify(b+'exact_trace/extraction.rs','use super::*;\n'+s+'\n'+t)
b='crates/nurbs-core/src/sweeps/progressive_sweep/'
s=segment(b+'authored_error.rs','#[cfg(test)]\nfn check_nonaxial_planar_rmf_original_image','pub(super) fn rmf_spatial_patch_error(').replace('fn check_nonaxial_planar_rmf_original_image','pub(super) fn check_nonaxial_planar_rmf_original_image',1).replace('fn check_original_planar_frame_arc_length','pub(super) fn check_original_planar_frame_arc_length',1)
verify(b+'authored_error/tests/frame_checks.rs','use super::*;\n'+s)
print(json.dumps({'baseCommit':BASE,'checks':checks,'passed':len(checks),'scope':'Complete relocated text matches base; only the explicitly listed helper visibility changes, module wrappers and imports are normalized. Compilation and behavior are separate checks.'},indent=2))
