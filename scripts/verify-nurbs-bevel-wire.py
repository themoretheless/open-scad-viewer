#!/usr/bin/env python3
"""Sample retained bevel wire against an independent Decimal source-normal oracle.
No continuous or region topology certificate is claimed by this sampled oracle.
"""
import importlib.util
import json
import sys
from pathlib import Path
from decimal import Decimal, localcontext
spec = importlib.util.spec_from_file_location('offset_oracle', Path(__file__).with_name('verify-nurbs-offset.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
number, evaluation = module.number, module.evaluation

def verify(request, response):
    assert response['ok'] is True
    result = response['value']
    curves, report = result['curves'], result['report']
    cells = report['cells']
    assert report['wholeWire'] is True and report['wholeCurve'] is False
    assert report['regionTrimmed'] is False and report['regionTopologyCertified'] is False
    assert report['method'] == 'outward-source-offset-bevel-wire/1'
    assert number(report['errorUpperMm']) <= number(request['toleranceMm'])
    assert curves[0]['knots'][0] == 0 and curves[-1]['knots'][-1] == len(cells)
    if report['closed']:
        assert curves[0]['controlPoints'][0] == curves[-1]['controlPoints'][-1]
    for a, b in zip(curves, curves[1:]):
        assert a['knots'][-1] == b['knots'][0]
        assert a['controlPoints'][-1] == b['controlPoints'][0]
    for curve in curves:
        assert curve['degree'] == 1 and len(curve['controlPoints']) <= 256
        assert all(w == 1 for w in curve['weights'])
    source, distance = request['curve'], number(request['distance'])
    def offset(t, side):
        point, derivative = evaluation(source, t, side)
        speed = sum(x*x for x in derivative).sqrt()
        assert speed > 0
        return [point[0]-distance*derivative[1]/speed, point[1]+distance*derivative[0]/speed]
    samples, maximum, chunk = 0, Decimal(0), 0
    for i, cell in enumerate(cells):
        assert cell['domain'] == [i, i+1] and cell['stationDomain'] == cell['domain']
        assert number(cell['errorUpperMm']) <= number(report['errorUpperMm'])
        role = cell['source']
        for station in range(9):
            fraction = Decimal(station)/8
            parameter = Decimal(i)+fraction
            while parameter > number(curves[chunk]['knots'][-1]):
                chunk += 1
            if role['kind'] == 'source-offset':
                a, b = map(number, role['domain'])
                expected = offset(a+(b-a)*fraction, 'left' if station == 8 else 'right')
            else:
                assert role['kind'] == 'bevel'
                knot = number(role['sourceKnot'])
                a, b = offset(knot, 'left'), offset(knot, 'right')
                expected = [x+(y-x)*fraction for x, y in zip(a, b)]
            actual, _ = evaluation(curves[chunk], parameter)
            error = sum((a-b)**2 for a, b in zip(actual, expected)).sqrt()
            assert error <= number(cell['errorUpperMm']), (i, station, error, cell)
            maximum = max(maximum, error)
            samples += 1
    return {'oracle':'Decimal(70) independent rational one-sided source-normal/bevel evaluation',
            'samples':samples,'cells':len(cells),'curves':len(curves),
            'maxSampledErrorMm':float(maximum),'errorUpperMm':report['errorUpperMm'],
            'closed':report['closed'],'continuousCertificateFromIndependentOracle':False,
            'regionTopologyCertified':False}
if __name__ == '__main__':
    request, response = [json.load(open(p, encoding='utf8')) for p in sys.argv[1:3]]
    with localcontext() as context:
        context.prec = 70
        print(json.dumps(verify(request, response), indent=2))
