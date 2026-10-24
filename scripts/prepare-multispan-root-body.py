"""Author an exact dyadic, independently partitioned root-chain fixture.
This is test data construction, never production geometry or CAD admission.
"""
import gzip
import json
import sys
from fractions import Fraction
from pathlib import Path
source=Path(sys.argv[1]);output=Path(sys.argv[2])
request=json.loads(gzip.decompress(source.read_bytes()))
count=0
def partition(value):
    global count
    if isinstance(value,dict):
        if value.get('degree')==1 and len(value.get('controlPoints',[]))==2:
            assert value['weights']==[1.,1.] and value['knots']==[0.,0.,1.,1.]
            a,b=value['controlPoints'];assert len(a)==len(b) and len(a) in (2,3)
            parameters=[Fraction(0),Fraction(1,4),Fraction(3,4),Fraction(1)] if len(a)==2 else [Fraction(0),Fraction(1,2),Fraction(1)]
            poles=[]
            for t in parameters:
                pole=[]
                for x,y in zip(a,b):
                    exact=(1-t)*Fraction(x)+t*Fraction(y);number=float(exact)
                    assert Fraction(number)==exact,'Fixture must have exact binary64 poles'
                    pole.append(number)
                poles.append(pole)
            value['controlPoints']=poles;value['weights']=[1.]*len(poles)
            value['knots']=[0.,0.]+[float(t) for t in parameters[1:-1]]+[1.,1.]
            count+=1
        else:
            for child in value.values():partition(child)
    elif isinstance(value,list):
        for child in value:partition(child)
partition(request)
for index in [5,6]:
    edge=request['definition']['shell']['pairs'][index]['edge']
    assert edge['recipe']['kind']=='mapped'
    candidates=[]
    for side,use in enumerate(edge['uses']):
        candidates.append([[.25 if side==0 else .75,.5] if end['kind']=='crossing' else None for end in use['endpoints']])
    edge['recipe']['candidates']=candidates
with output.open('x') as file:json.dump(request,file,separators=(',',':'))
print(json.dumps({'authoredCurves':count,'qualification':False,'output':str(output)}))
