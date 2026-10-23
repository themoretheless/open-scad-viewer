#!/usr/bin/env python3
"""Replace private copies of math_core::linalg vector helpers with imports.

usage: python3 -I -B scripts/rust-dedupe-linalg.py [--dry-run]

A copy is replaced only when it is a private, module-level `fn` whose name, parameter
types, return type and body are token-identical to the `math_core::linalg` definition
(after spelling `[f64; 3]`/`[f64; 2]` as `V3`/`V2` and ignoring whitespace and argument
names), and only in crates that already depend on math-core. Same arithmetic, same
evaluation order, so results stay bit-identical.
"""
import os
import re
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', 'crates'))
dry = '--dry-run' in sys.argv
FN = re.compile(r'^(?P<attrs>(?:#\[[^\]]*\]\n)*)fn (?P<name>\w+)\((?P<params>[^)]*)\)\s*->\s*(?P<ret>[^{]+?)\s*\{\n(?P<body>.*?)\n\}\n', re.S | re.M)


def norm_type(t):
    t = re.sub(r'\s+', '', t)
    return t.replace('[f64;3]', 'V3').replace('[f64;2]', 'V2')


def signature(params, ret):
    names, types = [], []
    for p in filter(None, (x.strip() for x in params.split(','))):
        n, _, t = p.partition(':')
        names.append(n.strip())
        types.append(norm_type(t))
    return names, tuple(types), norm_type(ret)


def norm_body(body, names):
    text = re.sub(r'\s+', '', body)
    for i, n in enumerate(names):
        text = re.sub(rf'\b{re.escape(n)}\b', f'${i}', text)
    return text


lib = open(os.path.join(ROOT, 'math-core/src/linalg.rs')).read().replace('pub fn ', 'fn ')
reference = {}
for m in FN.finditer(lib):
    names, types, ret = signature(m['params'], m['ret'])
    reference[m['name']] = (types, ret, norm_body(m['body'], names))

crates = [d for d in os.listdir(ROOT) if d != 'math-core' and os.path.exists(os.path.join(ROOT, d, 'Cargo.toml'))
          and re.search(r'^math-core\s*=', open(os.path.join(ROOT, d, 'Cargo.toml')).read(), re.M)]
report = {}
for crate in sorted(crates):
    for base, _, files in os.walk(os.path.join(ROOT, crate, 'src')):
        for f in files:
            if not f.endswith('.rs'):
                continue
            path = os.path.join(base, f)
            text = open(path).read()
            replaced = []

            def swap(m):
                ref = reference.get(m['name'])
                if not ref:
                    return m[0]
                names, types, ret = signature(m['params'], m['ret'])
                if (types, ret, norm_body(m['body'], names)) != ref:
                    return m[0]
                attrs = m['attrs']
                if attrs and not re.fullmatch(r'(#\[inline(\(always\))?\]\n)*', attrs):
                    return m[0]
                replaced.append(m['name'])
                return ''
            new = FN.sub(swap, text)
            if not replaced:
                continue
            # Names the file also defines another way (or already imports) stay untouched.
            if any(re.search(rf'\buse\b[^;]*\b{n}\b', text) for n in replaced):
                continue
            imports = f"use math_core::{{{', '.join(sorted(replaced))}}};\n" if len(replaced) > 1 \
                else f"use math_core::{replaced[0]};\n"
            lines = new.split('\n')
            # Insert after the last complete top-level `use` statement (they may span lines).
            at, in_use = 0, False
            for i, line in enumerate(lines):
                if line.startswith('//!') or line.startswith('#!['):
                    at = max(at, i + 1)
                if line.startswith('use '):
                    in_use = True
                if in_use and line.rstrip().endswith(';'):
                    in_use, at = False, i + 1
                if line.startswith(('fn ', 'pub ', 'struct ', 'impl ', 'const ', 'mod ', 'enum ', 'type ', '#[')) and not in_use:
                    break
            lines.insert(at, imports.rstrip('\n'))
            new = re.sub(r'\n{3,}', '\n\n', '\n'.join(lines))
            report[os.path.relpath(path, ROOT)] = sorted(replaced)
            if not dry:
                open(path, 'w').write(new)
for k, v in report.items():
    print(f'{k}: {", ".join(v)}')
print(f'{sum(map(len, report.values()))} copies in {len(report)} files')
