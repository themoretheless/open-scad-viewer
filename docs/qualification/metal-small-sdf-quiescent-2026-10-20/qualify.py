#!/usr/bin/env python3
"""Observe-only quiescence, AA-first qualification, full SDF workflow.
Competing jobs are observed, never stopped. Contaminated windows cannot qualify.
Legacy recovery resumes only exact identities recorded by older runner versions.
"""
import os, sys, time, json, signal, subprocess, pathlib, re, hashlib, random, statistics, math, csv
ROOT = pathlib.Path(__file__).resolve().parent
JOURNAL = ROOT / 'pause-journal.json'

def snapshot():
    result = {}
    raw = subprocess.check_output(['ps','-axo','uid=,pid=,ppid=,state=,lstart=,comm='], text=True)
    for line in raw.splitlines():
        c = line.split(None,9)
        if len(c)!=10 or int(c[0])!=os.getuid(): continue
        pid=int(c[1]);result[pid]={'pid':pid,'ppid':int(c[2]),'state':c[3], 'start':' '.join(c[4:9]),'comm':c[9]}
    # Explicit benchmark/test commands can live outside Cargo target paths.
    for line in subprocess.check_output(['ps','-axo','pid=,args='], text=True).splitlines():
        c=line.strip().split(None,1)
        if len(c)!=2:continue
        pid=int(c[0])
        if pid in result:
            result[pid]['benchmark']=bool(re.search(r'--offscreen-benchmark|--benchmark(?:[ =]|$)|(?:^|/)(?:pytest|vitest)(?:[ /]|$)|node .*--test(?:[ =]|$)',c[1]))
    return {pid:p for pid,p in result.items() if 'Z' not in p['state']}

def save(data):
    tmp=JOURNAL.with_suffix('.tmp');tmp.write_text(json.dumps(data,indent=2));tmp.replace(JOURNAL)

def identity(a,b): return a['pid']==b['pid'] and a['start']==b['start'] and a['comm']==b['comm']

def restore(data):
    current=snapshot()
    for p in reversed(data['paused']):
        now=current.get(p['pid'])
        if now and identity(p,now) and 'T' in now['state']:
            try: os.kill(p['pid'],signal.SIGCONT);p['resumed']=time.time()
            except ProcessLookupError: pass
    data['active']=False;data['restored_at']=time.time();save(data)

def watchdog(owner,deadline):
    while True:
        try:
            data=json.loads(JOURNAL.read_text())
            if not data['active']: return
            os.kill(owner,0)
            if time.monotonic()>deadline: restore(data);return
        except ProcessLookupError:
            restore(json.loads(JOURNAL.read_text()));return
        time.sleep(.5)

if len(sys.argv)>1 and sys.argv[1]=='--watchdog':
    watchdog(int(sys.argv[2]),float(sys.argv[3]));sys.exit(0)

BINARY=pathlib.Path(sys.argv[1]).resolve()
if JOURNAL.exists(): raise FileExistsError('Use a fresh evidence directory; do not overwrite pause journal')
data={'active':True,'owner':os.getpid(),'paused':[],'started_at':time.time()}
save(data)
deadline=time.monotonic()+480
guard=subprocess.Popen([sys.executable,str(pathlib.Path(__file__).resolve()),'--watchdog',str(os.getpid()),str(deadline)],start_new_session=True,stdout=subprocess.DEVNULL,stderr=open(ROOT/'watchdog.log','w'))
allowed={os.getpid(),guard.pid}
events=[]

def competitor(p):
    name=pathlib.Path(p['comm']).name
    if p.get('benchmark'):return True
    if name in {'cargo','rustc','rustdoc','clang','clang++','cc1','gcc','g++','ld','rust-lld','make','ninja','cmake','ctest','pytest'}: return True
    return bool(re.search(r'/(debug|release)/(build|deps)/',p['comm']) and re.search(r'[-_][0-9a-f]{16}$',name))

def observe_competitors():
    """Compatibility hook: observe competitors; never stop other sessions."""
    if time.monotonic()>deadline-2:raise RuntimeError('Observation lease expired')
    snap=snapshot();owned={os.getpid()}
    # Only this runner's descendants are ours. Allowed ancestors must not
    # exempt unrelated sessions launched by the same desktop application.
    while True:
        expanded=owned|{pid for pid,p in snap.items() if p['ppid'] in owned}
        if expanded==owned:break
        owned=expanded
    selected={pid for pid,p in snap.items() if pid not in allowed and pid not in owned and competitor(p) and 'T' not in p['state']}
    while True:
        expanded=selected|{pid for pid,p in snap.items() if p['ppid'] in selected and pid not in allowed and pid not in owned and 'T' not in p['state']}
        if expanded==selected:break
        selected=expanded
    observed=sorted(selected)
    if observed:events.append({'time':time.time(),'paused':[],'observed_competitors':observed})
    return observed

