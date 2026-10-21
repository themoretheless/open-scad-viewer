"""Exercise the complete native certificate path, including conservative covers."""
import json, pathlib, subprocess, sys, time
checker,fixtures,output=sys.argv[1:]
records=[]
for name in ['curved-through-hole','nonstar-c','two-holes-permuted','sealed-curved-cavity','sheared-through-hole','sheared-nonstar','sheared-sealed-cavity']:
    mesh={'positions':[0,0,0,.25,0,0,.25,.25,0,0,.25,0,0,0,.25,.25,0,.25,.25,.25,.25,0,.25,.25],'indices':[0,2,1,0,3,2,4,5,6,4,6,7,0,1,5,0,5,4,1,2,6,1,6,5,2,3,7,2,7,6,3,0,4,3,4,7]}
    request={'a':{'kind':'brep','model':json.loads((pathlib.Path(fixtures)/(name+'.json')).read_text())},'b':{'kind':'mesh','mesh':mesh},'maximumSourceDeviationMm':100,'meshPrecision':'f32'}
    start=time.monotonic()
    r=subprocess.run([checker,'certify-minkowski-nurbs'],input=json.dumps(request),capture_output=True,text=True,timeout=180)
    record={'name':name,'exitCode':r.returncode,'seconds':time.monotonic()-start,'error':r.stderr}
    if not r.returncode:
        value=json.loads(r.stdout);assert value['sourceApproximationTopology']==[True,True]
        request['certificate']=value['certificate'];record.update(request=request,verified=value)
    records.append(record);pathlib.Path(output).write_text(json.dumps({'records':records},indent=2)+'\n');print(name,r.returncode,record['seconds'],r.stderr,flush=True)
assert all(r['exitCode']==0 for r in records), 'Native full certificate path refused a positive fixture'
