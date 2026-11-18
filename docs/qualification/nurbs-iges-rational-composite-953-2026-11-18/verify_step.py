import json,math,sys
from pathlib import Path
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.TopAbs import TopAbs_SOLID
from OCP.TopExp import TopExp_Explorer
root=Path(sys.argv[1])
def measure(reader,file):
    assert reader.ReadFile(str(root/file))==IFSelect_RetDone and reader.TransferRoots()>0
    explorer=TopExp_Explorer(reader.OneShape(),TopAbs_SOLID)
    volumes=[]
    while explorer.More():
        shape=explorer.Current()
        assert BRepCheck_Analyzer(shape).IsValid()
        props=GProp_GProps()
        BRepGProp.VolumeProperties_s(shape,props,1e-9,True)
        volumes.append(props.Mass())
        explorer.Next()
    assert len(volumes)==1
    return volumes[0]
step=measure(STEPControl_Reader(),'restored.step')
assert abs(step-45*math.pi)<1e-6
print(json.dumps(dict(step=step,analytic=45*math.pi)),flush=True)
