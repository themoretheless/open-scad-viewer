from fractions import Fraction as F
from math import comb
from pathlib import Path
import json
cases=0;basis_cases=0;max_integer_numerator=0
for power in [1,2]:
 for k in range(12):
  for n in range(1,25):
   for m in range(1,26):
    if power==2 and m%2==0:continue
    d=m+power*(n+k)
    if d>25:continue
    w=[[F(0) for _ in range(k+1)] for _ in range(d+1)]
    for i in range(k+1):
     r=m+power*i;q=n+k-i
     for j in range(q+1 if power==2 else 1):
      num=comb(k,i)*comb(q,j)*2**j
      max_integer_numerator=max(max_integer_numerator,num)
      w[r+j][i]+=F(num,comb(d,r+j))
    for i in range(k+1):
     got=[F(0) for _ in range(d+1)];expected=[F(0) for _ in range(d+1)]
     for a in range(d+1):
      for j in range(d-a+1):got[a+j]+=w[a][i]*comb(d,a)*comb(d-a,j)*(-1)**j
     for j in range(n+k-i+1):expected[m+power*(i+j)]=F(comb(k,i)*comb(n+k-i,j)*(-1)**j)
     assert got==expected,(power,m,n,k,i)
     basis_cases+=1
    assert all(0<=sum(row)<=1 for row in w)
    x=[F(0) for _ in range(d+1)]
    for a in range(d+1):
     c=F(comb(a,power) if a>=power else 0,comb(d,power))
     for j in range(d-a+1):x[a+j]+=c*comb(d,a)*comb(d-a,j)*(-1)**j
    assert x==[F(int(i==power)) for i in range(d+1)]
    cases+=1
report={'status':'exact-algebraic-audit-only-not-native-qualification','model':'u=t^power; ordinate=t^m (1-t^power)^n sum a_i B_i^k(t^power)','parameterPower':'1 for integer n1; 2 for noninteger half-integer n1; m=power*n1','degreeMaximum':25,'cases':cases,'independentCoefficientIdentities':basis_cases,'coefficientWeightsNonnegativeAndSumAtMostOne':True,'maxIntegerNumerator':max_integer_numerator,'allIntegerFactorsFitBinary64Exactly':max_integer_numerator<2**53,'closedLoopControlsMaximum':51,'constantShapeDegreeReduction':'All equal coefficients give a degree-zero shape by partition of unity','countedNativeCapabilities':0,'remainingQualification':['Rust source and output bindings','Outward arithmetic proof and shared work enforcement','Nominal position bound excludes evaluator rounding, Gk and topology']}
p=Path(__file__).with_name('cst-algebraic-identity-audit-20261102.json');p.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
