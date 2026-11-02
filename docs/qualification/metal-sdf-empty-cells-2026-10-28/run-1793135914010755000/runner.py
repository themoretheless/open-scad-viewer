#!/usr/bin/env python3
"""Wait for naturally idle CPU/GPU; never signal competing processes.
All A/A gates precede A/B. Retain every contaminated or unstable attempt.
"""
import argparse
from collections import deque
import csv
import ctypes
import hashlib
import json
import math
import os
from pathlib import Path
import random
import re
import signal
import shutil
import statistics
import subprocess
import time

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[2]
CASES = ['sphere-gpu-64', 'tetra-auto-49', 'tetra-auto-64', 'sphere-auto-64', 'tetra-auto-48', 'negative-gpu-8', 'negative-cpu-8']
FIXTURE = 'gpu::small_batch_tests::paired_empty_cells'
SOURCE_CRATES = ['sdf-core', 'gpu-compute', 'compute-core', 'geometry-ops',
                 'math-core', 'planar-geometry', 'tensor-core', 'value-codec']
SOURCES = ['crates/Cargo.lock', 'crates/Cargo.toml']
for crate in SOURCE_CRATES:
    SOURCES.append(f'crates/{crate}/Cargo.toml')
    SOURCES.extend(str(p.relative_to(REPO)) for p in (REPO / 'crates' / crate / 'src').rglob('*')
                   if p.is_file() and p.suffix in {'.rs', '.wgsl'})
SOURCES = sorted(set(SOURCES))



def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write_json(path, value):
    temporary = path.with_suffix('.tmp')
    temporary.write_text(json.dumps(value, indent=2))
    temporary.replace(path)


def processes():
    # One process-table traversal. Resolve executable paths through libproc;
    # command-line arguments alone are not reliable executable identities.
    libproc = ctypes.CDLL('/usr/lib/libproc.dylib')
    libproc.proc_pidpath.argtypes = [ctypes.c_int, ctypes.c_void_p, ctypes.c_uint32]
    result = {}
    for line in subprocess.check_output(
            ['ps', '-axo', 'uid=,pid=,ppid=,state=,lstart=,time=,args='], text=True).splitlines():
        c = line.split(None, 10)
        if len(c) != 11 or int(c[0]) != os.getuid() or any(s in c[3] for s in 'ZT'):
            continue
        pid = int(c[1])
        buffer = ctypes.create_string_buffer(4096)
        if libproc.proc_pidpath(pid, buffer, len(buffer)) <= 0:
            # A process may exit or become a zombie between reads. Resolve
            # exceptional identities through the original ps executable column.
            fallback = subprocess.run(['ps', '-p', str(pid), '-o', 'state=,comm='],
                                      text=True, stdout=subprocess.PIPE,
                                      stderr=subprocess.DEVNULL)
            fields = fallback.stdout.strip().split(None, 1)
            if not fields or any(state in fields[0] for state in 'ZT'):
                continue
            if len(fields) != 2:
                raise RuntimeError(f'Executable identity unavailable for live process {pid}')
            comm = fields[1]
        else:
            comm = os.fsdecode(buffer.value)
        name = Path(comm).name.lower()
        interpreter = name.startswith(('python', 'node')) or name in {'sh', 'bash', 'zsh'}
        explicit = (
            (not interpreter and bool(re.search(r'(?:^|\s)--(?:offscreen-benchmark|benchmark)(?:[ =]|$)', c[10])))
            or (name.startswith('python') and bool(re.search(r' -m (?:pytest|unittest)(?:[ /]|$)', c[10])))
            or (name == 'node' and bool(re.search(r' --test(?:[ =]|$)', c[10]))))
        result[pid] = dict(pid=pid, ppid=int(c[2]), state=c[3], start=' '.join(c[4:9]),
                           comm=comm, cpu_time=c[9], explicit_work=explicit)
    return result



def descendants(snap, pid):
    owned = {pid}
    while True:
        expanded = owned | {p['pid'] for p in snap.values() if p['ppid'] in owned}
        if expanded == owned:
            return owned
        owned = expanded


