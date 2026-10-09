#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Обновляет ссылки на пункты чек-листа в doc-комментариях nurbs-core."""
import re, subprocess

# --- восстанавливаем мэппинг из pre-renumber копии ---
bak = open("/tmp/nurbs-rust-200.md.pre-renumber", encoding="utf-8").read().split("\n")
item_re = re.compile(r"^(\d+)\. \*\*")
stub_re = re.compile(r"объединено с п\. ([0-9, ]+)\.")
order, stub2canon = [], {}
for ln in bak:
    m = item_re.match(ln)
    if m:
        n = int(m.group(1)); order.append(n)
        s = stub_re.search(ln)
        if s:
            stub2canon[n] = int(re.findall(r"\d+", s.group(1))[0])
def canon(n):
    while n in stub2canon:
        n = stub2canon[n]
    return n
newnum = {n: i + 1 for i, n in enumerate(n for n in order if n not in stub2canon)}
mapnum = lambda n: newnum[canon(n)]

files = subprocess.check_output(
    ["grep", "-rl", "-E", "items? [0-9]|checklist [0-9]|checklist items",
     "crates/nurbs-core/src"], text=True).split()

# keyword, optional line-wrap ("...\n//! "), then number list
ref_re = re.compile(
    r"\b(items?|checklist(?:\s+items?)?)(\s*\n\s*//[!/]?\s*|\s+)"
    r"([0-9]+(?:[–—-]\s*[0-9]+)?(?:,\s*[0-9]+(?:[–—-]\s*[0-9]+)?)*)")

changed = {}
for f in files:
    src = open(f, encoding="utf-8").read()
    def repl(m):
        body = re.sub(r"\d+", lambda nm: str(mapnum(int(nm.group(0)))), m.group(3))
        return m.group(1) + m.group(2) + body
    new = ref_re.sub(repl, src)
    if new != src:
        open(f, "w", encoding="utf-8").write(new)
        changed[f] = len(ref_re.findall(src))
for f, c in sorted(changed.items()):
    print(f"{f}: {c} refs")
print("files changed:", len(changed))
