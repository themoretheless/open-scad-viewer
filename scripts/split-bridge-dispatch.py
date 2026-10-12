#!/usr/bin/env python3
"""Turn geometry-bridge's monolithic `dispatch_local` match into a chain of domain routers.

usage: python3 scripts/split-bridge-dispatch.py [--target LINES]

Every arm of `match v["op"]` in crates/geometry-bridge/src/lib.rs moves verbatim, in order,
into `src/dispatch/<name>.rs` routers of about `--target` lines:

    pub(crate) fn route(mut v: Value) -> Routed {
        Routed::Handled(match v["op"].as_str().unwrap_or("") { <arms> _ => return Routed::Unhandled(v) })
    }

and the arm group holding the `_ =>` fallback becomes `dispatch::fallback`. `dispatch_local`
tries the routers in order exactly like `dispatch` already tries the domain crates, so the
operation names (all distinct: rustc rejects an unreachable duplicate) route as before.
Each router is named after the module its arms call most.
"""
import os, re, sys
from collections import Counter

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..'))
LIB = os.path.join(ROOT, 'crates/geometry-bridge/src/lib.rs')
OUT = os.path.join(ROOT, 'crates/geometry-bridge/src/dispatch')
target = int(sys.argv[sys.argv.index('--target') + 1]) if '--target' in sys.argv else 200

sys.path.insert(0, os.path.join(ROOT, 'scripts'))
import importlib.util
spec = importlib.util.spec_from_file_location('splitter', os.path.join(ROOT, 'scripts/rust-split-module.py'))
splitter = importlib.util.module_from_spec(spec); spec.loader.exec_module(splitter)

src = open(LIB).read()
lines = src.split('\n')
dep = splitter.depths(src)
start = lines.index('fn dispatch_local(mut v: Value) -> Result<Value> {')
assert lines[start + 1].strip() == 'match v["op"].as_str().unwrap_or("") {', lines[start + 1]
match_line = start + 1
end = next(i for i in range(match_line + 1, len(lines)) if dep[i] == 1 and lines[i].strip().startswith('}'))
fn_end = next(i for i in range(end + 1, len(lines)) if dep[i] == 0 and lines[i] == '}')
assert fn_end == end + 1, (end, fn_end, lines[end:fn_end + 1])
body = list(range(match_line + 1, end))
arm_starts = [i for i in body if dep[i] == 2 and re.match(r'^        ("|_ )', lines[i])]
arms = [(a, (arm_starts[k + 1] if k + 1 < len(arm_starts) else end)) for k, a in enumerate(arm_starts)]
assert arms[0][0] == match_line + 1, 'match body must start with an arm'
fallback = [k for k, (a, _) in enumerate(arms) if lines[a].startswith('        _ ')]
assert fallback == [len(arms) - 1], 'the wildcard arm must be last'

groups, cur, size = [], [], 0
for k, (a, b) in enumerate(arms[:-1]):
    n = b - a
    if cur and size + n > target:
        groups.append(cur); cur, size = [], 0
    cur.append((a, b)); size += n
if cur: groups.append(cur)

used = set()
def name_of(group):
    text = '\n'.join(lines[a] for a0, b0 in group for a in range(a0, b0))
    calls = Counter(m for m in re.findall(r'\b([a-z][a-z0-9_]*)::[a-z_][a-z0-9_]*\(', text)
                    if m not in ('value_codec', 'nurbs_core', 'brep_core', 'json', 'std', 'self', 'crate', 'super'))
    base = calls.most_common(1)[0][0] if calls else 'routes'
    name, n = base, 2
    while name in used or name in ('mod', 'fallback'): name = f'{base}_{n}'; n += 1
    used.add(name)
    return name

os.makedirs(OUT, exist_ok=True)
names = []
for group in groups:
    name = name_of(group)
    names.append(name)
    arm_text = '\n'.join(lines[a] for a0, b0 in group for a in range(a0, b0))
    open(os.path.join(OUT, f'{name}.rs'), 'w').write(
        f'//! Bridge operations routed mostly to `{name}`.\nuse crate::*;\n\n'
        'pub(crate) fn route(mut v: Value) -> Routed {\n'
        '    Routed::Handled(match v["op"].as_str().unwrap_or("") {\n'
        f'{arm_text}\n'
        '        _ => return Routed::Unhandled(v),\n'
        '    })\n'
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
    '//! Local bridge operations, grouped into routers tried in order by `dispatch_local`.\n'
    + ''.join(f'pub(crate) mod {n};\n' for n in names) + 'pub(crate) mod fallback;\n\n'
    'use bridge_codec::Router;\n\n'
    '/// Local routers, in the order the former single match listed their operations.\n'
    'pub(crate) const ROUTES: &[Router] = &[\n'
    + ''.join(f'    {n}::route,\n' for n in names) + '];\n')
new_fn = ['fn dispatch_local(mut v: Value) -> Result<Value> {',
          '    for route in dispatch::ROUTES {',
          '        match route(v) {',
          '            Routed::Handled(result) => return result,',
          '            Routed::Unhandled(request) => v = request,',
          '        }',
          '    }',
          '    dispatch::fallback::dispatch(v)',
          '}']
lines = lines[:start] + new_fn + lines[fn_end + 1:]
# declare the module next to the other private modules
at = next(i for i, l in enumerate(lines) if l == 'mod cad_body_affine;')
lines.insert(at, 'mod dispatch;')
open(LIB, 'w').write('\n'.join(lines))
print({'routers': names, 'arms': len(arms), 'lib_lines': len(lines)})
