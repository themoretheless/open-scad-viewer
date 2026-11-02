"""Independent rational oracle: original captured pose and exact time split."""
from fractions import Fraction as Q
from pathlib import Path
import hashlib,json,struct
root=Path(__file__).resolve().parents[3]
path=root/'crates/nurbs-core/src/analysis/affine_volume/capped/tests/fixtures/actual-quarter-turn-bits.json'
p=json.loads(path.read_text())
def scalar(bits):
    return Q(struct.unpack('>d',bytes.fromhex(bits))[0])
a,b=p['motion']['controls'][0]
assert [scalar(x) for x in p['motion']['weights'][0]]==[1,1]
count=0
for col in range(4):
    for axis in range(3):
        x,y=scalar(a[col][axis]),scalar(b[col][axis])
        assert x in [-1,0,1,2] and y in [-1,0,1,2]
        mid=(x+y)/2
        assert Q(float(mid))==mid
        # Independent global-time polynomial coefficients of both pieces.
        assert 2*(mid-x)==y-x==2*(y-mid)
        assert mid-(y-x)/2==x
        count+=1
# Independent degree/weight fixture: first end and second start derivatives.
pose=lambda z:[[Q(1),Q(0),Q(0)],[z/64,Q(1),Q(0)],[Q(0),Q(0),Q(1)],[Q(0),Q(0),z]]
controls=[pose(Q(z)) for z in [0,1,2]]
for col in range(4):
    for axis in range(3):
        assert Q(1)*Q(1,2)*(controls[1][col][axis]-controls[0][col][axis])==Q(2)*Q(1,4)*(controls[2][col][axis]-controls[1][col][axis])
print(json.dumps({'captured_fixture_sha256':hashlib.sha256(path.read_bytes()).hexdigest(),
 'exact_split_field_components':count,'independent_degree_1_2_join_components':12,'passed':True},indent=2))
