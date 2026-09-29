"""Independent evaluation of the exported periodic rational rebuild fixture."""
import hashlib
import json
import math
from pathlib import Path
import sys

path = Path(sys.argv[1])
source = json.loads(path.with_name('periodic-canceled.json').read_text())['curves'][0]['curve']
target = json.loads(path.read_text())['curves'][0]['curve']

def evaluate(curve, u):
    knots, degree, points, weights = (curve[k] for k in ['knots', 'degree', 'controlPoints', 'weights'])
    count = len(points)
    # Cox-de Boor, with the left limit at the active upper boundary.
    values = [float(knots[i] <= u < knots[i+1]) for i in range(len(knots)-1)]
    if u == knots[count]:
        values = [0.] * (len(knots)-1)
        span = count-1
        while knots[span] == u:
            span -= 1
        values[span] = 1.
    for p in range(1, degree+1):
        result = []
        for i in range(len(values)-1):
            a, b = knots[i+p]-knots[i], knots[i+p+1]-knots[i+1]
            result.append(((u-knots[i])/a*values[i] if a else 0.)
                          + ((knots[i+p+1]-u)/b*values[i+1] if b else 0.))
        values = result
    w = sum(b*w for b,w in zip(values,weights))
    assert w > 0
    return [sum(b*w*p[k] for b,w,p in zip(values,weights,points))/w for k in range(3)]

assert target['periodic'] and target['degree'] == 3
assert len(target['controlPoints']) == 15
assert target['controlPoints'][-3:] == target['controlPoints'][:3]
assert target['weights'][-3:] == target['weights'][:3]
a, b = source['knots'][source['degree']], source['knots'][len(source['controlPoints'])]
assert [target['knots'][target['degree']], target['knots'][len(target['controlPoints'])]] == [a,b]
maximum = max(math.dist(evaluate(source, a+(b-a)*i/8192), evaluate(target, a+(b-a)*i/8192)) for i in range(8193))
seam = math.dist(evaluate(target,a), evaluate(target,b))
bound=json.loads(path.with_name('periodic-rebuild-browser.json').read_text())['deviationUpperMm']
assert maximum <= bound <= .2 and seam < 1e-12
report = {'sha256': hashlib.sha256(path.read_bytes()).hexdigest(), 'samples': 8193,
          'maxSampledDeviationMm': maximum, 'reportedDeviationUpperMm': bound, 'seamPositionResidualMm': seam,
          'oracle': 'Independent Python rational Cox-de Boor evaluation',
          'independentContinuousCertificate': False}
path.with_name('independent-periodic-report.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
