#!/usr/bin/env python3
"""Additional native roundtrips; immutable raw fixtures and explicit core guard."""
from pathlib import Path
from fractions import Fraction as F
import copy, hashlib, json, subprocess, sys, time, os, shutil

OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[3]
TARGET=Path(sys.argv[1])
BIN=Path(sys.argv[2])

def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def save(p,v): p.write_text(json.dumps(v,indent=2)+'\n')
def law(points,degree,a=0,b=1):
    return dict(degree=degree,knots=[a]*(degree+1)+[b]*(degree+1),controlPoints=points,weights=[1]*len(points),periodic=False)
def constant(p): return law([p,p],1)
def reverse(c):
    c=copy.deepcopy(c)
    c['controlPoints'].reverse();c['weights'].reverse()
    # All fixture spans are single clamped Beziers; their domain stays literal.
    return c

base=json.loads((ROOT/'crates/geometry-bridge/tests/fixtures/smooth-closed-material-v1.json').read_text())
base['originalSmoothness']=dict(order=2,maxCells=100000,maxExactWork=1000000,maxProducts=1000000)
cases={'rmf':copy.deepcopy(base)}
for mode in ['authored','guided']:
    r=copy.deepcopy(base);s=r['source'];p=s['path']['controlPoints']
    s['scale']=law([[x,0,0] for x in [1,1,1.125,1,1]],4,2,6)
    s['twist']=law([[x,0,0] for x in [0,0,1/64,0,0]],4,7,11)
    s['axis_scale']=constant([1,1.125,.875]);s['center_law']=constant([1/1024,1/2048,0])
    if mode=='authored':
        s['orientation']='authored';s['frame_axis']=law([[9*(b[k]-a[k]) for k in range(3)] for a,b in zip(p,p[1:])],8,2,6)
        s['frame_normal']=constant([0,0,1])
    else:
        s['orientation']='fixed_normal';g=copy.deepcopy(s['path'])
        for q in g['controlPoints']:q[2]+=1
        s['orientation_guide']=g;r['sections']=129
    cases[mode]=r
r=copy.deepcopy(base);inner=copy.deepcopy(r['source']['profiles']);uv=copy.deepcopy(r['material']['trims'][0])
for c in inner:
    c['controlPoints']=[[x/2 for x in p] for p in c['controlPoints']]
for c in uv:
    c['controlPoints']=[[1+(p[0]-1)/2,4+(p[1]-4)/2] for p in c['controlPoints']]
r['source']['profiles'] += [reverse(c) for c in inner[::-1]]
r['material']['trims'].append([reverse(c) for c in uv[::-1]]);r['loopSizes']=[4,4]
cases['hole']=r
r=copy.deepcopy(r)
r['source']['profiles']=[reverse(c) for a in [r['source']['profiles'][:4],r['source']['profiles'][4:]] for c in a[::-1]]
r['material']['trims']=[[reverse(c) for c in ring[::-1]] for ring in r['material']['trims']]
cases['reversed-hole']=r
r=copy.deepcopy(base);grid=[-3,3];h=F(1,65536);scale=F(1,128)
r['material']['surface']=dict(degreeU=1,degreeV=1,knotsU=[-3,-3,3,3],knotsV=[-3,-3,3,3],
    controlPoints=[[[float(h*u*v),float(scale*u),float(scale*v)] for v in grid] for u in grid],
    weights=[[1,1],[1,1]],periodicU=False,periodicV=False)
