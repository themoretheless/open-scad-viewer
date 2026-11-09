#!/usr/bin/env python3
"""Aggregate audit-rs TSV output into per-crate metrics and ranked findings (JSON + text)."""
import csv, json, sys, statistics, collections, re, os
A = sys.argv[1]
def rows(name):
    with open(os.path.join(A, name), newline='') as f:
        return list(csv.DictReader(f, delimiter='\t', quoting=csv.QUOTE_NONE))
files = rows('files.tsv'); fns = rows('functions.tsv'); types = rows('types.tsv'); allows = rows('allows.tsv')
INT_F = ['loc','code','comment','blank','fns','test_fns','types','pub_items','allows','reach','macro_rules','mod_decls']
for r in files:
    for k in INT_F: r[k] = int(r[k])
    r['test'] = r['test'] == '1'
INT_N = ['line','loc','body_loc','params','bool_params','generics','cyclo','cognitive','max_nest','stmts','unwraps','expects','clones','alloc_in_loop','panics','unsafes','tries','closures','max_loop_depth','index_ops','as_casts']
for r in fns:
    for k in INT_N: r[k] = int(r[k])
    r['test'] = r['test'] == '1'
    r['prod'] = (not r['test']) and r['kind'] in ('lib', 'bin')
def pct(v, p):
    if not v: return 0
    v = sorted(v); i = min(len(v) - 1, int(round(p / 100 * (len(v) - 1)))); return v[i]
crates = collections.OrderedDict()
for r in files:
    c = crates.setdefault(r['crate'], dict(files=0, prod_files=0, test_files=0, orphan_files=0, orphan_loc=0, loc=0, prod_loc=0, prod_code=0, test_loc=0, example_loc=0, pub_items=0, over800=0, max_file=0))
    c['files'] += 1; c['loc'] += r['loc']
    if r['kind'] in ('orphan', 'included'):
        if r['kind'] == 'orphan': c['orphan_files'] += 1; c['orphan_loc'] += r['loc']
    elif r['kind'] in ('lib', 'bin') and not r['test']:
        c['prod_files'] += 1; c['prod_loc'] += r['loc']; c['prod_code'] += r['code']; c['pub_items'] += r['pub_items']
        if r['loc'] > 800: c['over800'] += 1
        c['max_file'] = max(c['max_file'], r['loc'])
    elif r['kind'] == 'example': c['example_loc'] += r['loc']
    else:
        c['test_files'] += 1; c['test_loc'] += r['loc']
byc = collections.defaultdict(list)
for r in fns: byc[r['crate']].append(r)
SUMK = ['unwraps','expects','clones','alloc_in_loop','panics','unsafes','index_ops','as_casts']
for name, c in crates.items():
    fs = byc.get(name, []); prod = [f for f in fs if f['prod']]
    c['fns'] = len(fs); c['prod_fns'] = len(prod); c['test_fns'] = sum(1 for f in fs if f['test'])
    L = [f['loc'] for f in prod]; CY = [f['cyclo'] for f in prod]; CG = [f['cognitive'] for f in prod]
    c['fn_loc_med'] = statistics.median(L) if L else 0; c['fn_loc_p90'] = pct(L, 90); c['fn_loc_p99'] = pct(L, 99); c['fn_loc_max'] = max(L) if L else 0
    c['cyclo_p90'] = pct(CY, 90); c['cyclo_max'] = max(CY) if CY else 0; c['cog_p90'] = pct(CG, 90); c['cog_max'] = max(CG) if CG else 0
    c['loc_gt60'] = sum(1 for x in L if x > 60); c['loc_gt100'] = sum(1 for x in L if x > 100); c['loc_gt200'] = sum(1 for x in L if x > 200)
    c['cyclo_gt15'] = sum(1 for x in CY if x > 15); c['cyclo_gt30'] = sum(1 for x in CY if x > 30); c['cog_gt25'] = sum(1 for x in CG if x > 25); c['cog_gt60'] = sum(1 for x in CG if x > 60)
    c['params_gt5'] = sum(1 for f in prod if f['params'] > 5); c['params_gt7'] = sum(1 for f in prod if f['params'] > 7)
    c['bool_params'] = sum(1 for f in prod if f['bool_params'] > 0); c['nest_ge5'] = sum(1 for f in prod if f['max_nest'] >= 5)
    c['loc_in_long_fns'] = sum(x for x in L if x > 100)
    c['undocumented_pub'] = sum(1 for f in prod if f['vis'] == 'pub' and f['has_doc'] == '0')
    c['pub_fns'] = sum(1 for f in prod if f['vis'] == 'pub')
    for k in SUMK: c[k] = sum(f[k] for f in prod)
