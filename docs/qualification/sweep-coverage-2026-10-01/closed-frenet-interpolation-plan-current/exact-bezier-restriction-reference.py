"""Independent exact-rational oracle; never used by native admission."""
from fractions import Fraction as F

def blossom(points, parameters):
    d = list(points)
    for t in parameters:
        d = [(1-t)*a+t*b for a,b in zip(d,d[1:])]
    return d[0]

def split(points, t):
    d=list(points);left=[d[0]];right=[d[-1]]
    while len(d)>1:
        d=[(1-t)*a+t*b for a,b in zip(d,d[1:])]
        left.append(d[0]);right.append(d[-1])
    return left,right[::-1]

def restrict(points,a,b):
    if b==0:return [points[0]]*len(points)
    left,_=split(points,b)
    return split(left,a/b)[1] if a else left

def self_test():
    count=0
    for degree in range(14):
        for controls in [[F((i*7)%11-5,8) for i in range(degree+1)],
            [F(i+1,degree+2) for i in range(degree+1)],
            [F(int(i==degree)) for i in range(degree+1)]]:
            for a,b in [(F(0),F(0)),(F(0),F(1)),(F(1),F(1)),
                (F(1,4),F(3,4)),(F(1,3),F(4,5)),(F(1,2),F(1,2))]:
                expected=[blossom(controls,[a]*(degree-k)+[b]*k) for k in range(degree+1)]
                assert restrict(controls,a,b)==expected,(degree,a,b)
                assert all(min(controls)<=v<=max(controls) for v in expected)
                count+=1
    return count

if __name__=='__main__':print(self_test(),'exact rational identities passed')
