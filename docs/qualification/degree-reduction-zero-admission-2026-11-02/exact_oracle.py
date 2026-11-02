"""Independent binary-rational quadratic vs line comparison, no CAD imports."""
from fractions import Fraction as F
import json, math
p=[F.from_float(x) for x in [1.,1.,math.nextafter(1.,math.inf)]]
t=F(1,2)
source=(1-t)**2*p[0]+2*t*(1-t)*p[1]+t*t*p[2]
line=(1-t)*p[0]+t*p[2]
assert line-source==F(1,2**54)
assert F.from_float(float((p[0]+p[2])/2))==p[1]
assert float(source)==float(line)==1.
print(json.dumps({'exactSourceMidpoint':str(source),'exactLineMidpoint':str(line),'nominalDeviation':str(line-source),'roundedMidpointsEqual':True,'roundedReelevationMatchesSourcePoles':True},indent=2))