tot = collections.Counter()
for c in crates.values():
    for k, v in c.items():
        if isinstance(v, (int, float)) and not k.endswith(('_med','_p90','_p99','_max')) and k != 'max_file': tot[k] += v
out = dict(crates=crates, total=dict(tot))
prod = [f for f in fns if f['prod']]
def top(key, n=40, flt=lambda f: True):
    return [dict(crate=f['crate'], file=f['file'], line=f['line'], fn=(f['ctx'] + '::' if f['ctx'] else '') + f['name'], loc=f['loc'], cyclo=f['cyclo'], cognitive=f['cognitive'], nest=f['max_nest'], params=f['params'], bools=f['bool_params'], alloc_in_loop=f['alloc_in_loop'], clones=f['clones'], unwraps=f['unwraps'])
            for f in sorted([f for f in prod if flt(f)], key=key, reverse=True)[:n]]
out['top_cognitive'] = top(lambda f: f['cognitive'])
out['top_loc'] = top(lambda f: f['loc'])
out['top_params'] = top(lambda f: (f['params'], f['loc']))
out['top_nest'] = top(lambda f: (f['max_nest'], f['cognitive']))
out['top_alloc_in_loop'] = top(lambda f: f['alloc_in_loop'])
out['top_bool_params'] = top(lambda f: (f['bool_params'], f['params']), flt=lambda f: f['bool_params'] >= 2)
# types
meth = collections.Counter(); meth_loc = collections.Counter(); impl_blocks = collections.Counter()
for t in types:
    if t['kind'] not in ('lib', 'bin') or t['test'] == '1': continue
    if t['tkind'] == 'impl' and not t['extra']:
        k = (t['crate'], t['name']); meth[k] += int(t['members']); meth_loc[k] += int(t['loc']); impl_blocks[k] += 1
out['god_types'] = [dict(crate=k[0], type=k[1], methods=v, impl_loc=meth_loc[k], impl_blocks=impl_blocks[k]) for k, v in meth.most_common(30)]
P = lambda tk: [t for t in types if t['tkind'] == tk and t['kind'] in ('lib', 'bin') and t['test'] == '0']
out['big_structs'] = [dict(crate=t['crate'], file=t['file'], line=int(t['line']), name=t['name'], fields=int(t['members']), pub_fields=int(t['pub_members'])) for t in sorted(P('struct'), key=lambda t: -int(t['members']))[:30]]
out['big_enums'] = [dict(crate=t['crate'], file=t['file'], line=int(t['line']), name=t['name'], variants=int(t['members'])) for t in sorted(P('enum'), key=lambda t: -int(t['members']))[:30]]
out['big_traits'] = [dict(crate=t['crate'], file=t['file'], line=int(t['line']), name=t['name'], methods=int(t['members'])) for t in sorted(P('trait'), key=lambda t: -int(t['members']))[:20]]
out['statics'] = [dict(crate=t['crate'], file=t['file'], line=int(t['line']), name=t['name'], kind=t['tkind'], ty=t['extra']) for t in types if t['tkind'] in ('static', 'static_mut') and t['kind'] in ('lib', 'bin') and t['test'] == '0']
st = collections.Counter(t['tkind'] for t in types if t['kind'] in ('lib', 'bin') and t['test'] == '0'); out['type_counts'] = dict(st)
all_structs = P('struct'); out['pub_field_structs'] = sum(1 for t in all_structs if int(t['pub_members']) > 0 and t['vis'] == 'pub'); out['pub_structs'] = sum(1 for t in all_structs if t['vis'] == 'pub')
# allows
al = collections.Counter((a['lint']) for a in allows if a['test'] == '0'); out['allows_by_lint'] = al.most_common(40)
out['crate_level_allows'] = [dict(crate=a['crate'], file=a['file'], line=int(a['line']), lint=a['lint']) for a in allows if int(a['line']) <= 25 and re.search(r'/src/(lib|main)\.rs$', a['file'])]
# files
pf = [r for r in files if r['kind'] in ('lib', 'bin') and not r['test']]
out['top_files'] = [dict(file=r['file'], loc=r['loc'], fns=r['fns']) for r in sorted(pf, key=lambda r: -r['loc'])[:40]]
bk = collections.Counter()
for r in pf:
    l = r['loc']; bk['<=100' if l <= 100 else '101-300' if l <= 300 else '301-500' if l <= 500 else '501-800' if l <= 800 else '801-1500' if l <= 1500 else '>1500'] += 1
