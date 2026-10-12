#!/usr/bin/env python3
"""Turn geometry-bridge's monolithic `dispatch_local` match into a chain of domain routers.

usage: python3 scripts/split-bridge-dispatch.py [--target LINES]

Every arm of `match v["op"]` in crates/geometry-bridge/src/lib.rs moves verbatim into a
`src/dispatch/<family>.rs` router. A router owns exactly the operation names its arm patterns
list (`OPS`; the arms have no guards), hands every other operation back, and runs its arms
unchanged inside `handle`, which keeps their `?` and early `return` semantics. The `_ =>`
fallback arm becomes `dispatch::fallback`. `dispatch_local` tries the routers in turn exactly
like `dispatch` already tries the domain crates; operation names are distinct (rustc rejects
an unreachable duplicate arm), so every operation routes to the same arm as before.

Routers group the arms by operation family (mesh, intersections, sweep, interchange, nurbs,
cad, brep, general), keeping their relative order; a family over `--target` lines splits in order.
"""
import importlib.util
import os
import re
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..'))
LIB = os.path.join(ROOT, 'crates/geometry-bridge/src/lib.rs')
OUT = os.path.join(ROOT, 'crates/geometry-bridge/src/dispatch')
target = int(sys.argv[sys.argv.index('--target') + 1]) if '--target' in sys.argv else 780

spec = importlib.util.spec_from_file_location('splitter', os.path.join(ROOT, 'scripts/rust-split-module.py'))
splitter = importlib.util.module_from_spec(spec)
spec.loader.exec_module(splitter)

FAMILIES = [
    ('mesh', lambda op: op.startswith('mesh_')),
    ('intersections', lambda op: op.startswith(('brep_intersect', 'brep_intersection'))),
    ('sweep', lambda op: op.startswith(('brep_sweep', 'brep_miter', 'brep_progressive', 'brep_transform_certified',
                                        'brep_smooth_certified', 'brep_reconstruct_certified'))),
    ('interchange', lambda op: op.startswith(('brep_nurbs_import', 'brep_nurbs_export', 'brep_nurbs_compose'))),
    ('nurbs', lambda op: op.startswith('brep_nurbs')),
    ('cad', lambda op: op.startswith('cad_')),
    ('brep', lambda op: op.startswith('brep_')),
    ('general', lambda op: True),
]

src = open(LIB).read()
lines = src.split('\n')
dep = splitter.depths(src)
start = lines.index('fn dispatch_local(mut v: Value) -> Result<Value> {')
match_line = start + 1
assert lines[match_line].strip() == 'match v["op"].as_str().unwrap_or("") {', lines[match_line]
end = next(i for i in range(match_line + 1, len(lines)) if dep[i] == 2 and lines[i] == '    }')
fn_end = next(i for i in range(end + 1, len(lines)) if dep[i] == 1 and lines[i] == '}')
arm_starts = [i for i in range(match_line + 1, end) if dep[i] == 2 and re.match(r'^        ("|_ )', lines[i])]
arms = [(a, arm_starts[k + 1] if k + 1 < len(arm_starts) else end) for k, a in enumerate(arm_starts)]
assert arms[0][0] == match_line + 1, 'match body must start with an arm'
assert [k for k, (a, _) in enumerate(arms) if lines[a].startswith('        _ ')] == [len(arms) - 1], \
    'the wildcard arm must be the last arm'


def arm_ops(a0, b0):
    pattern = []
    for a in range(a0, b0):
        head, sep, _ = lines[a].partition('=>')
        pattern.append(head)
        if sep:
            break
    text = ' '.join(pattern)
    assert ' if ' not in text, f'guarded arm at line {a0 + 1}'
    return re.findall(r'"([^"]+)"', text)


by_family = {name: [] for name, _ in FAMILIES}
for a0, b0 in arms[:-1]:
    family = next(name for name, test in FAMILIES if test(arm_ops(a0, b0)[0]))
    by_family[family].append((a0, b0))
groups, names = [], []
for family, family_arms in by_family.items():
    if not family_arms:
        continue
    part, size, k = [], 0, 1
    for arm in family_arms:
        if part and size + (arm[1] - arm[0]) > target:
            groups.append(part); names.append(family if k == 1 else f'{family}_{k}')
            k += 1; part, size = [], 0
        part.append(arm); size += arm[1] - arm[0]
    groups.append(part); names.append(family if k == 1 else f'{family}_{k}')

os.makedirs(OUT, exist_ok=True)
for name, group in zip(names, groups):
    arm_text = '\n'.join(lines[a] for a0, b0 in group for a in range(a0, b0))
    ops = [op for a0, b0 in group for op in arm_ops(a0, b0)]
    uses_mut = re.search(r'&mut v\b|\bv\s*(?:\[[^\]]*\])+\s*=[^=]', arm_text) is not None
    ops_text = ''.join(f'    "{op}",\n' for op in ops)
    family = name.split('_')[0]
    open(os.path.join(OUT, f'{name}.rs'), 'w').write(
        f'//! Bridge operations of the `{family}` family.\nuse crate::*;\n\n'
        f'const OPS: &[&str] = &[\n{ops_text}];\n\n'
        'pub(crate) fn route(v: Value) -> Routed {\n'
        '    if !OPS.contains(&v["op"].as_str().unwrap_or("")) {\n'
        '        return Routed::Unhandled(v);\n'
        '    }\n'
        '    Routed::Handled(handle(v))\n'
        '}\n\n'
        f'fn handle({"mut " if uses_mut else ""}v: Value) -> Result<Value> {{\n'
        '    match v["op"].as_str().unwrap_or("") {\n'
        f'{arm_text}\n'
        '        _ => unreachable!("route admits only OPS"),\n'
        '    }\n'
        '}\n')
fa, fb = arms[-1]
open(os.path.join(OUT, 'fallback.rs'), 'w').write(
    '//! Operations no local router owns go to the NURBS kernel.\nuse crate::*;\n\n'
    'pub(crate) fn dispatch(v: Value) -> Result<Value> {\n'
    '    match v["op"].as_str().unwrap_or("") {\n'
    + '\n'.join(lines[fa:fb]) + '\n'
    '    }\n'
    '}\n')
open(os.path.join(OUT, 'mod.rs'), 'w').write(
    '//! Local bridge operations, one router per operation family; `dispatch_local` tries them\n'
    '//! in turn and leaves unowned operations to `fallback`.\n'
    + ''.join(f'pub(crate) mod {n};\n' for n in names) + 'pub(crate) mod fallback;\n\n'
    'use bridge_codec::Router;\n\n'
    'pub(crate) const ROUTES: &[Router] = &[\n'
    + ''.join(f'    {n}::route,\n' for n in names) + '];\n')
lines = lines[:start] + [
    'fn dispatch_local(mut v: Value) -> Result<Value> {',
    '    for route in dispatch::ROUTES {',
    '        match route(v) {',
    '            Routed::Handled(result) => return result,',
    '            Routed::Unhandled(request) => v = request,',
    '        }',
    '    }',
    '    dispatch::fallback::dispatch(v)',
    '}'] + lines[fn_end + 1:]
lines.insert(next(i for i, l in enumerate(lines) if l == 'mod cad_body_affine;'), 'mod dispatch;')
open(LIB, 'w').write('\n'.join(lines))
print({'routers': dict(zip(names, (len(g) for g in groups))), 'arms': len(arms), 'lib_lines': len(lines)})