signs=[[3,0],[3,3],[0,3],[-3,3],[-3,0],[-3,-3],[0,-3],[3,-3],[3,0]]
profiles=[];trims=[]
def elevate(p): return [p[0],F(p[0]+p[1],2),F(p[0]+4*p[1]+p[2],6),F(p[1]+p[2],2),p[2]]
for i in range(4):
    uv=signs[2*i:2*i+3];u=[p[0] for p in uv];v=[p[1] for p in uv]
    x=[u[0]*v[0],F(u[0]*v[1]+u[1]*v[0],2),F(u[0]*v[2]+4*u[1]*v[1]+u[2]*v[0],6),F(u[1]*v[2]+u[2]*v[1],2),u[2]*v[2]]
    y=elevate(u);z=elevate(v);q=[[h*x[j],scale*y[j],scale*z[j]] for j in range(5)]
    assert all(F(float(v))==v for p in q for v in p)
    profiles.append(law([[float(v) for v in p] for p in q],4));trims.append(law(uv,2,7,11))
r['source']['profiles']=profiles;r['material']['trims']=[trims];cases['nonplanar']=r
r=copy.deepcopy(base);r['sections']=41;cases['forty-intervals']=r

helper=BIN.with_name(BIN.name+'-build');helper.mkdir(exist_ok=True)
manifest='[package]\nname="original-g2-step-probe"\nversion="0.0.0"\nedition="2024"\n[workspace]\n[dependencies]\n'
for name in ['geometry-bridge','value-codec']:
    manifest+=name+'={path='+json.dumps(str(ROOT/'crates'/name))+'}\n'
manifest+='[patch.crates-io]\n'
for name in ['usvg','resvg']:
    manifest+=name+'={path='+json.dumps(str(ROOT/'crates/vendor'/name))+'}\n'
manifest+='[[bin]]\nname="original-g2-step-probe"\npath='+json.dumps(str(OUT/'step_matrix.rs'))+'\n'
(helper/'Cargo.toml').write_text(manifest)
if not (helper/'Cargo.lock').exists():shutil.copyfile(ROOT/'crates/Cargo.lock',helper/'Cargo.lock')
command=['cargo','build','--offline','--manifest-path',str(helper/'Cargo.toml'),'--message-format=json']
run=subprocess.run(command,cwd=ROOT,env={**os.environ,'CARGO_TARGET_DIR':str(TARGET)},text=True,capture_output=True)
(OUT/'probe-build.stderr.log').write_text(run.stderr)
if run.returncode:raise RuntimeError('Native probe build failed; see probe-build.stderr.log')
executables=[m['executable'] for line in run.stdout.splitlines() if line.startswith('{')
    for m in [json.loads(line)] if m.get('reason')=='compiler-artifact' and m.get('executable')]
assert len(executables)==1,executables
shutil.copyfile(executables[0],BIN);BIN.chmod(0o755)
summary={'schema':'original-boundary-g2-additional-native-step-v1','coreInputsSha256':json.loads((OUT.parent/'native-inputs.json').read_text())['inputsSha256'],
    'codeRevision':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
    'rustc':subprocess.check_output(['rustc','--version'],cwd=ROOT,text=True).strip(),
    'runnerSha256':sha(OUT/'step_matrix.rs'),'driverSha256':sha(Path(__file__)),'binarySha256':sha(BIN),
    'externalStepQualified':False,'allPassed':False,'cases':[]}
save(OUT/'step-matrix-summary.json',summary)
for name,r in cases.items():
    fixture=OUT/(name+'.json');save(fixture,r);start=time.time();print('START',name,flush=True)
    run=subprocess.run([str(BIN),str(fixture)],cwd=ROOT,text=True,capture_output=True)
    (OUT/(name+'.stdout.log')).write_text(run.stdout);(OUT/(name+'.stderr.log')).write_text(run.stderr)
    item={'name':name,'fixture':fixture.name,'fixtureSha256':sha(fixture),'exitCode':run.returncode,'elapsedSeconds':round(time.time()-start,3)}
    if run.returncode==0:item['result']=json.loads(run.stdout)
    summary['cases'].append(item);save(OUT/'step-matrix-summary.json',summary);print('DONE',name,run.returncode,flush=True)
summary['allPassed']=all(c['exitCode']==0 for c in summary['cases']);save(OUT/'step-matrix-summary.json',summary)
raise SystemExit(0 if summary['allPassed'] else 1)
