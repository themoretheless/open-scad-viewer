#!/usr/bin/env python3
"""Run already-built release test executable, serially; never build while timing."""
import argparse, pathlib, subprocess, datetime, hashlib, json
p=argparse.ArgumentParser();p.add_argument('binary',type=pathlib.Path);p.add_argument('--runs',type=int,default=3);p.add_argument('--prefix',default='paired');a=p.parse_args()
root=pathlib.Path(__file__).resolve().parent
repo=root.parents[2]
meta={'start_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'binary_sha256':hashlib.sha256(a.binary.read_bytes()).hexdigest(),'cpu':subprocess.check_output(['sysctl','-n','machdep.cpu.brand_string'],text=True).strip()}
meta['sources']={str(x.relative_to(repo)):hashlib.sha256(x.read_bytes()).hexdigest() for x in [repo/'crates/photogrammetry-core/src/gpu/matching.rs',repo/'crates/photogrammetry-core/src/gpu/matching/small_batch_tests.rs',repo/'crates/gpu-compute/src/buffer.rs']}
(root/(a.prefix+'-environment.json')).write_text(json.dumps(meta,indent=2))
import os
for run in range(1,a.runs+1):
    csv=root/f'{a.prefix}-{run}.csv'
    if csv.exists():raise FileExistsError(csv)
    env=dict(os.environ,METAL_SMALL_MATCHER_CSV=str(csv),COMPUTE_REQUIRE_GPU='1')
    with (root/f'{a.prefix}-{run}.log').open('x') as log:
        subprocess.run([str(a.binary.resolve()),'--exact','gpu::matching::small_batch_tests::paired_full_matcher','--ignored','--test-threads=1'],env=env,stdout=log,stderr=subprocess.STDOUT,check=True)
    assert csv.exists(), 'benchmark test was not executed'
print('completed',a.runs,'runs')
