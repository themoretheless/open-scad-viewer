#!/usr/bin/env python3
"""Split one oversized Rust source file into a directory module, by pure code motion.

usage: python3 scripts/rust-split-module.py <crate-dir> <path/to/file.rs> [--target LINES]

The file `x.rs` (or `lib.rs`/`mod.rs` is refused) becomes `x/mod.rs` plus submodules:

* top-level items are found with a brace-aware scanner (strings, raw strings, chars,
  comments and lifetimes are skipped) and grouped, in order, into submodules of about
  `--target` lines, each named after its first public item;
* the leading `use`/attribute/doc preamble and small leading constants stay in `mod.rs`;
* every top-level `#[cfg(test)] mod NAME { .. }` becomes `NAME/mod.rs` (dedented), so the
  architecture ratchet keeps treating it as test code;
* `include_str!`/`include_bytes!` paths gain the `../` the extra directory level needs,
  and `pub(super)` becomes `pub(in super::super)` so it keeps naming the same module;
* `mod.rs` declares the submodules at its end (so `macro_rules!` stay in textual scope)
  and re-exports them with `pub use child::*`, which keeps every public path unchanged.

Visibility is then repaired from `cargo check` diagnostics until the crate compiles: items,
fields and methods that siblings reach become `pub(super)`, which is visible to the parent
module and therefore to every sibling, but no wider than the original file module.
Nothing else changes; `--verify` re-checks that the line multiset is preserved.
"""
import json, os, re, subprocess, sys
from collections import Counter

KEYWORDS = {'as', 'break', 'const', 'continue', 'crate', 'else', 'enum', 'extern', 'false', 'fn', 'for', 'if',
            'impl', 'in', 'let', 'loop', 'match', 'mod', 'move', 'mut', 'pub', 'ref', 'return', 'self', 'Self',
            'static', 'struct', 'super', 'trait', 'true', 'type', 'unsafe', 'use', 'where', 'while', 'async',
            'await', 'dyn', 'abstract', 'become', 'box', 'do', 'final', 'macro', 'override', 'priv', 'typeof',
            'unsized', 'virtual', 'yield', 'try', 'gen', 'test', 'tests', 'core', 'std', 'alloc'}
ITEM = re.compile(r'^(?:pub(?:\([^)]*\))? )?(?:(?:const|unsafe|async|extern "C"|default) )*'
                  r'(?:(fn|struct|enum|union|trait|impl|mod|const|static|type)\b|(macro_rules!|include!))')
USE_START = re.compile(r'^(?:pub(?:\([^)]*\))? )?use ')


def depths(src):
    """Brace depth at the start of every line, skipping strings, chars and comments."""
    out, depth, i, n = [], 0, 0, len(src)
    block = 0
    out.append(0)
    inside = [False]
    while i < n:
        c = src[i]
        if block:
            if src.startswith('*/', i): block -= 1; i += 2; continue
            if src.startswith('/*', i): block += 1; i += 2; continue
            if c == '\n': out.append(depth); inside.append(True)
            i += 1; continue
        if src.startswith('//', i):
            j = src.find('\n', i)
            i = n if j < 0 else j
            continue
        if src.startswith('/*', i): block = 1; i += 2; continue
        m = re.match(r'b?r(#*)"', src[i:i + 70]) if c in 'rb' and (i == 0 or not (src[i - 1].isalnum() or src[i - 1] == '_')) else None
        if m:
            end = '"' + m.group(1)
            j = src.find(end, i + m.end())
            seg = src[i:j + len(end)]
            for _ in range(seg.count('\n')): out.append(depth); inside.append(True)
            i = j + len(end); continue
        if c == '"' or (c == 'b' and src.startswith('b"', i)):
            j = i + (2 if c == 'b' else 1)
            while j < n and src[j] != '"':
                j += 2 if src[j] == '\\' else 1
            seg = src[i:j + 1]
            for _ in range(seg.count('\n')): out.append(depth); inside.append(True)
            i = j + 1; continue
        if c == "'":
            m = re.match(r"'(\\(?:u\{[0-9a-fA-F]+\}|x[0-9a-fA-F]{2}|.)|[^\\'])'", src[i:i + 12])
            if m: i += m.end(); continue
            i += 1; continue
        if c == '{': depth += 1
        elif c == '}': depth -= 1
        elif c == '\n': out.append(depth); inside.append(False)
        i += 1
    return [d if not f else None for d, f in zip(out, inside)]


def item_starts(lines, dep):
    starts = []
    for k, l in enumerate(lines):
        if dep[k] == 0 and ITEM.match(l):
            j = k
            while j > 0 and dep[j - 1] == 0 and lines[j - 1].startswith(('#[', '//')) \
                    and not lines[j - 1].startswith('//!'):
                j -= 1
            starts.append((j, k))
    return starts


