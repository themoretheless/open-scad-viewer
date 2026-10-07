#!/usr/bin/env python3
# -*- coding: utf-8 -*-
import re, sys

path = "nurbs-rust-200.md"
text = open(path, encoding="utf-8").read()
lines = text.split("\n")

item_re = re.compile(r"^(\d+)\. \*\*")
stub_re = re.compile(r"объединено с п\. ([0-9, ]+)\.")

# 1. Parse items
items = {}  # num -> line index
order = []
for i, ln in enumerate(lines):
    m = item_re.match(ln)
    if m:
        n = int(m.group(1))
        items[n] = i
        order.append(n)
assert len(order) == 1000, len(order)

# 2. Stub -> canonical (first listed number)
stub2canon = {}
for n in order:
    m = stub_re.search(lines[items[n]])
    if m:
        nums = [int(x) for x in re.findall(r"\d+", m.group(1))]
        stub2canon[n] = nums[0]

stubs = set(stub2canon)
kept = [n for n in order if n not in stubs]
assert len(kept) == 931, len(kept)

# sanity: canonical targets must themselves be kept (resolve transitively just in case)
def canon(n):
    while n in stub2canon:
        n = stub2canon[n]
    return n

# 3. old -> new mapping
newnum = {}
for idx, n in enumerate(kept, 1):
    newnum[n] = idx
def mapnum(n):
    return newnum[canon(n)]

# 4. Build new lines: drop stub lines
stub_lines = {items[n] for n in stubs}
out = [ln for i, ln in enumerate(lines) if i not in stub_lines]

# 5. Renumber items
def renumber_item(m):
    n = int(m.group(1))
    return f"{mapnum(n)}. **"
out = [item_re.sub(renumber_item, ln) if item_re.match(ln) else ln for ln in out]

# 6. Rewrite references "п. ..." / "пп. ..." (numbers, commas, ranges with –,—,-)
ref_re = re.compile(r"(пп?\.\s*)([0-9][0-9,\s]*(?:[–—-]\s*[0-9]+)?(?:,\s*[0-9]+(?:[–—-]\s*[0-9]+)?)*)")

def repl_ref(m):
    prefix, body = m.group(1), m.group(2)
    # replace each number; ranges: map both endpoints
    def numrep(nm):
        return str(mapnum(int(nm.group(0))))
    return prefix + re.sub(r"\d+", numrep, body)

out = [ref_re.sub(repl_ref, ln) for ln in out]

# 7. Headers: recompute ranges in ## / ### lines with (a–b)
# First pass: know new numbering; for each header line, find range of following items until next header of same-or-higher level.
result = []
n = len(out)
i = 0
hdr_re = re.compile(r"^(#{2,3}) (.*?) \((\d+)–(\d+)\)\s*$")
while i < n:
    ln = out[i]
    hm = hdr_re.match(ln)
    if hm:
        level = len(hm.group(1))
        # scan ahead for items until next header with level <= this level
        nums = []
        j = i + 1
        while j < n:
            l2 = out[j]
            if re.match(r"^#{1,%d} " % level, l2) and re.match(r"^#", l2):
                # header of same or higher rank (fewer #'s)
                if len(l2) - len(l2.lstrip("#")) <= level:
                    break
            m2 = item_re.match(l2)
            if m2:
                nums.append(int(m2.group(1)))
            j += 1
        if nums:
            ln = f"{hm.group(1)} {hm.group(2)} ({nums[0]}–{nums[-1]})"
        else:
            sys.stderr.write(f"WARN: no items under header: {ln}\n")
        result.append(ln)
    else:
        result.append(ln)
    i += 1

# 8. Title
result[0] = "# NURBS в Rust: 931 ключевая возможность"

open(path, "w", encoding="utf-8").write("\n".join(result))
print("stubs removed:", len(stubs))
print("kept:", len(kept))
print("stub->canon:", sorted(stub2canon.items()))
