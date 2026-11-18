#!/usr/bin/env python3
import csv, sys, os, re
A, a, b = sys.argv[1], sys.argv[2], sys.argv[3]
fns = [f for f in csv.DictReader(open(os.path.join(A, 'functions.tsv')), delimiter='\t', quoting=csv.QUOTE_NONE) if f['file'] in (a, b)]
def text(f):
    L = open(f['file'], encoding='utf-8').read().split('\n'); s = int(f['line']) - 1
    return re.sub(r'\s+', '', '\n'.join(L[s:s + int(f['loc'])]))
def is_test(f):
    L = open(f['file'], encoding='utf-8').read().split('\n'); s = int(f['line']) - 1
    return any('#[test]' in L[i] for i in range(max(0, s - 4), s))
A_ = {f['name']: f for f in fns if f['file'] == a}; B_ = {f['name']: f for f in fns if f['file'] == b}
print(f"A = {a} ({len(A_)} fns)\nB = {b} ({len(B_)} fns)")
for n, f in sorted(A_.items(), key=lambda kv: int(kv[1]['line'])):
    if n in B_:
        st = 'IDENTICAL' if text(f) == text(B_[n]) else f"DIFFERS (A {f['loc']} lines, B {B_[n]['loc']} lines)"
    else: st = 'ONLY IN A'
    print(f"  A:{f['line']:>5} {'#[test]' if is_test(f) else 'helper '} {st:34s} {n}")
for n, f in sorted(B_.items(), key=lambda kv: int(kv[1]['line'])):
    if n not in A_: print(f"  B:{f['line']:>5} {'#[test]' if is_test(f) else 'helper '} ONLY IN B                          {n}")
