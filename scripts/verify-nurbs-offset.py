#!/usr/bin/env python3
"""Independent Decimal Cox-de Boor oracle for a bounded native offset chain.

Usage: verify-nurbs-offset.py request.json result.json
This samples the reported cells; it does not independently certify the continuum.
"""
import json
import sys
from decimal import Decimal, localcontext
from functools import lru_cache


def number(value):
    return Decimal.from_float(value) if isinstance(value, float) else Decimal(value)


def evaluation(curve, parameter, side="right"):
    knots = [number(x) for x in curve['knots']]
    degree = curve['degree']
    if curve.get('periodic', False):
        start, end = knots[degree], knots[len(curve['controlPoints'])]
        parameter = start + (parameter - start) % (end - start)
        if side == "left" and parameter == start:
            parameter = end
    controls = [[number(x) for x in p] for p in curve['controlPoints']]
    weights = [number(x) for x in curve['weights']]

    @lru_cache(None)
    def basis(i, p):
        if p == 0:
            active = knots[i] < parameter <= knots[i + 1] if side == "left" else knots[i] <= parameter < knots[i + 1]
            if parameter == knots[-1]:
                active = knots[i] < parameter <= knots[i + 1]
            return Decimal(int(active))
        result = Decimal(0)
        a, b = knots[i + p] - knots[i], knots[i + p + 1] - knots[i + 1]
        if a:
            result += (parameter - knots[i]) / a * basis(i, p - 1)
        if b:
            result += (knots[i + p + 1] - parameter) / b * basis(i + 1, p - 1)
        return result

    denominator, derivative_denominator = Decimal(0), Decimal(0)
    numerator, derivative_numerator = [Decimal(0)] * 2, [Decimal(0)] * 2
    for i, (control, weight) in enumerate(zip(controls, weights)):
        n, dn = basis(i, degree), Decimal(0)
        a, b = knots[i + degree] - knots[i], knots[i + degree + 1] - knots[i + 1]
        if a:
            dn += degree / a * basis(i, degree - 1)
        if b:
            dn -= degree / b * basis(i + 1, degree - 1)
        denominator += n * weight
        derivative_denominator += dn * weight
        for axis in range(2):
            numerator[axis] += n * weight * control[axis]
            derivative_numerator[axis] += dn * weight * control[axis]
    point = [p / denominator for p in numerator]
    derivative = [(dp - p * derivative_denominator) / denominator
                  for p, dp in zip(point, derivative_numerator)]
    return point, derivative


def verify(request, response):
    assert response['ok'] is True
    result = response['value']
    curves, report = result['curves'], result['report']
    assert report['wholeCurve'] is True and report['regionTopologyCertified'] is False
    assert number(report['errorUpperMm']) <= number(request['toleranceMm'])
    source = request['curve']
    assert curves[0]['knots'][0] == source['knots'][source['degree']]
    assert curves[-1]['knots'][-1] == source['knots'][len(source['controlPoints'])]
    if report.get('closed'):
        assert curves[0]['controlPoints'][0] == curves[-1]['controlPoints'][-1]
    for a, b in zip(curves, curves[1:]):
        assert a['knots'][-1] == b['knots'][0]
        assert a['controlPoints'][-1] == b['controlPoints'][0]
    for curve in curves:
        assert curve['degree'] == 1 and len(curve['controlPoints']) <= 256
        assert all(w == 1 for w in curve['weights'])
    samples, maximum = 0, Decimal(0)
    distance = number(request['distance'])
    chunk = 0
    previous = None
    for cell in report['cells']:
        lo, hi = map(number, cell['domain'])
        assert lo < hi
        if previous is not None:
            assert lo == previous
        previous = hi
        assert number(cell['errorUpperMm']) <= number(report['errorUpperMm'])
        for station in range(9):
            parameter = lo + (hi - lo) * Decimal(station) / 8
            while parameter > number(curves[chunk]['knots'][-1]):
                chunk += 1
            point, derivative = evaluation(source, parameter)
            speed = sum(d * d for d in derivative).sqrt()
            assert speed > 0
            expected = [point[0] - distance * derivative[1] / speed,
                        point[1] + distance * derivative[0] / speed]
            actual, _ = evaluation(curves[chunk], parameter)
            error = sum((a - b) ** 2 for a, b in zip(actual, expected)).sqrt()
            assert error <= number(cell['errorUpperMm']), (parameter, error, cell)
            maximum = max(maximum, error)
            samples += 1
    return {'oracle': 'Decimal(70) Cox-de Boor and rational first derivative',
            'samples': samples, 'curves': len(curves), 'cells': len(report['cells']),
            'maxSampledErrorMm': float(maximum), 'errorUpperMm': report['errorUpperMm'],
            'continuousCertificateFromIndependentOracle': False,
            'regionTopologyCertified': False}


if __name__ == '__main__':
    request, response = [json.load(open(p, encoding='utf8')) for p in sys.argv[1:3]]
    with localcontext() as context:
        context.prec = 70
        print(json.dumps(verify(request, response), indent=2))