def snake(name):
    s = re.sub(r'(?<!^)(?=[A-Z])', '_', name).lower()
    return re.sub(r'[^a-z0-9_]', '', s)


def name_for(chunk, used):
    first = None
    for l in chunk:
        m = re.match(r'^pub (?:(?:const|unsafe|async) )*(fn|struct|enum|trait|type) ([A-Za-z_][A-Za-z0-9_]*)', l)
        if m: first = m.group(2); break
    if first is None:
        for l in chunk:
            m = re.match(r'^(?:pub(?:\([^)]*\))? )?(?:(?:const|unsafe|async) )*(fn|struct|enum|trait|type|impl(?:<[^>]*>)?) ([A-Za-z_][A-Za-z0-9_]*)', l)
            if m: first = m.group(2); break
    base = snake(first or 'items') or 'items'
    if base in KEYWORDS or base[0].isdigit(): base = base + '_items'
    name, k = base, 2
    while name in used: name = f'{base}_{k}'; k += 1
    used.add(name)
    return name


def child_text(lines):
    text = '\n'.join(lines)
    text = re.sub(r'(include_(?:str|bytes)!\(\s*")', r'\1../', text)
    text = re.sub(r'(?<![:\w])super::', 'super::super::', text)
    text = text.replace('pub(super)', 'pub(in super::super)')
    return text


def split(path, target):
    src = open(path).read()
    lines = src.split('\n')
    if lines and lines[-1] == '': lines.pop()
    dep = depths(src)
    starts = item_starts(lines, dep)
    if not starts: raise SystemExit('no items found')
    # Preamble: everything before the first fn/struct/enum/impl/trait/macro; keeps uses, consts, types.
    first_big = next((s for s, k in starts if re.match(r'^(?:pub(?:\([^)]*\))? )?(?:(?:const|unsafe|async) )*(fn|struct|enum|union|trait|impl|macro_rules!)\b', lines[k])), None)
    if first_big is None: raise SystemExit('no movable items')
    pre = lines[:first_big]
    bounds = [s for s, _ in starts if s >= first_big] + [len(lines)]
    segs = [lines[bounds[i]:bounds[i + 1]] for i in range(len(bounds) - 1)]
    tests, body, decls = [], [], []
    offset = first_big
    for seg in segs:
        base_line = offset; offset += len(seg)
        # Hoist top-level `use` statements into mod.rs: they scope the whole module.
        kept, hoisted, k = [], [], 0
        while k < len(seg):
            if dep[base_line + k] == 0 and USE_START.match(seg[k]):
                j = k
                while not seg[j].rstrip().endswith(';'): j += 1
                hoisted += seg[k:j + 1]; k = j + 1; continue
            kept.append(seg[k]); k += 1
        if hoisted: decls.append(hoisted)
        seg = kept
        code = [l for l in seg if l.strip() and not l.startswith(('#[', '//'))]
        if not code:
            if any(l.strip() for l in seg): decls.append(seg)
            continue
        first_code = re.sub(r'^(#\[[^\]]*\]\s*)+', '', code[0])
        if (len(code) == 1 and (re.match(r'^(?:pub(?:\([^)]*\))? )?mod \w+;$', first_code)
                                or re.match(r'^include!\(\s*"[^"]+"\s*\);$', first_code))) \
                or code[0].startswith('macro_rules!'):
            decls.append(seg); continue
        head = [l for l in seg if l.strip()]
        is_test = (len(head) >= 2 and head[0].strip() == '#[cfg(test)]' and re.match(r'^mod \w+ \{$', head[1])
                   and seg[[i for i, l in enumerate(seg) if l.strip()][-1]] == '}')
        (tests if is_test else body).append(seg)
    chunks, cur = [], []
    for seg in body:
        if cur and sum(map(len, cur)) + len(seg) > target:
            chunks.append(cur); cur = []
        cur.append(seg)
    if cur: chunks.append(cur)
    if len(chunks) < 2:
        raise SystemExit('one oversized item: splitting would only nest it, refusing')
    stem = os.path.splitext(path)[0]
    os.makedirs(stem, exist_ok=True)
    used = {'tests'} | {os.path.splitext(f)[0] for f in os.listdir(stem)}
    names = []
    for chunk in chunks:
        flat = [l for seg in chunk for l in seg]
        while flat and flat[-1] == '': flat.pop()
        name = name_for(flat, used)
        names.append(name)
        with open(os.path.join(stem, name + '.rs'), 'w') as f:
            f.write('use super::*;\n\n' + child_text(flat) + '\n')
    test_names = []
    for seg in tests:
        idx = [i for i, l in enumerate(seg) if l.strip()]
        name = re.match(r'^mod (\w+) \{$', seg[idx[1]].strip()).group(1)
        inner = seg[idx[1] + 1:idx[-1]]
        inner = [l[4:] if l.startswith('    ') else l for l in inner]
        while inner and inner[-1] == '': inner.pop()
        os.makedirs(os.path.join(stem, name), exist_ok=True)
        text = '\n'.join(inner)
        text = re.sub(r'(include_(?:str|bytes)!\(\s*")', r'\1../../', text)
        with open(os.path.join(stem, name, 'mod.rs'), 'w') as f:
            f.write(text + '\n')
        test_names.append(name)
    for seg in decls:
        pre += [''] + [l for l in seg if l.strip()]
    # Paths in #[path] and include! are relative to the file's directory, one level up now.
    pre = [re.sub(r'^(#\[path\s*=\s*"|include!\(\s*")', r'\1../', l) for l in pre]
    while pre and pre[-1] == '': pre.pop()
    tail = [''] + [f'mod {n};' for n in names] + [f'#[cfg(test)]\nmod {n};' for n in test_names] \
        + [f'pub use {n}::*;' for n in names]
    with open(os.path.join(stem, 'mod.rs'), 'w') as f:
        f.write('\n'.join(pre + tail) + '\n')
    os.remove(path)
    return stem, names, test_names


