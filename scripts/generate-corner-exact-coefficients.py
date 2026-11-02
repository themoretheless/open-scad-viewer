#!/usr/bin/env python3
"""Generate exact rational Bernstein pullback coefficients, not sampled geometry."""
from fractions import Fraction as F
from math import comb,lcm
N=7

def elevate(p,n):
 p=list(map(F,p))
 while len(p)<=n:
  d=len(p);q=[F(0)]*(d+1);q[0]=p[0];q[d]=p[-1]
  for i in range(1,d):q[i]=F(i,d)*p[i-1]+(1-F(i,d))*p[i]
  p=q
 return p

def pc(p,n):return [sum(F(p[k])*F(comb(i,k),comb(n,k)) for k in range(min(i,len(p)-1)+1)) for i in range(n+1)]
def prod(a,b):
 p=len(a)-1;q=len(b)-1;c=[[F(0)]*(p+q+1) for _ in range(p+q+1)]
 for i in range(p+1):
  for j in range(p+1):
   for k in range(q+1):
    for l in range(q+1):c[i+k][j+l]+=a[i][j]*b[k][l]*F(comb(p,i)*comb(q,k),comb(p+q,i+k))*F(comb(p,j)*comb(q,l),comb(p+q,j+l))
 return c
nodes=[[1,0,0],[0,1,0],[0,0,1],[0,F(1,2),F(1,2)],[F(1,2),0,F(1,2)],[F(1,2),F(1,2),0],[F(1,4),F(3,8),F(3,8)]]
roles=[[0,5,6,4],[1,3,6,5],[2,4,6,3]]
out=[]
for R,h in [(1,0),(0,1)]:
 net={}
 for m in range(3):
  for layer in range(3):
   d=7-layer;y=elevate([0,0,0,R-2*h,R-h,R],d);x=y[::-1];la=pc([5*h,5*h,5*h,-20*h,10*h],d);mu=pc([0,10*h,20*h,-60*h,30*h],d)
   for i in range(d+1):
    role=[0]*3;role[m]=layer;role[(m+1)%3]=i;role[(m+2)%3]=d-i
    z=[F(R),R-la[i]/7,R-2*la[i]/7+mu[i]/42][layer];pt=[F(0)]*3;pt[m]=z;pt[(m+1)%3]=x[i];pt[(m+2)%3]=y[i]
    if tuple(role) in net:assert net[tuple(role)]==pt
    net[tuple(role)]=pt
 caps=[]
 for quad in roles:
  maps=[[[nodes[quad[0]][k],nodes[quad[3]][k]],[nodes[quad[1]][k],nodes[quad[2]][k]]] for k in range(3)]
  cp=[[[F(0)]*3 for _ in range(8)] for _ in range(8)]
  for ex,pt in net.items():
   p=[[F(1)]]
   for axis in range(3):
    for _ in range(ex[axis]):p=prod(p,maps[axis])
   factor=comb(7,ex[0])*comb(7-ex[0],ex[1])
   for i in range(8):
    for j in range(8):
     for k in range(3):cp[i][j][k]+=factor*p[i][j]*pt[k]
  caps.append(cp)
 out.append(caps)
D=1
for caps in out:
 for cp in caps:
  for row in cp:
   for p in row:
    for c in p:D=lcm(D,c.denominator)
assert D == 2205 * (1 << 19)
from pathlib import Path
coeff=[]
for a,b in zip(out[0],out[1]):
 coeff.append([[[[int(x*D),int(y*D)] for x,y in zip(pa,pb)] for pa,pb in zip(ra,rb)] for ra,rb in zip(a,b)])
p=Path(__file__).resolve().parents[1]/'crates/brep-core/src/region_replacement/corner_patch_network/exact_coefficients.rs'
p.write_text('//! Exact linear cap coefficients: denominator 2205 * 2^19.\n//! Regenerate with scripts/generate-corner-exact-coefficients.py.\npub(super) const COEFFICIENTS: [[[[[i64;2];3];8];8];3] = '+repr(coeff).replace(' ','')+';\n')
