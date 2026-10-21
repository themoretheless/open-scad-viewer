#!/usr/bin/env python3
"""Independent STEP volume, radius and sampled G2 checks of the quarter control."""
import hashlib
import json
import math
import sys
from fractions import Fraction
from pathlib import Path
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.Bnd import Bnd_Box
from OCP.BRepBndLib import BRepBndLib
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_SOLID, TopAbs_FACE, TopAbs_EDGE, TopAbs_VERTEX
from OCP.TopoDS import TopoDS
from OCP.BRep import BRep_Tool
from OCP.gp import gp_Pnt, gp_Vec


def entities(shape, kind):
    explorer=TopExp_Explorer(shape,kind)
    found=[]
    while explorer.More():
        item=explorer.Current()
        if not any(item.IsSame(old) for old in found):
            found.append(item)
        explorer.Next()
    return found


def xyz(v):
    return [v.X(),v.Y(),v.Z()]


def dot(a,b):
    return sum(x*y for x,y in zip(a,b))


def cross(a,b):
    return [a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]


def shape_tensor(surface,u,v):
    p=gp_Pnt()
    du,dv,duu,dvv,duv=[gp_Vec() for _ in range(5)]
    surface.D2(u,v,p,du,dv,duu,dvv,duv)
    du,dv,duu,dvv,duv=map(xyz,[du,dv,duu,dvv,duv])
    area_vector=cross(du,dv)
    area=math.sqrt(dot(area_vector,area_vector))
    assert area>0
    n=[x/area for x in area_vector]
    dual=[[x/area for x in cross(dv,n)],[x/area for x in cross(n,du)]]
    second=[[dot(n,duu),dot(n,duv)],[dot(n,duv),dot(n,dvv)]]
    tensor=[sum(dual[r][a]*second[r][s]*dual[s][b] for r in range(2) for s in range(2))
            for a in range(3) for b in range(3)]
    return n,tensor


def main(root):
    manifest=json.loads((root/'manifest.json').read_text())
    for name,key in [('transition.step','stepSha256'),('model.json','modelSha256')]:
        assert hashlib.sha256((root/name).read_bytes()).hexdigest()==manifest[key]
    reader=STEPControl_Reader()
    assert reader.ReadFile(str(root/'transition.step'))==IFSelect_RetDone and reader.TransferRoots()
    shape=reader.OneShape()
    assert not shape.IsNull() and BRepCheck_Analyzer(shape).IsValid()
    topology={name:len(entities(shape,kind)) for name,kind in [('solids',TopAbs_SOLID),('faces',TopAbs_FACE),('edges',TopAbs_EDGE),('vertices',TopAbs_VERTEX)]}
    splits=manifest.get('retainedContactSplitCount',0)
    assert isinstance(splits,int) and 0<=splits<=126
    expected_topology={'solids':1,'faces':11,'edges':21+splits,'vertices':12+splits}
    assert topology==expected_topology
    assert all(manifest['expected'][key]==value for key,value in expected_topology.items())
    r0,r1=map(Fraction,map(str,manifest['radiiMm']))
    width=Fraction(str(manifest['transitionWidthMm']))
    # Integrate the authored quintic law exactly, without fitting output geometry.
    profile=[r0,Fraction(0),Fraction(0),10*(r1-r0),-15*(r1-r0),6*(r1-r0)]
    integral=sum(a*b/Fraction(i+j+1) for i,a in enumerate(profile) for j,b in enumerate(profile))
    expected_volume=math.pi/4*float(width*(r0*r0+r1*r1+integral))
    props=GProp_GProps()
    integration_error=BRepGProp.VolumeProperties_s(shape,props,1e-11,True)
    volume_error=abs(props.Mass()-expected_volume)
    assert volume_error<=1e-9
    box=Bnd_Box()
    BRepBndLib.AddOptimal_s(shape,box,False,False)
    bounds=[xyz(box.CornerMin()),xyz(box.CornerMax())]
    bound_error=max(abs(a-b) for row,target in zip(bounds,manifest['expected']['boundsMm']) for a,b in zip(row,target))
    assert bound_error<=2e-7
    skins=[]
    for face in entities(shape,TopAbs_FACE):
        surface=BRep_Tool.Surface_s(TopoDS.Face_s(face))
        if hasattr(surface,'UDegree') and surface.UDegree()==2 and surface.VDegree() in [1,5] and (surface.IsURational() or surface.IsVRational()):
            skins.append(surface)
    assert len(skins)==3
    skins.sort(key=lambda s:s.Value(.5,.5).Z())
    radial_error=0.
    for index,surface in enumerate(skins):
        for i in range(13):
            for j in range(13):
                p=surface.Value(i/12,j/12)
                t=p.Z()/float(width)
                radius=float(r0) if index==0 else float(r1) if index==2 else float(r0)+float(r1-r0)*(10*t**3-15*t**4+6*t**5)
                radial_error=max(radial_error,abs(math.hypot(p.X(),p.Y())-radius))
    normal_error=tensor_error=0.
    for first,at_first,second,at_second in [(skins[0],1.,skins[1],0.),(skins[1],1.,skins[2],0.)]:
        for i in range(25):
            a=shape_tensor(first,i/24,at_first)
            b=shape_tensor(second,i/24,at_second)
            normal_error=max(normal_error,math.sqrt(sum((x-y)**2 for x,y in zip(a[0],b[0]))))
            tensor_error=max(tensor_error,math.sqrt(sum((x-y)**2 for x,y in zip(a[1],b[1]))))
    assert radial_error<=1e-7 and normal_error<=1e-5 and tensor_error<=1e-4
    assert manifest['nativeTransitionAdmitted'] and manifest['nativeClosureWithinTolerance'] and manifest['trimRegionsValid']
    assert manifest['circularRadiusErrorUpperMm']<=1e-7
    assert manifest['stationPlaneErrorUpperMm']<=1e-7 and manifest['widthErrorUpperMm']<=1e-7
    assert all(all(a<=b for a,b in zip(upper,[1e-7,1e-5,1e-4])) for upper in manifest['seamUpper'])
    assert not manifest['wholeBodyG2Claim'] and not manifest['generalCornerClaim']
    report=dict(valid=True,topology=topology,volumeMm3=props.Mass(),expectedVolumeMm3=expected_volume,
        volumeErrorMm3=volume_error,integrationErrorEstimate=integration_error,boundsMm=bounds,
        maximumBoundsErrorMm=bound_error,sampledRadiusErrorMm=radial_error,
        sampledNormalChord=normal_error,sampledWorldTensorDifference=tensor_error,
        radiusSamples=507,seamSamples=50,wholeBodyG2Claim=False,generalCornerClaim=False,
        wholeProofSource='native original coefficients; independent STEP samples supplement it',
        stepSha256=manifest['stepSha256'],passed=True)
    (root/'independent-occt.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report,indent=2))


if __name__=='__main__':
    main(Path(sys.argv[1]))
