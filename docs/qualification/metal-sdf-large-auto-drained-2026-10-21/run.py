#!/usr/bin/env python3
import pathlib, subprocess, os, sys, json, time
root=pathlib.Path(__file__).resolve().parent
cases=['sphere-gpu-64','sphere-auto-64','tetra-auto-48','tetra-auto-49','mesh84-auto-32','mesh86-auto-32','cross-auto-32']
for phase in ['controls','full']:
    for case in cases:
        directory=root/f'{phase}-{case}-{time.time_ns()}'
        directory.mkdir()
        (directory/'qualify.py').write_text("from pathlib import Path\nsource = Path(__file__).resolve().parent.parent / 'qualify-template.py'\nexec(compile(source.read_text(), str(source), 'exec'))\n")
        env=dict(os.environ,SDF_CASE=case,SDF_PHASE=phase)
        subprocess.run([sys.executable,str(directory/'qualify.py'),sys.argv[1]],env=env,check=True)
        result=json.loads((directory/'decision.json').read_text())
        print(phase,case,result,flush=True)
        if not result['accepted']:
            sys.exit(2)
