#!/usr/bin/env python3
"""Render docs/refactor/inventory/*.md from the audit data. usage: gen_inventory.py <audit_dir> <out_dir>"""
import csv, sys, os, json, collections
A, OUT = sys.argv[1], sys.argv[2]
os.makedirs(OUT, exist_ok=True)
def rows(n): return list(csv.DictReader(open(os.path.join(A, n), newline=''), delimiter='\t', quoting=csv.QUOTE_NONE))
J = lambda n: json.load(open(os.path.join(A, n)))
files, fns = rows('files.tsv'), rows('functions.tsv')
dup = {r['file']: r for r in J('dupes-rs.json')['per_file']}
clippy = J('clippy-by-file.json'); graph = J('graph.json'); fw = J('forcewarn-summary.json')
churn = {}
for l in open(os.path.join(A, 'churn60.txt')):
    n, p = l.split(None, 1); churn[p.strip()] = int(n)
dead = collections.Counter()
for i in fw['items']: dead['crates/' + i['file']] += i['n']
cyc = collections.defaultdict(dict)
for c, g in graph['graphs'].items():
    for k, s in enumerate(g['scc'], 1):
        for m in s['members']: cyc[c][m] = k
top = lambda f: (f['module'].split('::')[0] if f['module'] else '(root)')
M = collections.defaultdict(lambda: collections.Counter())
for f in files:
    if f['kind'] in ('orphan', 'included'): continue
    k = (f['crate'], top(f) if f['kind'] in ('lib', 'bin') else '(tests/examples)')
    m = M[k]; prod = f['kind'] in ('lib', 'bin') and f['test'] == '0'
    m['files'] += 1
    if prod:
        m['loc'] += int(f['loc']); m['pfiles'] += 1
        d = dup.get(f['file'])
        if d: m['sig'] += d['sig']; m['dup'] += d['dup']
        m['clippy'] += clippy.get(f['file'], 0); m['dead'] += dead.get(f['file'], 0)
    else: m['tloc'] += int(f['loc'])
    m['churn'] += churn.get(f['file'], 0)
fidx = {f['file']: f for f in files}
for f in fns:
    fi = fidx[f['file']]
    if fi['kind'] not in ('lib', 'bin') or f['test'] == '1': continue
    m = M[(f['crate'], top(fi))]
    m['fns'] += 1; cg = int(f['cognitive']); m['cog'] += cg; m['maxcog'] = max(m['maxcog'], cg)
    if int(f['loc']) > 100: m['long'] += 1
    if int(f['cyclo']) > 15: m['cy15'] += 1
    if int(f['params']) > 5: m['par'] += 1
    m['unwrap'] += int(f['unwraps']); m['alloc'] += int(f['alloc_in_loop'])
scored = sorted(((m['churn'] * m['cog'], k) for k, m in M.items() if k[1] != '(tests/examples)' and m['loc']), reverse=True)
n = len(scored); prio = {}
for i, (s, k) in enumerate(scored): prio[k] = 'P1' if i < n * 0.10 and s > 0 else 'P2' if i < n * 0.35 and s > 0 else 'P3'
HEAD = "| Модуль | LOC | Файлов | Fn | max cog | Fn >100 | cy >15 | par >5 | unwrap | Клоны % | Churn 60д | Clippy | Dead | Цикл | Приоритет | Статус |\n|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|:--:|:--:|---|\n"
def table(crate, limit=None):
    ks = sorted([k for k in M if k[0] == crate and k[1] != '(tests/examples)'], key=lambda k: -M[k]['loc'])
    out = HEAD
    for k in ks[:limit]:
        m = M[k]; d = f"{100 * m['dup'] / m['sig']:.0f}" if m['sig'] else '0'
        c = cyc[crate].get(k[1]); out += f"| `{k[1]}` | {m['loc']} | {m['pfiles']} | {m['fns']} | {m['maxcog']} | {m['long']} | {m['cy15']} | {m['par']} | {m['unwrap']} | {d} | {m['churn']} | {m['clippy']} | {m['dead']} | {('C' + str(c)) if c else ''} | {prio.get(k, 'P3')} | todo |\n"
    return out, len(ks)
