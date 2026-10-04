"""Independent evaluation and pole-face checks for circular transition supports."""
import hashlib
import json
import math
from pathlib import Path
import sys
from OCP.Geom import Geom_BSplineSurface
from OCP.collections import Array2_gp_Pnt as TColgp_Array2OfPnt
from OCP.collections import Array2_double as TColStd_Array2OfReal, Array1_double as TColStd_Array1OfReal, Array1_int as TColStd_Array1OfInteger
from OCP.gp import gp_Pnt
from OCP.BRepBuilderAPI import BRepBuilderAPI_MakeFace
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.GeomLProp import GeomLProp_SLProps

def knot_arrays(values):
    unique = list(dict.fromkeys(values))
    knots = TColStd_Array1OfReal(1,len(unique))
    multiplicities = TColStd_Array1OfInteger(1,len(unique))
    for i,v in enumerate(unique,1):
        knots.SetValue(i,v)
        multiplicities.SetValue(i,values.count(v))
    return knots,multiplicities

def surface(data):
    net = data['controlPoints']
    poles = TColgp_Array2OfPnt(1,len(net),1,len(net[0]))
    weights = TColStd_Array2OfReal(1,len(net),1,len(net[0]))
    for i,row in enumerate(net,1):
        for j,p in enumerate(row,1):
            poles.SetValue(i,j,gp_Pnt(*p))
            weights.SetValue(i,j,data['weights'][i-1][j-1])
    uk,um = knot_arrays(data['knotsU'])
    vk,vm = knot_arrays(data['knotsV'])
    return Geom_BSplineSurface(poles,weights,uk,vk,um,vm,data['degreeU'],data['degreeV'],False,False)

def expected(case,u,v):
    angle=case['start'];sweep=case['sweep'];weight=math.cos(sweep/2)
    basis=[(1-u)**2,2*u*(1-u),u*u]
    denominator=basis[0]+weight*basis[1]+basis[2]
    # Homogeneous circular arc numerator, independently evaluated from angles.
    x=(basis[0]*math.cos(angle)+basis[1]*math.cos(angle+sweep/2)+basis[2]*math.cos(angle+sweep))/denominator
    y=(basis[0]*math.sin(angle)+basis[1]*math.sin(angle+sweep/2)+basis[2]*math.sin(angle+sweep))/denominator
    radius=case['r0']+(case['r1']-case['r0'])*(3*u*u-2*u*u*u)
    vb=[(1-v)**2,2*v*(1-v),v*v];w=math.sqrt(.5)
    vd=vb[0]+w*vb[1]+vb[2]
    radial_fraction=(w*vb[1]+vb[2])/vd
    axial_fraction=(vb[0]+w*vb[1])/vd
    radial=case['outerRadius']+radius*(radial_fraction-1)
    return [radial*x,radial*y,case['height']+radius*(axial_fraction-1)],radius

def main(path):
    data=json.loads(path.read_text());rows=[]
    for case in data['cases']:
        geom=surface(case['surface']);error=0.;normal_error=0.;samples=0
        maker=BRepBuilderAPI_MakeFace(geom,0.,1.,0.,1.,1e-7)
        valid=maker.IsDone() and BRepCheck_Analyzer(maker.Face()).IsValid()
        for i in range(41):
            u=i/40
            for j in range(21):
                v=j/20;target,radius=expected(case,u,v);p=geom.Value(u,v)
                error=max(error,max(abs(a-b) for a,b in zip([p.X(),p.Y(),p.Z()],target)));samples+=1
                if j in [0,20] and radius>1e-5:
                    props=GeomLProp_SLProps(geom,u,v,1,1e-10)
                    assert props.IsNormalDefined()
                    n=props.Normal()
                    alignment=n.Z() if j==0 else (n.X()*p.X()+n.Y()*p.Y())/case['outerRadius']
                    normal_error=max(normal_error,abs(abs(alignment)-1))
        passed=valid and error<1e-9 and normal_error<1e-9
        rows.append(dict(r0=case['r0'],r1=case['r1'],sweep=case['sweep'],faceValid=valid,samples=samples,maxPointErrorMm=error,maxNormalAlignmentError=normal_error,passed=passed))
    report=dict(schema='cad-circular-support-occt/1',sourceSha256=hashlib.sha256(path.read_bytes()).hexdigest(),scope='Independent rational surface evaluation, contact normals and OCCT-created open-face validity. Does not certify a sewn solid, native coedge import, or absence of all intersections.',passed=all(r['passed'] for r in rows),cases=rows)
    path.with_name('occt-support-report.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(passed=report['passed'],cases=len(rows),maxPointErrorMm=max(r['maxPointErrorMm'] for r in rows),maxNormalAlignmentError=max(r['maxNormalAlignmentError'] for r in rows))))
    return 0 if report['passed'] else 1
if __name__=='__main__':
    sys.exit(main(Path(sys.argv[1])))
