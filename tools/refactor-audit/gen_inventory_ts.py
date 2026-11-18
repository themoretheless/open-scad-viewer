#!/usr/bin/env python3
"""Render docs/refactor/inventory/typescript.md. usage: gen_inventory_ts.py <audit_dir> <out_dir>"""
import json, sys, os, collections
A, OUT = sys.argv[1], sys.argv[2]
J = lambda n: json.load(open(os.path.join(A, n)))
d = J('ts.json'); reach = J('ts-reach.json'); dup = J('dupes-ts-src.json')
status = {}
for l in open(os.path.join(A, 'gitstatus.txt')): status[l[3:].strip()] = l[:2]
churn = {}
for l in open(os.path.join(A, 'churn60.txt')):
    n, p = l.split(None, 1); churn[p.strip()] = int(n)
dupf = {r['file']: r for r in dup['per_file']}
def area(p):
    s = p.split('/')
    if s[0] != 'src': return s[0]
    if len(s) == 2: return 'src/(root)'
    if s[1] in ('services', 'features', 'mcp') and len(s) > 3: return '/'.join(s[:3])
    return '/'.join(s[:2]) + ('/(files)' if s[1] in ('services', 'features', 'mcp') else '')
M = collections.defaultdict(collections.Counter)
for f in d['files']:
    if not f['file'].startswith('src/'): continue
    m = M[area(f['file'])]; m['files'] += 1; m['loc'] += f['loc']; m['any'] += f['any']; m['as'] += f['asCasts']; m['nn'] += f['nonNull']
    m['over'] += f['loc'] > (400 if f['file'].endswith('.svt') else 500); m['churn'] += churn.get(f['file'], 0)
    x = dupf.get(f['file'])
    if x: m['sig'] += x['sig']; m['dup'] += x['dup']
for n in d['fns']:
    if not n['file'].startswith('src/'): continue
    m = M[area(n['file'])]; m['fns'] += 1; m['cog'] += n['cognitive']; m['maxcog'] = max(m['maxcog'], n['cognitive'])
    m['long'] += n['loc'] > 100; m['cy15'] += n['cyclo'] > 15; m['par'] += n['params'] > 4
sc = sorted(((m['churn'] * m['cog'], k) for k, m in M.items()), reverse=True); N = len(sc)
prio = {k: ('P1' if i < N * .10 and s else 'P2' if i < N * .35 and s else 'P3') for i, (s, k) in enumerate(sc)}
o = "# Инвентаризация: TypeScript и Svet (`src/`)\n\nИсточник: `tools/refactor-audit/audit-ts.mjs` (TypeScript compiler API, без проверки типов). Для `.svt` анализируется блок `<script>`.\n\n"
o += "Столбцы: `as` это приведения типов, `!` это non-null assertion, `>лимит` это файлы длиннее лимита архитектурного аудита (500 строк для `.ts`, 400 для `.svt`). Клоны считаются от 20 значимых строк.\n\n"
o += "| Каталог | Файлов | LOC | Fn | max cog | Fn >100 | cy >15 | par >4 | any | as | ! | >лимит | Клоны % | Churn 60д | Приоритет | Статус |\n|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|:--:|---|\n"
for k in sorted(M, key=lambda k: -M[k]['loc']):
    m = M[k]; dp = f"{100 * m['dup'] / m['sig']:.0f}" if m['sig'] else '0'
    o += f"| `{k}` | {m['files']} | {m['loc']} | {m['fns']} | {m['maxcog']} | {m['long']} | {m['cy15']} | {m['par']} | {m['any']} | {m['as']} | {m['nn']} | {m['over']} | {dp} | {m['churn']} | {prio[k]} | todo |\n"
src = [n for n in d['fns'] if n['file'].startswith('src/')]
o += "\n## Топ-60 функций по когнитивной сложности\n\n| # | Функция | Файл:строка | LOC | cog | cyclo | nest | params |\n|---:|---|---|---:|---:|---:|---:|---:|\n"
for i, n in enumerate(sorted(src, key=lambda n: -n['cognitive'])[:60], 1):
    o += f"| {i} | `{(n['cls'] + '.' if n['cls'] else '') + n['name']}` | `{n['file']}:{n['line']}` | {n['loc']} | {n['cognitive']} | {n['cyclo']} | {n['maxNest']} | {n['params']} |\n"
F = {f['file']: f for f in d['files']}
def lst(title, names, note):
    out = f"\n## {title}\n\n{note}\n\n| Файл | LOC | Состояние в git |\n|---|---:|---|\n"
    for p in sorted(names, key=lambda p: -F[p]['loc']):
        st = status.get(p, ''); out += f"| `{p}` | {F[p]['loc']} | {'не закоммичен (текущая работа)' if st.strip() else 'в индексе, без изменений'} |\n"
    return out
o += lst(f"Модули, недостижимые ни из одной точки входа ({len(reach['dead'])} файлов)", reach['dead'], "Не импортируются ни приложением (`src/main.ts`), ни MCP-сервером (`src/mcp/server.ts`), ни воркерами, ни скриптами, ни тестами. Учитываются статические и динамические импорты, реэкспорты, `new URL(..., import.meta.url)` и `import()`-типы.")
o += lst(f"Модули, достижимые только из тестов ({len(reach['test_only'])} файлов)", reach['test_only'], "Продакшен-код их не использует. Либо это заготовки, либо мёртвый код, который держат живым тесты.")
open(os.path.join(OUT, 'typescript.md'), 'w').write(o)
print("areas:", len(M), "P1:", [k for k, v in prio.items() if v == 'P1'])