LEGEND = ("Столбцы: LOC и Fn только по продакшен-коду (без `#[cfg(test)]`, tests/, examples/). `max cog` это максимальная когнитивная сложность функции в модуле. "
          "`Клоны %` это доля значимых строк, входящих в точные дубли длиной от 30 строк. `Churn 60д` это число коммитов за 60 дней по файлам модуля. "
          "`Dead` это элементы, которые rustc считает мёртвыми при `--force-warn dead_code`. `Цикл` это номер группы циклических зависимостей (SCC) между модулями верхнего уровня. "
          "Приоритет считается как churn x суммарная когнитивная сложность: верхние 10% это P1, следующие 25% это P2.\n\n")
crates = sorted({k[0] for k in M})
big = ['brep-core', 'nurbs-core', 'geometry-bridge', 'cad-predicates']
for c in big:
    t, n_ = table(c)
    g = graph['graphs'].get(c, {})
    head = f"# Инвентаризация: `{c}`\n\nМодулей верхнего уровня: {n_}. Модулей в циклах: {g.get('modules_in_cycles', 0)} ({g.get('loc_in_cycles', 0)} LOC из {g.get('loc', 0)}).\n\n{LEGEND}"
    open(os.path.join(OUT, f'{c}.md'), 'w').write(head + t)
body = "# Инвентаризация: остальные крейты\n\n" + LEGEND
for c in crates:
    if c in big: continue
    t, n_ = table(c)
    if n_: body += f"## `{c}`\n\n{t}\n"
open(os.path.join(OUT, 'other-crates.md'), 'w').write(body)
# worst functions
prod = [f for f in fns if fidx[f['file']]['kind'] in ('lib', 'bin') and f['test'] == '0']
def ftable(key, n_):
    out = "| # | Функция | Файл:строка | LOC | cog | cyclo | nest | params | alloc в цикле |\n|---:|---|---|---:|---:|---:|---:|---:|---:|\n"
    for i, f in enumerate(sorted(prod, key=key, reverse=True)[:n_], 1):
        name = (f['ctx'] + '::' if f['ctx'] else '') + f['name']
        out += f"| {i} | `{name}` | `{f['file']}:{f['line']}` | {f['loc']} | {f['cognitive']} | {f['cyclo']} | {f['max_nest']} | {f['params']} | {f['alloc_in_loop']} |\n"
    return out
w = "# Худшие функции (Rust, продакшен-код)\n\nИсточник: `tools/refactor-audit`. Всего измерено функций: %d, из них продакшен: %d.\n\n" % (len(fns), len(prod))
w += "## Топ-120 по когнитивной сложности\n\n" + ftable(lambda f: int(f['cognitive']), 120)
w += "\n## Топ-60 по длине\n\n" + ftable(lambda f: int(f['loc']), 60)
w += "\n## Топ-40 по числу параметров\n\n" + ftable(lambda f: (int(f['params']), int(f['loc'])), 40)
w += "\n## Топ-40 по аллокациям внутри циклов (кандидаты на оптимизацию, требуют профиля)\n\n" + ftable(lambda f: int(f['alloc_in_loop']), 40)
open(os.path.join(OUT, 'worst-functions-rust.md'), 'w').write(w)
print("modules:", len([k for k in M if k[1] != '(tests/examples)']), "P1:", sum(1 for v in prio.values() if v == 'P1'), "P2:", sum(1 for v in prio.values() if v == 'P2'))
print("top P1:", [f"{k[0]}::{k[1]}" for s, k in scored[:25]])
json.dump({f"{k[0]}::{k[1]}": dict(M[k], prio=prio.get(k, 'P3')) for k in M}, open(os.path.join(A, 'modules.json'), 'w'))