def cargo(crate):
    r = subprocess.run(['cargo', 'check', '--offline', '-p', crate, '--tests', '--message-format=json'],
                       cwd=CRATES, capture_output=True, text=True)
    msgs = []
    for line in r.stdout.splitlines():
        try:
            m = json.loads(line)
        except ValueError:
            continue
        if m.get('reason') == 'compiler-message':
            msg = m['message']
            spans = [s for s in msg.get('spans', []) if s.get('is_primary')]
            msgs.append((msg['level'], msg['message'], spans[0]['file_name'] if spans else '',
                         spans[0]['line_start'] if spans else 0))
    return msgs


def files_of(stem):
    out = []
    for root, _, fs in os.walk(stem):
        out += [os.path.join(root, f) for f in fs if f.endswith('.rs')]
    return out


def widen_item(stem, name, vis='pub(super)'):
    pat = re.compile(rf'^((?:(?:const|unsafe|async|extern "C") )*)(fn|struct|enum|union|trait|const|static|type) {re.escape(name)}\b', re.M)
    pat_super = re.compile(rf'^pub\(in super::super\) ((?:(?:const|unsafe|async) )*(?:fn|struct|enum|union|trait|const|static|type) {re.escape(name)}\b)', re.M)
    hit = False
    for p in files_of(stem):
        s = open(p).read()
        t = pat.sub(lambda m: f'{vis} {m.group(1)}{m.group(2)} {name}', s)
        if t == s and vis == 'pub(crate)':
            t = pat_super.sub(r'pub(crate) \1', s)
        if t != s:
            open(p, 'w').write(t); hit = True
    return hit


def widen_fields(stem, struct):
    hit = False
    for p in files_of(stem):
        lines = open(p).read().split('\n'); changed = False
        k = 0
        while k < len(lines):
            l = lines[k]
            m = re.match(rf'^(?:pub(?:\([^)]*\))? )?struct {re.escape(struct)}\b(.*)$', l)
            if m:
                rest = m.group(1)
                if rest.rstrip().endswith('{'):
                    j = k + 1
                    while j < len(lines) and lines[j] != '}':
                        t = re.sub(r'^    (?!pub\b)(r#)?(\w+:)', r'    pub(super) \1\2', lines[j])
                        if t != lines[j]: lines[j] = t; changed = True
                        j += 1
                elif '(' in rest:
                    t = re.sub(r'([(,]\s*)(?!pub\b)([A-Za-z_&\[(])', r'\1pub(super) \2', l, count=0) if rest.lstrip().startswith(('(', '<')) else l
                    if t != l: lines[k] = t; changed = True
            k += 1
        if changed:
            open(p, 'w').write('\n'.join(lines)); hit = True
    return hit


def widen_method(stem, method):
    hit = False
    pat = re.compile(rf'^(    )((?:(?:const|unsafe|async) )*fn {re.escape(method)}\b)', re.M)
    for p in files_of(stem):
        s = open(p).read()
        t = pat.sub(r'\1pub(super) \2', s)
        if t != s: open(p, 'w').write(t); hit = True
    return hit


