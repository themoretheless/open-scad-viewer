"""Independent STEP validity, volume and bounds for a saved acceptance manifest."""
import hashlib,json,sys
from pathlib import Path
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepBndLib import BRepBndLib
from OCP.Bnd import Bnd_Box
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_SOLID

def main(directory):
    manifest=json.loads((directory/'manifest.json').read_text());rows=[]
    for part in manifest['parts']:
        file=directory/part['file'];digest=hashlib.sha256(file.read_bytes()).hexdigest()
        assert digest==part['sha256']
        reader=STEPControl_Reader();assert reader.ReadFile(str(file))==IFSelect_RetDone and reader.TransferRoots()>0
        shape=reader.OneShape();valid=not shape.IsNull() and BRepCheck_Analyzer(shape).IsValid()
        box=Bnd_Box();BRepBndLib.AddOptimal_s(shape,box,False,False)
        bounds=[[q.X(),q.Y(),q.Z()] for q in [box.CornerMin(),box.CornerMax()]]
        props=GProp_GProps();BRepGProp.VolumeProperties_s(shape,props,1e-10)
        volume=props.Mass();expected=part['expected'];volume_error=abs(volume-expected['volumeMm3'])
        bounds_error=max(abs(a-b) for actual,wanted in zip(bounds,expected['boundsMm']) for a,b in zip(actual,wanted))
        solids=0;explorer=TopExp_Explorer(shape,TopAbs_SOLID)
        while explorer.More():solids+=1;explorer.Next()
        passed=valid and solids==expected.get('solids',1) and bounds_error<=manifest['toleranceMm'] and volume_error<=max(1e-6,abs(expected['volumeMm3'])*manifest['relativeVolumeTolerance'])
        rows.append(dict(name=part['name'],sha256=digest,valid=valid,solids=solids,boundsMm=bounds,volumeMm3=volume,maxBoundsErrorMm=bounds_error,volumeErrorMm3=volume_error,passed=passed))
    report=dict(schema='cad-step-independent-metrics/1',scope='Saved manifest geometry: OCCT solid validity, bounds and volume; numerical section gauges are separate',passed=all(r['passed'] for r in rows),parts=rows)
    with (directory/'metrics-report.json').open('x') as f:json.dump(report,f,indent=2,allow_nan=False)
    print(json.dumps(report));return 0 if report['passed'] else 1
if __name__=='__main__':sys.exit(main(Path(sys.argv[1])))