out['file_size_buckets'] = dict(bk)
near = sum(1 for r in pf if 700 <= r['loc'] <= 800); out['files_700_800'] = near
mech = re.compile(r'(_impl_\d+|_part_?\d+|(^|/)methods_|_\d+\.rs$|_cont(inued)?\.rs$|_tail\.rs$|_more\.rs$|_rest\.rs$|_extra\.rs$|_second\.rs$|_third\.rs$|_next\.rs$|_split\.rs$|_helpers?_\d|_a\.rs$|_b\.rs$)')
out['mechanical_split_names'] = sorted([dict(file=r['file'], loc=r['loc']) for r in files if mech.search(r['file']) and r['kind'] not in ('test', 'example')], key=lambda r: r['file'])
out['orphans'] = sorted([dict(file=r['file'], loc=r['loc'], crate=r['crate']) for r in files if r['kind'] == 'orphan'], key=lambda r: (r['crate'], -r['loc']))
out['included'] = [dict(file=r['file'], loc=r['loc']) for r in files if r['kind'] == 'included']
out['multi_reach'] = [dict(file=r['file'], reach=r['reach'], loc=r['loc']) for r in sorted(files, key=lambda r: -r['reach'])[:15] if r['reach'] > 1 and r['kind'] in ('lib', 'bin')]
json.dump(out, open(os.path.join(A, 'summary.json'), 'w'), indent=1)
# ---- text
print("crate                  prodLOC  testLOC orphLOC  files  fns  med p90  p99  max | >60 >100 >200 | cy>15 cy>30 cog>25 cog>60 | par>5 par>7 bool nest>=5 | unwrap expect panic clone allocLoop unsafe | pubItems")
for name, c in sorted(crates.items(), key=lambda kv: -kv[1]['prod_loc']):
    print(f"{name:22s}{c['prod_loc']:8d}{c['test_loc']:9d}{c['orphan_loc']:8d}{c['prod_files']:7d}{c['prod_fns']:6d}{int(c['fn_loc_med']):4d}{c['fn_loc_p90']:4d}{c['fn_loc_p99']:5d}{c['fn_loc_max']:5d} |{c['loc_gt60']:4d}{c['loc_gt100']:5d}{c['loc_gt200']:5d} |{c['cyclo_gt15']:6d}{c['cyclo_gt30']:6d}{c['cog_gt25']:7d}{c['cog_gt60']:7d} |{c['params_gt5']:6d}{c['params_gt7']:6d}{c['bool_params']:5d}{c['nest_ge5']:8d} |{c['unwraps']:7d}{c['expects']:7d}{c['panics']:6d}{c['clones']:6d}{c['alloc_in_loop']:10d}{c['unsafes']:7d} |{c['pub_items']:8d}")
t = out['total']
print("TOTAL", {k: t[k] for k in ['files','prod_files','test_files','orphan_files','loc','prod_loc','test_loc','orphan_loc','example_loc','fns','prod_fns','test_fns','loc_gt60','loc_gt100','loc_gt200','cyclo_gt15','cyclo_gt30','cog_gt25','cog_gt60','params_gt5','params_gt7','bool_params','nest_ge5','unwraps','expects','panics','clones','alloc_in_loop','unsafes','index_ops','as_casts','pub_items','pub_fns','undocumented_pub','over800','loc_in_long_fns']})
print("file size buckets (prod):", out['file_size_buckets'], "files 700..800:", out['files_700_800'])
print("type counts:", out['type_counts'], "pub structs:", out['pub_structs'], "with pub fields:", out['pub_field_structs'])