def regen_globs(stem, names):
    mod = os.path.join(stem, 'mod.rs')
    lines = open(mod).read().split('\n')
    for n in names:
        text = open(os.path.join(stem, n + '.rs')).read()
        if re.search(r'^pub (?!\(|use )', text, re.M) or re.search(r'^pub (?:const |unsafe |async )*(?:fn|struct|enum|trait|type|const|static|union|mod) ', text, re.M):
            vis = 'pub '
        elif re.search(r'^pub\(crate\) ', text, re.M):
            vis = 'pub(crate) '
        else:
            vis = ''
        for i, l in enumerate(lines):
            if re.match(rf'^(?:pub(?:\(crate\))? )?use {n}::\*;$', l):
                lines[i] = f'{vis}use {n}::*;'
    open(mod, 'w').write('\n'.join(lines))


def repair(crate, stem, names, limit=40):
    mod = os.path.join(stem, 'mod.rs')
    for _ in range(limit):
        regen_globs(stem, names)
        msgs = cargo(crate)
        errors = [m for m in msgs if m[0] == 'error']
        warns = [m for m in msgs if m[0] == 'warning' and os.path.abspath(os.path.join(CRATES, m[2])).startswith(os.path.abspath(stem))]
        progress = False
        for _, text, fname, line in errors + warns:
            m = re.search(r'cannot find (?:function|type|value|struct, variant or union type|trait|constant|associated type) `(\w+)`', text) \
                or re.search(r'^(?:function|type|struct|enum|constant|static|trait|type alias|union) `(\w+)` is private', text) \
                or re.search(r'^(?:function|struct|enum|constant|type alias) import `(\w+)` is private', text) \
                or re.search(r'^type `(?:[\w:]+::)?(\w+)` is more private than', text)
            if m:
                vis = 'pub(crate)' if 'import' in text else 'pub(super)'
                progress |= widen_item(stem, m.group(1), vis)
                continue
            m = re.search(r'fields? `\w+`.*? of struct `(?:[\w:]+::)?(\w+)` (?:is|are) private', text) \
                or re.search(r'cannot initialize a tuple struct which contains private fields', text) and re.search(r'`(\w+)`', text)
            if m:
                progress |= widen_fields(stem, m.group(1)); continue
            m = re.search(r'(?:method|associated function) `(\w+)` is private', text)
            if m:
                progress |= widen_method(stem, m.group(1)); continue
        if not errors and not progress:
            return [m for m in msgs if m[0] in ('error', 'warning') and os.path.abspath(os.path.join(CRATES, m[2])).startswith(os.path.abspath(stem))]
        if not progress:
            for e in errors[:15]: print('unresolved:', e)
            raise SystemExit('could not repair automatically')
    raise SystemExit('repair loop limit')


def norm_lines(text):
    out = []
    for l in text.split('\n'):
        l = l.strip()
        l = l.replace('pub(in super::super) ', '').replace('pub(super) ', '').replace('pub(crate) ', '')
        l = re.sub(r'(include_(?:str|bytes)!\(\s*")(?:\.\./)+', r'\1', l)
        if not l or l in ('use super::*;', '#[cfg(test)]', '}') or re.match(r'^(pub )?(use|mod) \w+(::\*)?;$', l) \
                or re.match(r'^mod \w+ \{$', l):
            continue
        out.append(l)
    return Counter(out)


if __name__ == '__main__':
    args = sys.argv[1:]
    target = 600
    if '--target' in args:
        i = args.index('--target'); target = int(args[i + 1]); del args[i:i + 2]
    crate_dir, rel = args[0], args[1]
    CRATES = os.path.abspath(os.path.join(crate_dir, '..'))
    crate = re.search(r'^name\s*=\s*"([^"]+)"', open(os.path.join(crate_dir, 'Cargo.toml')).read(), re.M).group(1)
    path = os.path.join(crate_dir, rel)
    if os.path.basename(path) in ('lib.rs', 'mod.rs', 'main.rs'):
        raise SystemExit('refusing to split a crate or module root; split its children instead')
    original = open(path).read()
    stem, names, tests = split(path, target)
    left = repair(crate, stem, names)
    new = ''.join(open(p).read() + '\n' for p in sorted(files_of(stem)))
    a, b = norm_lines(original), norm_lines(new)
    diff = (a - b) + (b - a)
    print(json.dumps({'crate': crate, 'module': os.path.relpath(stem), 'submodules': names, 'tests': tests,
                      'sizes': {os.path.relpath(p, stem): open(p).read().count('\n') for p in sorted(files_of(stem))},
                      'multiset_diff': sum(diff.values()), 'diff_sample': list(diff)[:8],
                      'remaining': [f'{m[0]}: {m[1]} ({m[2]}:{m[3]})' for m in left]}, indent=1))