def competing(snap, owned):
    compilers = {'cargo', 'rustc', 'rustdoc', 'clang', 'clang++', 'cc1', 'gcc', 'g++',
                 'ld', 'rust-lld', 'make', 'ninja', 'cmake', 'ctest', 'pytest'}
    selected = set()
    for pid, p in snap.items():
        if pid in owned:
            continue
        name = Path(p['comm']).name
        if (p.get('explicit_work') or name in compilers
                or re.search(r'/(debug|release)/(build|deps)/.*[-_][0-9a-f]{16}$', p['comm'])
                or (re.search(r'(?:^bench[_-]|[_-]bench$)', name)
                    and any(k in p['comm'] for k in ['/target/', '/artifacts/', '/tmp/']))):
            selected.add(pid)
    while True:
        expanded = selected | {pid for pid, p in snap.items()
                               if p['ppid'] in selected and pid not in owned}
        if expanded == selected:
            return [snap[pid] for pid in sorted(selected)]
        selected = expanded


class Observer:
    def __init__(self, log):
        self.log = log
        self.lib = ctypes.CDLL('/usr/lib/libSystem.B.dylib')
        self.lib.mach_host_self.restype = ctypes.c_uint
        self.host = self.lib.mach_host_self()
        self.previous = self.cpu_ticks()
        self.quiet = time.monotonic()
        self.gpu_quiet = self.quiet
        self.cpu_history = deque()
        self.cpu_started = self.quiet
        self.last_progress = 0

    def cpu_ticks(self):
        values = (ctypes.c_uint * 4)()
        count = ctypes.c_uint(4)
        if self.lib.host_statistics(self.host, 3, values, ctypes.byref(count)):
            raise RuntimeError('CPU host statistics unavailable')
        return list(values)

    def poll(self, stage, own_work=False):
        poll_start = time.perf_counter_ns()
        snap = processes()
        rivals = competing(snap, descendants(snap, os.getpid()))
        raw = subprocess.check_output(['ioreg', '-r', '-c', 'AGXAccelerator', '-d', '1'], text=True)
        match = re.search(r'"Device Utilization %"=(\d+)', raw)
        if not match:
            raise RuntimeError('AGX utilization unavailable')
        gpu = int(match[1])
        ticks = self.cpu_ticks()
        delta = [(b - a) % (1 << 32) for a, b in zip(self.previous, ticks)]
        self.previous = ticks
        idle = 100 * delta[2] / sum(delta) if sum(delta) else 0
        now = time.monotonic()
        if rivals:
            self.quiet = now
        if not own_work:
            if gpu > 10:
                self.gpu_quiet = now
            self.cpu_history.append((now, sum(delta), delta[2]))
            while len(self.cpu_history) > 1 and self.cpu_history[1][0] <= now - 5:
                self.cpu_history.popleft()
        record = dict(time=time.time(), stage=stage, gpu_percent=gpu, cpu_idle_percent=idle,
                      observer_poll_ns=time.perf_counter_ns() - poll_start, rivals=rivals,
                      process_cpu=[dict(pid=p['pid'], start=p['start'], comm=p['comm'],
                                        cpu_time=p['cpu_time']) for p in snap.values()])
        self.log.write(json.dumps(record) + '\n')
        self.log.flush()
        if stage == 'waiting' and now - self.last_progress >= 30:
            print(json.dumps(dict(waiting=True, rivals=len(rivals), gpu_percent=gpu,
                                  cpu_idle_percent=round(idle, 1))), flush=True)
            self.last_progress = now
        return record

    def wait(self):
        # A fresh GPU/CPU-idle streak is required after our previous workload;
        # the no-competitor streak can span our own completed calls.
        self.gpu_quiet = self.cpu_started = time.monotonic()
        self.cpu_history.clear()
        while True:
            self.poll('waiting')
            now = time.monotonic()
            total = sum(p[1] for p in self.cpu_history)
            cpu_idle_window = 100 * sum(p[2] for p in self.cpu_history) / total if total else 0
            if (now - self.quiet >= 15 and now - self.gpu_quiet >= 5
                    and now - self.cpu_started >= 5 and cpu_idle_window >= 80):
                self.log.write(json.dumps(dict(time=time.time(), stage='admitted',
                                               cpu_idle_window=cpu_idle_window)) + '\n')
                self.log.flush()
                return
            time.sleep(.5)


def summarize(path):
    groups = {}
    for row in csv.DictReader(path.open()):
        groups.setdefault(row['case'], []).append(row)
    results = []
    for kind, rows in groups.items():
        assert len(rows) == 48, (kind, len(rows))
        logs = [math.log(float(r['a_ns']) / float(r['b_ns'])) for r in rows]
        rng = random.Random(712)
        boots = sorted(math.exp(statistics.median(rng.choices(logs, k=len(logs))))
                       for _ in range(10000))
        results.append(dict(kind=kind, ratio=math.exp(statistics.median(logs)),
                            lo=boots[250], hi=boots[9749],
                            a_ns=statistics.median(float(r['a_ns']) for r in rows),
                            b_ns=statistics.median(float(r['b_ns']) for r in rows)))
    return results


