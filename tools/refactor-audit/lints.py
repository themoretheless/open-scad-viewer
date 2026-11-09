#!/usr/bin/env python3
"""Summarize `cargo clippy --message-format=json` and a `--force-warn dead_code` check. usage: lints.py <out_dir> <audit_dir>"""
import json, sys, os, collections, re
OUT, A = sys.argv[1], sys.argv[2]
def diagnostics(path):
    seen = set()
    if not os.path.exists(path): return
    for line in open(path, encoding='utf-8', errors='replace'):
        try: j = json.loads(line)
        except ValueError: continue
        m = j.get('message') if j.get('reason') == 'compiler-message' else None
        if not m or not m.get('spans') or m.get('level') not in ('warning', 'error'): continue
        sp = next((s for s in m['spans'] if s['is_primary']), m['spans'][0])
        code = (m.get('code') or {}).get('code') or '(no code)'
        key = (code, sp['file_name'], sp['line_start'], sp['column_start'])
        if key in seen: continue
        seen.add(key); yield code, sp['file_name'], sp['line_start'], m
by_lint = collections.Counter(); by_crate = collections.Counter(); by_file = collections.Counter(); lint_crate = collections.defaultdict(collections.Counter)
for code, file, line, m in diagnostics(os.path.join(OUT, 'clippy.json')):
    crate = file.split('/')[0]; by_lint[code] += 1; by_crate[crate] += 1; by_file['crates/' + file] += 1; lint_crate[code][crate] += 1
json.dump(dict(total=sum(by_lint.values()), byLint=by_lint.most_common(), byCrate=by_crate.most_common(), lintCrate=lint_crate), open(os.path.join(A, 'clippy-summary.json'), 'w'), indent=1)
json.dump(by_file, open(os.path.join(A, 'clippy-by-file.json'), 'w'))
by = collections.defaultdict(collections.Counter); items = []
for code, file, line, m in diagnostics(os.path.join(OUT, 'forcewarn.json')):
    if not re.match(r'^[a-z-]+/src/', file): continue
    by[code][file.split('/')[0]] += 1
    if code == 'dead_code': items.append(dict(crate=file.split('/')[0], file=file, line=line, msg=m['message'][:110], n=len(m['spans'])))
json.dump(dict(by=by, items=items), open(os.path.join(A, 'forcewarn-summary.json'), 'w'), indent=1)
print(f"clippy diagnostics: {sum(by_lint.values())} in {len(by_file)} files; dead_code items: {sum(i['n'] for i in items)}")
