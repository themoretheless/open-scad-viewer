#!/usr/bin/env python3
"""Render docs/refactor/inventory/duplicates.md. usage: gen_duplicates.py <audit_dir> <out_dir>"""
import json, sys, os, csv, collections
A, OUT = sys.argv[1], sys.argv[2]
J = lambda n: json.load(open(os.path.join(A, n)))
d = J('dupes-rs.json'); dec = J('orphan-decision.json'); dt = J('duptests.json'); ts = J('dupes-ts-src.json')
files = {r['file']: r for r in csv.DictReader(open(os.path.join(A, 'files.tsv')), delimiter='\t')}
def live(f):
    k = files[f]['kind']
    return 'не компилируется' if k in ('orphan', 'included') else ('тест' if (k in ('test', 'bench') or files[f]['test'] == '1') else 'пример' if k == 'example' else 'прод')
o = "# Дубликаты и мёртвые копии\n\nИсточник: `tools/refactor-audit/dupes.py` (точные клоны: совпадение нормализованных значимых строк, окно от 30 строк для Rust и от 20 для TypeScript) и разбор дерева модулей `tools/refactor-audit/audit-rs`.\n\n"
o += f"## Rust: сводка\n\n- Значимых строк: {d['sig_lines']}. В точных клонах от 30 строк: {d['dup_lines']} ({100 * d['dup_lines'] / d['sig_lines']:.1f}%).\n- Наборов полностью идентичных файлов: {d['identical_file_sets']}.\n\n"
o += "### Клоны по крейтам\n\n| Крейт | Строк в клонах | Значимых строк | % |\n|---|---:|---:|---:|\n"
for k, v in sorted(d['groups'].items(), key=lambda kv: -kv[1]['dup']):
    if v['dup']: o += f"| `{k}` | {v['dup']} | {v['sig']} | {v['pct']} |\n"
o += "\n### Масса клонов между крейтами\n\n| A | B | Строк |\n|---|---|---:|\n"
for p in d['pair_groups'][:20]: o += f"| `{p['a']}` | `{p['b']}` | {p['lines']} |\n"
o += "\n### 80 крупнейших клонов\n\n| Строк | Копия A | Копия B |\n|---:|---|---|\n"
for t in d['top'][:80]:
    o += f"| {t['lines']} | `{t['a']}:{t['a_line']}` ({live(t['a'])}) | `{t['b']}:{t['b_line']}` ({live(t['b'])}) |\n"
o += "\n## Rust: функции, текстуально идентичные в двух компилируемых файлах\n\nСравнение полного текста функции без пробелов. Тесты из этого списка выполняются дважды.\n\n| Идентичных fn | из них в тестовом коде | LOC | Файл A | Файл B |\n|---:|---:|---:|---|---|\n"
for p in dt['pairs'][:45]: o += f"| {p['n']} | {p['tests']} | {p['loc']} | `{p['a']}` | `{p['b']}` |\n"
o += f"\n## Rust: некомпилируемые файлы\n\nФайл считается некомпилируемым, если он не достижим из корня ни одной цели Cargo (lib, bin, test, example, bench) через `mod`, `#[path]` или `include!`.\n\n"
o += f"### Доказуемо устаревшие копии, кандидаты на удаление ({len(dec['delete'])} файлов, {sum(x['loc'] for x in dec['delete'])} LOC)\n\nКритерии: файл в индексе git и не изменён; не менее 80% его строк есть в компилируемом файле того же крейта; каждое имя функции из него существует в живом коде крейта; путь к нему не упоминается в отслеживаемых не-Rust файлах; он не дочерний модуль оставленного файла. Удалённое ранее перечислено в `removed-*.md`.\n\n| Файл | LOC | Доля строк в живом файле | Живой двойник |\n|---|---:|---:|---|\n"
for x in sorted(dec['delete'], key=lambda x: (x['crate'], -x['loc'])): o += f"| `{x['file']}` | {x['loc']} | {x['sim']:.2f} | `{x['twin']}` |\n"
keep = [x for x in dec['keep'] if x['crate'] not in ('independent-modeling',)]
o += f"\n### Оставлено до решения владельца ({len(keep)} файлов, {sum(x['loc'] for x in keep)} LOC)\n\n| Файл | LOC | Почему не удалён автоматически |\n|---|---:|---|\n"
for x in sorted(keep, key=lambda x: (x['crate'], -x['loc'])): o += f"| `{x['file']}` | {x['loc']} | {'; '.join(x['reasons'])} |\n"
o += f"\n## TypeScript (`src/`)\n\n- Значимых строк: {ts['sig_lines']}. В точных клонах от 20 строк: {ts['dup_lines']} ({100 * ts['dup_lines'] / ts['sig_lines']:.1f}%).\n\n| Каталог | Строк в клонах | Значимых строк | % |\n|---|---:|---:|---:|\n"
for k, v in sorted(ts['groups'].items(), key=lambda kv: -kv[1]['dup']):
    if v['dup']: o += f"| `{k}` | {v['dup']} | {v['sig']} | {v['pct']} |\n"
o += "\n### 40 крупнейших клонов\n\n| Строк | Копия A | Копия B |\n|---:|---|---|\n"
for t in ts['top'][:40]: o += f"| {t['lines']} | `{t['a']}:{t['a_line']}` | `{t['b']}:{t['b_line']}` |\n"
open(os.path.join(OUT, 'duplicates.md'), 'w').write(o)
print("ok", len(dec['delete']), len(keep))
