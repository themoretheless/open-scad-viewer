#!/usr/bin/env python3
"""Render docs/refactor/INVENTORY.md (crate-level table + index). usage: gen_inventory_index.py <audit_dir> <out_file>"""
import json, sys, os, collections, subprocess, datetime
A, OUT = sys.argv[1], sys.argv[2]
J = lambda n: json.load(open(os.path.join(A, n)))
s = J('summary.json'); g = J('graph.json'); d = J('dupes-rs.json'); mods = J('modules.json'); CS = J('clippy-summary.json'); cl = dict(CS['byCrate'])
ts = J('ts.json'); T = s['total']
HEAD = subprocess.run(['git', 'rev-parse', '--short', 'HEAD'], capture_output=True, text=True).stdout.strip()
NMOD = collections.Counter(k.split('::')[0] for k, v in mods.items() if not k.endswith('(tests/examples)') and v.get('loc'))
churn = collections.Counter()
for l in open(os.path.join(A, 'churn60.txt')):
    n, p = l.split(None, 1); q = p.strip().split('/')
    if q[0] == 'crates' and len(q) > 2: churn[q[1]] += int(n)
LAYER = {'value-codec': 0, 'cad-predicates': 0, 'vr-core': 0, 'wasm-brotli': 0, 'bridge-codec': 0, 'mesh-topology': 1, 'mesh-query': 1, 'mesh-section': 1, 'mesh-io': 1,
         'geometry-ops': 2, 'planar-geometry': 2, 'polygon-core': 2, 'nurbs-core': 2, 'nurbs-intersect': 2, 'sdf-core': 2, 'subdivision-core': 2, 'sketch-core': 2, 'brep-topology': 2, 'manifold-core': 2, 'manifold-ops': 2,
         'brep-core': 3, 'cad-step': 4, 'brep-intersect': 4, 'rush-runtime': 4, 'rush-frontend': 4, 'openscad-core': 4, 'mechanical-core': 4, 'mechanics-core': 4, 'gcode-core': 4, 'gcode-optimize': 4, 'slicer-core': 4,
         'laser-core': 4, 'printer-core': 4, 'photogrammetry-core': 4, 'bridge-cam': 5, 'bridge-analysis': 5, 'bridge-svg': 5, 'geometry-bridge': 5, 'languages-bridge': 5, 'photogrammetry-ffi': 5, 'printer-cli': 5,
         'geometry-wasm': 6, 'languages-wasm': 6, 'photogrammetry-wasm': 6}
o = """# Инвентаризация кодовой базы

Дата среза: {DATE}, рабочее дерево на коммите `{HEAD}`. Все числа получены инструментами из `tools/refactor-audit` и воспроизводимы одной командой (см. `tools/refactor-audit/README.md`).

Что измерено:

- Rust: каждый из {NFILES} файлов `.rs` разобран `syn`, построено дерево модулей каждой цели Cargo, измерена каждая из {NFNS} функций (длина, параметры, цикломатическая и когнитивная сложность, вложенность, `unwrap`/`expect`/`panic`, клонирования, аллокации в циклах, `unsafe`).
- TypeScript и Svet: {TSFILES} файлов, {TSFNS} функций, граф импортов с динамическими импортами и воркерами.
- Дубли: точные клоны по нормализованным строкам; функции, совпадающие текстуально; файлы вне дерева модулей.
- История: число коммитов по каждому файлу за 60 дней.

Чего здесь нет: качественного чтения каждого из {PFNS} продакшен-методов глазами. Это {PKLOC} тыс. строк, такой проход не помещается в одну сессию. Вместо него каждая функция измерена, а разбор по чеклисту идёт в порядке приоритета (см. `PLAN.md`).

## Детальные таблицы

| Файл | Содержимое |
|---|---|
| [inventory/brep-core.md](inventory/brep-core.md) | модули верхнего уровня `brep-core` ({NB}) |
| [inventory/nurbs-core.md](inventory/nurbs-core.md) | модули верхнего уровня `nurbs-core` ({NN}) |
| [inventory/geometry-bridge.md](inventory/geometry-bridge.md) | модули `geometry-bridge` ({NG}) |
| [inventory/cad-predicates.md](inventory/cad-predicates.md) | модули `cad-predicates` ({NC}) |
| [inventory/other-crates.md](inventory/other-crates.md) | модули остальных крейтов |
| [inventory/typescript.md](inventory/typescript.md) | каталоги `src/`, худшие функции, недостижимые модули |
| [inventory/worst-functions-rust.md](inventory/worst-functions-rust.md) | рейтинги функций Rust |
| [inventory/duplicates.md](inventory/duplicates.md) | клоны, дубли тестов, некомпилируемые файлы |

## Крейты

Слой взят из `scripts/audit-architecture.mjs`. `Fn >100` это функции длиннее 100 строк, `cog >25` это функции с когнитивной сложностью выше 25, `Клоны %` это доля строк в точных клонах от 30 строк, `Циклы` это доля продакшен-кода в модулях, входящих в циклические зависимости.

| Крейт | Слой | Прод LOC | Тест LOC | Файлов | Fn | Fn >100 | cog >25 | par >5 | unwrap | Клоны % | Циклы % | Clippy | Churn 60д | Запахи (кратко) | Приоритет | Статус |
|---|:--:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|:--:|---|
"""
for k_, v_ in dict(DATE=datetime.date.today().isoformat(), HEAD=HEAD, NFILES=T['files'], NFNS=T['fns'], TSFILES=len(ts['files']), TSFNS=len(ts['fns']), PFNS=T['prod_fns'], PKLOC=T['prod_loc'] // 1000,
                   NB=NMOD['brep-core'], NN=NMOD['nurbs-core'], NG=NMOD['geometry-bridge'], NC=NMOD['cad-predicates']).items():
    o = o.replace('{' + k_ + '}', str(v_))
