#!/usr/bin/env python3
"""Imported declared controls -> current native fillet/STEP. No UI/WASM delivery claim."""
import hashlib
import json
import math
from pathlib import Path
import subprocess
import sys


def digest(file):
    return hashlib.sha256(file.read_bytes()).hexdigest()


def main(inputs_path, executable, output):
    inputs_bytes = inputs_path.read_bytes()
    inputs = json.loads(inputs_bytes)
    names = {'bracket','flange','enclosure'}
    if (inputs.get('schema') != 'cad-native-bridge-part-inputs/1' or
            len(inputs.get('parts', [])) != 3 or {p['name'] for p in inputs['parts']} != names):
        raise ValueError('Expected the three declared source/placed B-rep controls')
    executable = executable.resolve(strict=True)
    executable_hash = digest(executable)
    output.mkdir()
    parts, chains = [], []

    def call(request):
        process = subprocess.run([str(executable)], input=json.dumps(request,allow_nan=False),
                                 text=True, capture_output=True, check=True, timeout=120)
        response = json.loads(process.stdout)
        if response.get('ok') is not True:
            raise RuntimeError(str(response.get('error', response)))
        return response['value']

    for part in inputs['parts']:
        name = part['name']
        op = {'bracket':'simple_prism','flange':'annular','enclosure':'layered_prism'}[name]
        rounded = call(dict(op='brep_nurbs_exact_'+op+'_fillet',model=part['placed'],edges=part['edges'],radius=1))
        if (not rounded['audit']['ok'] or not rounded['certificate']['complete'] or not rounded['namingComplete']):
            raise RuntimeError('Native feature not admitted: '+name)
        removed = 2*math.pi*(19*(1-math.pi/4)+1/6) if name=='flange' else 20*(1-math.pi/4)
        for stage, model in [('source',part['source']),('edited-rounded',rounded['model'])]:
            result = call(dict(op='brep_nurbs_export_step_v9',model=model))
            text = result['text']
            file = output/(name+'-'+stage+'.step')
            with file.open('x') as stream:
                stream.write(text)
            shift = [0,0,0] if stage=='source' else [3,-2,1]
            expected = dict(boundsMm=[[v+d for v,d in zip(p,shift)] for p in part['expectedBounds']],
                            volumeMm3=part['expectedVolume']-(removed if stage!='source' else 0),solids=1)
            parts.append(dict(name=name+'-'+stage,file=file.name,sha256=digest(file),expected=expected))
        with (output/(name+'-native-feature.json')).open('x') as stream:
            json.dump(rounded,stream,allow_nan=False)
        chains.append(dict(part=name,selectedBrepEdges=part['edges'],capability=rounded['certificate']['capability'],
                           namingComplete=True,operations=['import declared B-rep JSON','native fillet','native AP242 STEP export']))
        print(json.dumps(dict(part=name,nativeFeatureAdmitted=True)),flush=True)
    if digest(executable)!=executable_hash or inputs_path.read_bytes()!=inputs_bytes:
        raise RuntimeError('Native executable or inputs changed during qualification')
    manifest = dict(schema='fresh-cad-parts-step/1',geometryBackend='native-bridge-executable',
                    nativeExecutableSha256=executable_hash,inputWasmSha256=inputs['inputWasmSha256'],
                    inputSha256=hashlib.sha256(inputs_bytes).hexdigest(),producerSha256=digest(Path(__file__)),
                    toleranceMm=1e-5,relativeVolumeTolerance=1e-7,parts=parts,chains=chains,
                    independentStepQualified=False,browserQualified=False,historyQualified=False,
                    scope='Imported controls and placement from stamped WASM; edits/export executed by current native binary. No WASM rebuild, browser or history qualification.')
    with (output/'manifest.json').open('x') as stream:
        json.dump(manifest,stream,indent=2,allow_nan=False)
    print(json.dumps(dict(parts=3,stepFiles=len(parts),nativeExecutableSha256=executable_hash)))


if __name__=='__main__':
    main(Path(sys.argv[1]).resolve(),Path(sys.argv[2]),Path(sys.argv[3]).resolve())