def stable(summary):
    aa = [r for r in summary if r['kind'] == 'aa']
    return len(aa) == 1 and all(.97 <= r['ratio'] <= 1.03 and r['hi'] / r['lo'] <= 1.06 for r in aa)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('binary', type=Path)
    parser.add_argument('--resume-from', type=Path)
    args = parser.parse_args()
    binary = args.binary.resolve()
    predecessor = args.resume_from.resolve() if args.resume_from else None
    if predecessor:
        prior_meta = json.loads((predecessor / 'environment.json').read_text())
        prior_decision = json.loads((predecessor / 'decision.json').read_text())
        if (prior_decision.get('error') != 'Own fixture timed out'
                or not all(prior_decision.get(k) for k in ['binary_unchanged', 'sources_unchanged', 'runner_unchanged'])
                or digest(binary) != prior_meta['sha256']
                or any(digest(REPO / p) != h for p, h in prior_meta['sources'].items())):
            raise RuntimeError('Timeout recovery requires unchanged frozen evidence')
    out = ROOT / f'run-{time.time_ns()}' 
    out.mkdir()
    (out / 'runner.py').write_bytes(Path(__file__).read_bytes())
    meta = dict(binary=str(binary), sha256=digest(binary), runner_sha256=digest(__file__),
                sources={p: digest(REPO / p) for p in SOURCES},
                cpu=subprocess.check_output(['sysctl', '-n', 'machdep.cpu.brand_string'], text=True).strip(),
                cases=CASES,
                reps_per_case={case: (32 if case.startswith(('mesh', 'cross')) else 8) for case in CASES},
                blocks=48, warmup_ms=3000, sampling="adjacent single-workflow ABBA cycles averaged within blocks", started_at=time.time())
    for source in SOURCES:
        destination = out / 'sources' / source
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes((REPO / source).read_bytes())
    if predecessor:
        meta['resumed_from'] = str(predecessor)
    write_json(out / 'environment.json', meta)
    print(json.dumps(dict(output=str(out))), flush=True)
    trials = json.loads((predecessor / 'trials.json').read_text()) if predecessor else []
    accepted = json.loads((predecessor / 'accepted-windows.json').read_text()) if predecessor else []
    if predecessor:
        for file in predecessor.iterdir():
            if file.suffix in {'.csv', '.log'}:
                shutil.copyfile(file, out / file.name)
    write_json(out / 'trials.json', trials)
    write_json(out / 'accepted-windows.json', accepted)
    child = None
    result = dict(completed=False, confirmed_improvements=[], confirmed_regressions=[])

    def check_frozen():
        if digest(binary) != meta['sha256'] or digest(__file__) != meta['runner_sha256']:
            raise RuntimeError('Frozen binary or runner changed')
        if any(digest(REPO / p) != h for p, h in meta['sources'].items()):
            raise RuntimeError('Frozen SDF sources changed; rebuild and restart qualification')

    def interrupted(signum, frame):
        raise KeyboardInterrupt(f'signal {signum}')

    signal.signal(signal.SIGINT, interrupted)
    signal.signal(signal.SIGTERM, interrupted)
    try:
        with (out / 'telemetry.jsonl').open('x') as log:
            observer = Observer(log)
            profile_attempt = 0
            while True:
                profile_attempt += 1
                observer.wait()
                check_frozen()
                contaminated = set()
                with (out / f'profile-{profile_attempt}.log').open('x') as output:
                    child = subprocess.Popen([str(binary), '--exact', 'gpu::small_batch_tests::profile_large_stages', '--ignored', '--nocapture', '--test-threads=1'], stdout=output, stderr=subprocess.STDOUT)
                    started = time.monotonic()
                    while child.poll() is None:
                        sample = observer.poll('profile', own_work=True)
                        contaminated.update(p['pid'] for p in sample['rivals'])
                        if time.monotonic() - started > 180:
                            child.terminate()
                            child.wait(timeout=10)
                            raise RuntimeError('Own profile timed out')
                        time.sleep(.5)
                    code = child.returncode
                    child = None
                if code: raise RuntimeError('Own profile failed')
                time.sleep(1)
                tail = observer.poll('profile-tail')
                contaminated.update(p['pid'] for p in tail['rivals'])
                check_frozen()
                write_json(out / f'profile-{profile_attempt}.json', dict(clean=not contaminated and tail['gpu_percent'] <= 10, rivals=sorted(contaminated), tail_gpu=tail['gpu_percent']))
                if not contaminated and tail['gpu_percent'] <= 10:
                    write_json(out / 'profile-accepted.json', dict(log=f'profile-{profile_attempt}.log'))
                    break
            for phase in ['controls', 'full']:
                for case in CASES:
                    for repetition in range(1, 4):
                        if any(t['phase'] == phase and t['case'] == case and t['repetition'] == repetition for t in accepted):
                            continue
                        prefix = f'{phase}-{case}-{repetition}-attempt-'
                        attempt = max([int(f.stem[len(prefix):]) for f in out.glob(prefix + '*.log')] or [0])
                        while True:
                            attempt += 1
                            observer.wait()
                            check_frozen()
                            name = f'{phase}-{case}-{repetition}-attempt-{attempt}'
                            path = out / f'{name}.csv'
                            env = dict(os.environ, COMPUTE_REQUIRE_GPU='1', METAL_SMALL_SDF_PHASE=phase,
                                       METAL_SMALL_SDF_CASE=case, METAL_SMALL_SDF_CSV=str(path))
                            contaminated = set()
                            with (out / f'{name}.log').open('x') as output:
                                child = subprocess.Popen([str(binary), '--exact', FIXTURE, '--ignored',
                                                          '--test-threads=1'], env=env, stdout=output,
                                                         stderr=subprocess.STDOUT)
                                started = time.monotonic()
                                while child.poll() is None:
                                    sample = observer.poll(name, own_work=True)
                                    contaminated.update(p['pid'] for p in sample['rivals'])
                                    if time.monotonic() - started > 180:
                                        child.terminate()  # Our own explicitly created child only.
                                        child.wait(timeout=10)
                                        raise RuntimeError('Own fixture timed out')
                                    time.sleep(.5)
                                code = child.returncode
                                child = None
                            if code:
                                raise RuntimeError(f'Own fixture failed: {name}, exit {code}')
                            # Submitted work has completed before the fixture exits.
                            # Sustained GPU load one second later is outside this workflow.
                            time.sleep(1)
                            tail = observer.poll(name + '-tail')
                            contaminated.update(p['pid'] for p in tail['rivals'])
                            summary = summarize(path)
                            trial = dict(phase=phase, case=case, repetition=repetition, attempt=attempt,
                                         csv=path.name, rivals=sorted(contaminated), tail_gpu=tail['gpu_percent'],
                                         summary=summary)
                            trial['clean'] = not contaminated and tail['gpu_percent'] <= 10
                            trial['stable'] = stable(summary)
                            trials.append(trial)
                            check_frozen()
                            write_json(out / 'trials.json', trials)
                            print(json.dumps(trial), flush=True)
                            if not trial['clean']:
                                continue
                            if not trial['stable']:
                                # Do not select favorable controls or retry numerical failures.
                                raise RuntimeError(f'Clean A/A gate failed: {name}')
                            accepted.append(trial)
                            write_json(out / 'accepted-windows.json', accepted)
                            break
                if phase == 'controls':
                    assert sum(t['phase'] == 'controls' for t in accepted) == len(CASES) * 3
                    result['aa_first_passed'] = True
                    write_json(out / 'decision.json', result)
                    print('All AA-first controls passed; beginning full A/B + A/A.', flush=True)
            for case in CASES:
                full = [t for t in accepted if t['phase'] == 'full' and t['case'] == case]
                effects = [r for t in full for r in t['summary'] if r['kind'] == 'batch']
                assert len(effects) == 3
                if all(r['lo'] > 1 for r in effects):
                    result['confirmed_improvements'].append(case)
                if all(r['hi'] < 1 for r in effects):
                    result['confirmed_regressions'].append(case)
            result['completed'] = True
    except BaseException as error:
        result['error'] = str(error)
    finally:
        if child is not None and child.poll() is None:
            child.terminate()  # Never signal a process discovered by observation.
            child.wait(timeout=10)
        result['binary_unchanged'] = digest(binary) == meta['sha256']
        result['sources_unchanged'] = all(digest(REPO / p) == h for p, h in meta['sources'].items())
        result['runner_unchanged'] = digest(__file__) == meta['runner_sha256']
        write_json(out / 'decision.json', result)
        print(json.dumps(result), flush=True)
    return 0 if result['completed'] else 2


if __name__ == '__main__':
    raise SystemExit(main())
