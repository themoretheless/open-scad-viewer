#!/usr/bin/env python3
"""Conservative full-path decision, including independent A/A adjustment."""
import csv
import json
import math
from pathlib import Path
import random
import statistics
import sys

NEGATIVE_CONTROLS = {'sphere-auto-64', 'tetra-auto-48', 'mesh86-auto-32'}


def adjusted_interval(path):
    rows = list(csv.DictReader(path.open()))
    groups = {kind: [math.log(float(r['a_ns']) / float(r['b_ns']))
                     for r in rows if r['case'] == kind] for kind in ['batch', 'aa']}
    assert all(len(g) == 48 for g in groups.values())
    rng = random.Random(917)
    boots = sorted(math.exp(statistics.median(rng.choices(groups['batch'], k=48))
                            - statistics.median(rng.choices(groups['aa'], k=48)))
                   for _ in range(10000))
    return dict(ratio=math.exp(statistics.median(groups['batch']) - statistics.median(groups['aa'])),
                lo=boots[250], hi=boots[9749])


def main():
    root = Path(sys.argv[1])
    decision = json.loads((root / 'decision.json').read_text())
    assert decision['completed'] and decision['aa_first_passed'], decision
    assert all(decision[k] for k in ['binary_unchanged', 'sources_unchanged', 'runner_unchanged'])
    windows = json.loads((root / 'accepted-windows.json').read_text())
    assert len(windows) == 48 and all(w['clean'] and w['stable'] for w in windows)
    cases = {}
    table = []
    for window in windows:
        if window['phase'] != 'full':
            continue
        raw = next(r for r in window['summary'] if r['kind'] == 'batch')
        aa = next(r for r in window['summary'] if r['kind'] == 'aa')
        adjusted = adjusted_interval(root / window['csv'])
        row = dict(case=window['case'], repetition=window['repetition'],
                   a_ms=raw['a_ns'] / 1e6, b_ms=raw['b_ns'] / 1e6,
                   ratio=raw['ratio'], lo=raw['lo'], hi=raw['hi'],
                   aa_ratio=aa['ratio'], aa_lo=aa['lo'], aa_hi=aa['hi'],
                   adjusted_ratio=adjusted['ratio'], adjusted_lo=adjusted['lo'], adjusted_hi=adjusted['hi'])
        table.append(row)
        cases.setdefault(window['case'], []).append(row)
    with (root / 'summary.csv').open('w') as f:
        writer = csv.DictWriter(f, fieldnames=list(table[0]))
        writer.writeheader()
        writer.writerows(table)
    report = dict(completed=True, cases={}, confirmed_improvements=[], confirmed_regressions=[])
    for case, rows in cases.items():
        assert len(rows) == 3
        proven = (case not in NEGATIVE_CONTROLS and
                  all(r['lo'] > 1 and r['adjusted_lo'] > 1 for r in rows))
        regression = (case not in NEGATIVE_CONTROLS and
                      all(r['hi'] < 1 and r['adjusted_hi'] < 1 for r in rows))
        report['cases'][case] = dict(negative_control=case in NEGATIVE_CONTROLS,
                                    a_ms=statistics.median(r['a_ms'] for r in rows),
                                    b_ms=statistics.median(r['b_ms'] for r in rows),
                                    ratios=[r['ratio'] for r in rows],
                                    min_raw_lo=min(r['lo'] for r in rows), max_raw_hi=max(r['hi'] for r in rows),
                                    min_adjusted_lo=min(r['adjusted_lo'] for r in rows),
                                    max_adjusted_hi=max(r['adjusted_hi'] for r in rows),
                                    confirmed_gain=proven, confirmed_regression=regression)
        if proven:
            report['confirmed_improvements'].append(case)
        if regression:
            report['confirmed_regressions'].append(case)
    (root / 'analysis.json').write_text(json.dumps(report, indent=2))
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