notes = {
 'brep-core': 'god crate; 244 плоских модуля; копии кода из cad-step, nurbs-intersect, brep-intersect; тесты-дубли',
 'nurbs-core': 'god crate; один цикл на 90% кода; `#![allow(dead_code, unused_imports)]` на весь крейт',
 'geometry-bridge': '112 модулей-адаптеров; `dispatch` по строкам; 46 `unsafe` в ABI',
 'cad-step': '70% строк это копия `brep-core`',
 'nurbs-intersect': '90% строк это копия `brep-core`',
 'brep-intersect': '66% строк это копия `brep-core::intersections`',
 'cad-predicates': '204 предупреждения clippy; много функций с 6+ параметрами',
 'rush-runtime': '`evaluator_impl_N` механические части',
 'rush-frontend': '`Compiler::operation` cog 451; файлы `compiler_impl_N`, `include!`-части',
 'openscad-core': '`eval_node_cases_N`, `evaluator_2_impl_N` механические части',
 'bridge-svg': '`preflight_inner` 573 строки',
 'sdf-core': '`lib.rs` 1439 строк; `methods_Field_from_value.rs`',
 'sketch-core': 'механические `methods_*`',
 
 'photogrammetry-core': 'цикл из 7 модулей; функции с 19 параметрами',
 'polygon-core': 'хаб: зависит от 13 внутренних крейтов',
 'planar-geometry': 'цикл path/rings/stroke/tessellation',
}
cr = s['crates']
for name, c in sorted(cr.items(), key=lambda kv: -kv[1]['prod_loc']):
    if not c['prod_loc']: continue
    gg = g['graphs'].get(name, {}); dd = d['groups'].get('crates/' + name, {})
    cyc = f"{100 * gg.get('loc_in_cycles', 0) // max(gg.get('loc', 1), 1)}" if gg else '0'
    score = churn[name] * c['cog_gt25']
    pr = 'P1' if name in ('brep-core', 'nurbs-core', 'geometry-bridge', 'cad-step', 'nurbs-intersect', 'brep-intersect') else 'P2' if score > 300 or c['loc_gt100'] >= 10 else 'P3'
    o += f"| `{name}` | {LAYER.get(name, '')} | {c['prod_loc']} | {c['test_loc']} | {c['prod_files']} | {c['prod_fns']} | {c['loc_gt100']} | {c['cog_gt25']} | {c['params_gt5']} | {c['unwraps']} | {dd.get('pct', 0)} | {cyc} | {cl.get(name, 0)} | {churn[name]} | {notes.get(name, '')} | {pr} | todo |\n"
t = s['total']
o += f"| **Итого** | | {t['prod_loc']} | {t['test_loc']} | {t['prod_files']} | {t['prod_fns']} | {t['loc_gt100']} | {t['cog_gt25']} | {t['params_gt5']} | {t['unwraps']} | {100 * d['dup_lines'] / d['sig_lines']:.1f} | | {CS['total']} | | | | |\n"
o += """
## Каталоги вне рабочего пространства Cargo

| Каталог | Состояние |
|---|---|
| `crates/compute-core/shaders` | не Rust: WGSL, которые импортирует `src/services/gpu*.ts`; живые |
| `crates/independent-modeling` | отдельное рабочее пространство по ADR-0010; не трогать |

Каталоги `modelgraph-runtime`, `math-core`, `tensor-core`, `raster-core`, `modelgraph-text` без `Cargo.toml` удалены 2026-11-10 (план, M4).

Статусы: `todo` означает, что модуль измерен и ждёт разбора по чеклисту; изменения этой итерации перечислены в `PLAN.md` и `BASELINE.md`.
"""
open(OUT, 'w').write(o)
print("written", OUT)
