#!/usr/bin/env python3
"""Group brep-core top-level modules into candidate bounded contexts (by naming family) and measure coupling."""
import csv, sys, os, collections, json, re
A = sys.argv[1]
def rows(n): return list(csv.DictReader(open(os.path.join(A, n), newline=''), delimiter='\t', quoting=csv.QUOTE_NONE))
files, refs, rootuse = rows('files.tsv'), rows('refs.tsv'), rows('rootuse.tsv')
C = 'brep-core'
RULES = [
 ('interchange', r'^(step_|iges_|nurbs_step_|import_policy$|source_exchange_)'),
 ('intersection', r'^(intersections$|analytic_ss$|nurbs_ss_|ss_|predicate_evidence$|coverage_verifier$|profile_intersections$|self_intersection$|face_crossings$)'),
 ('minkowski', r'^(minkowski3d$|affine_lattice$)'),
 ('source_body', r'^(source_|material_|wall_|exact_affine_source$|exact_graph_strip$|linear_canal|trimmed_|graph_body$|rational_identity$|rational_cap_)'),
 ('profile', r'^(profile_|bezier_profile$|prism|stepped_prism$|planar_cap$)'),
 ('sweep_offset', r'^(sweep_|miter_seams$|offset_|retained_annular_cap$|radius_transition_body$)'),
 ('contact', r'^(contact_|face_contact|boundary_|shared_boundary$|control_hull_separation$|sphere_mate$|fillet_contact$)'),
 ('boolean', r'^(operations$|analytic_boolean$|tolerant_boolean$|boolean_|.*_boolean$|trim_sew$|imprint_|uv_|planar_trim$|face_cut$|retained_uv_|close_topology$|corner_close$)'),
 ('features', r'^(feature_ops$|analytic_features$|aag|fillet_radius$|circular_blend$|edge_partition$|edge_tangency$|direct_edit$|body_edit$|face_replacement$|region_replacement$|healing|pcurve_repair$|tolerance_unify$|operation_tolerance$|rounded_patch_network$)'),
 ('analysis', r'^(analysis$|face_curvature$|face_injectivity$|shell_thickness$|shell_distance$|solid_distance$|ray_parity$|volume_validity$|solid_audit$|watertight$|defects$|model_validation$|shell_nesting$|shell_orientation$|shell_relation$)'),
 ('parts', r'^(analytic$|gear$|thread$|rack$|worm$|blade$|sketch$|cuboid_primitive$)'),
 ('model', r'.*'),
]
def fam(m):
    for name, rx in RULES:
        if re.search(rx, m): return name
tops = collections.Counter()
for f in files:
    if f['crate'] == C and f['kind'] == 'lib' and f['test'] == '0':
        tops[f['module'].split('::')[0] if f['module'] else '(root)'] += int(f['loc'])
alias = {}
for u in rootuse:
    if u['crate'] == C and u['kind'] == 'lib' and u['glob'] == '0':
        p = u['path'].split('::')
        if p and p[0] in ('crate', 'self'): p = p[1:]
        if p: alias[u['alias']] = p[0]
floc = collections.Counter(); fmods = collections.Counter()
for m, l in tops.items(): floc[fam(m)] += l; fmods[fam(m)] += 1
ed = collections.Counter(); med = collections.defaultdict(collections.Counter)
for r in refs:
    if r['crate'] != C or r['tkind'] != 'crate' or r['kind'] != 'lib' or r['test'] == '1': continue
    a = r['from_module'].split('::')[0] if r['from_module'] else '(root)'; b = r['t0']
    if b not in tops: b = alias.get(b, '(root)')
    if b not in tops or a == '(root)' or b == '(root)' or a == b: continue
    fa, fb = fam(a), fam(b)
    if fa != fb: ed[(fa, fb)] += int(r['count']); med[(fa, fb)][(a, b)] += int(r['count'])
names = [n for n, _ in RULES]
print("family          modules     LOC")
for n in sorted(names, key=lambda n: -floc[n]): print(f"  {n:14s}{fmods[n]:6d}{floc[n]:9d}")
print("\ncoupling matrix (row uses column; number of references):")
print(' ' * 14 + ''.join(f"{n[:9]:>10s}" for n in names))
for a in names: print(f"{a:14s}" + ''.join(f"{(ed[(a, b)] or ''):>10}" for b in names))
# best linear order (greedy: repeatedly pick family with fewest outgoing refs to remaining)
remaining = set(names); order = []
while remaining:
    best = min(remaining, key=lambda a: (sum(ed[(a, b)] for b in remaining if b != a), -sum(ed[(b, a)] for b in remaining if b != a)))
    order.append(best); remaining.discard(best)
pos = {n: i for i, n in enumerate(order)}
up = [(a, b, n) for (a, b), n in ed.items() if pos[b] > pos[a]]
tot = sum(ed.values())
print("\nlowest-violation layering (bottom -> top):", ' < '.join(order))
print(f"upward refs under this order: {sum(n for *_, n in up)} of {tot} cross-family refs ({100*sum(n for *_, n in up)/tot:.1f}%), in {len(up)} family pairs")
for a, b, n in sorted(up, key=lambda x: -x[2])[:14]:
    ex = ', '.join(f"{x}->{y}({k})" for (x, y), k in med[(a, b)].most_common(3))
    print(f"   {a:12s} -> {b:12s} {n:5d}   e.g. {ex}")
json.dump(dict(families={n: dict(modules=fmods[n], loc=floc[n]) for n in names}, order=order, matrix={f"{a}->{b}": n for (a, b), n in ed.items()},
               upward=[dict(a=a, b=b, refs=n, examples=[f"{x}->{y}({k})" for (x, y), k in med[(a, b)].most_common(4)]) for a, b, n in sorted(up, key=lambda x: -x[2])],
               members={n: sorted([m for m in tops if fam(m) == n]) for n in names}), open(os.path.join(A, 'brep-families.json'), 'w'), indent=1)