def settle():
    quiet=time.monotonic();limit=quiet+90
    while time.monotonic()-quiet<10:
        if observe_competitors():quiet=time.monotonic()
        if time.monotonic()>limit:raise RuntimeError('No ten-second quiescent window')
        time.sleep(.5)

def summarize(path):
    groups={};rng=random.Random(712)
    for r in csv.DictReader(path.open()):groups.setdefault((r['case'],r['field'],int(r['cells'])),[]).append(r)
    out=[]
    for (case,field,cells),rows in groups.items():
        logs=[math.log(float(r['a_ns'])/float(r['b_ns'])) for r in rows]
        boots=sorted(math.exp(statistics.median(rng.choices(logs,k=len(logs)))) for _ in range(4000))
        out.append({'case':case,'field':field,'cells':cells,'ratio':math.exp(statistics.median(logs)), 'lo':boots[100],'hi':boots[3899], 'a_ns':statistics.median(float(r['a_ns']) for r in rows),'b_ns':statistics.median(float(r['b_ns']) for r in rows)})
    return out

def stable(rows):
    aa=[r for r in rows if r['case']=='aa']
    return len(aa)==4 and all(.97<=r['ratio']<=1.03 and r['hi']/r['lo']<=1.06 for r in aa)

def run(phase,slot):
    for attempt in [1,2]:
        settle();name=f'{phase}-{slot}-attempt-{attempt}'
        path=ROOT/(name+'.csv')
        env=dict(os.environ,COMPUTE_REQUIRE_GPU='1',METAL_SMALL_SDF_PHASE=phase,METAL_SMALL_SDF_CSV=str(path))
        with (ROOT/(name+'.log')).open('x') as log:
            child=subprocess.Popen([str(BINARY),'--exact','gpu::small_batch_tests::paired_polygonize','--ignored','--test-threads=1'],env=env,stdout=log,stderr=subprocess.STDOUT)
            allowed.add(child.pid);contaminated=[];started=time.monotonic()
            while child.poll() is None:
                contaminated+=observe_competitors()
                if time.monotonic()-started>100:
                    child.terminate();child.wait();raise RuntimeError('Own benchmark timed out')
                time.sleep(.5)
            allowed.remove(child.pid)
            if child.returncode:raise RuntimeError(f'Own benchmark failed: {child.returncode}')
        rows=summarize(path)
        trial={'phase':phase,'slot':slot,'attempt':attempt,'file':path.name,'new_competitors':contaminated,'summary':rows}
        trials.append(trial);(ROOT/'trials.json').write_text(json.dumps(trials,indent=2))
        if not contaminated:return trial
    raise RuntimeError('New competing work entered both attempts')

def terminate(signum,frame):raise KeyboardInterrupt(f'signal {signum}')
signal.signal(signal.SIGTERM,terminate);signal.signal(signal.SIGINT,terminate)
trials=[];result={'accepted':False}
try:
    meta={'binary':str(BINARY),'sha256':hashlib.sha256(BINARY.read_bytes()).hexdigest(),'cpu':subprocess.check_output(['sysctl','-n','machdep.cpu.brand_string'],text=True).strip()}
    repo=ROOT.parents[2]
    meta['sources']={p:hashlib.sha256((repo/p).read_bytes()).hexdigest() for p in ['crates/sdf-core/src/gpu.rs','crates/sdf-core/src/gpu/small_batch_tests.rs','crates/gpu-compute/src/buffer.rs','crates/compute-core/src/runtime.rs','crates/Cargo.lock']}
    (ROOT/'environment.json').write_text(json.dumps(meta,indent=2))
    controls=[run('controls',i) for i in range(1,4)]
    result['controls_stable']=all(stable(t['summary']) for t in controls)
    (ROOT/'decision.json').write_text(json.dumps(result,indent=2))
    if not result['controls_stable']:raise RuntimeError('AA calibration did not pass; no A/B measurements performed')
    print('Three AA-only runs passed. Starting full workflow A/B.',flush=True)
    full=[run('full',i) for i in range(1,4)]
    result['full_controls_stable']=all(stable(t['summary']) for t in full)
    batch=[r for t in full for r in t['summary'] if r['case']=='batch']
    result['all_full_intervals_above_one']=len(batch)==12 and all(r['lo']>1 for r in batch)
    result['accepted']=result['full_controls_stable'] and result['all_full_intervals_above_one']
except BaseException as e:
    result['error']=str(e)
finally:
    restore(data)
    (ROOT/'pause-events.json').write_text(json.dumps(events,indent=2))
    (ROOT/'decision.json').write_text(json.dumps(result,indent=2))
    guard.wait(timeout=5)
    print(json.dumps(result),flush=True)
