import importlib.util
import math
import unittest
from fractions import Fraction
from pathlib import Path

spec = importlib.util.spec_from_file_location('reference', Path(__file__).with_name('reference-sweep-generator-volume.py'))
reference = importlib.util.module_from_spec(spec)
spec.loader.exec_module(reference)


def patches(radius=1, reverse=False, expanding=False):
    signs = [[(1, 0), (1, 1), (0, 1)], [(0, 1), (-1, 1), (-1, 0)],
             [(-1, 0), (-1, -1), (0, -1)], [(0, -1), (1, -1), (1, 0)]]
    return [{'degreeU': 2, 'degreeV': 1, 'knotsU': [0, 0, 0, 1, 1, 1],
             'knotsV': [0, 0, 1, 1], 'weights': [[w, w] for w in [1, math.sqrt(.5), 1]],
             'controlPoints': [[[radius*x, radius*y*(-1 if reverse else 1)*(1+v if expanding else 1), 10*v]
                                for v in range(2)] for x, y in quarter]} for quarter in signs]


class VolumeReference(unittest.TestCase):
    def test_cylinder_and_oriented_hole(self):
        outer = reference.section_integral(patches())
        hole = reference.section_integral(patches(.5, reverse=True))
        self.assertEqual(outer, 10)
        self.assertEqual(hole, Fraction(-5, 2))
        self.assertEqual(outer+hole, Fraction(15, 2))

    def test_varying_transverse_axis_integrates_jacobian(self):
        self.assertEqual(reference.section_integral(patches(expanding=True)), 15)

    def test_conic_area_and_weighted_polynomial_generator(self):
        self.assertAlmostEqual(reference.conic_area(1), 10/3, places=14)
        self.assertAlmostEqual(reference.conic_area(math.sqrt(.5)), math.pi, places=14)
        self.assertAlmostEqual(reference.conic_area(.5), 4/3+8*math.pi/(9*math.sqrt(3)), places=14)
        self.assertAlmostEqual(reference.conic_area(2), 14/3-4*math.acosh(2)/(3*math.sqrt(3)), places=14)
        self.assertAlmostEqual(reference.conic_area(1+1e-12), 10/3, places=11)
        for weight in [.5, 1, 2]:
            candidate = patches()
            for surface in candidate:
                surface['weights'][1] = [weight, weight]
            self.assertEqual(reference.section_integral(candidate, weight), 10)
        for weight in [0, -1, float('inf'), float('nan')]:
            with self.assertRaises(AssertionError):
                reference.conic_area(weight)

    def test_noncanonical_control_and_weight_refuse(self):
        for field in ['controlPoints', 'weights']:
            candidate = patches()
            if field == 'controlPoints':
                candidate[0][field][1][0][0] += .125
            else:
                candidate[0][field][1][0] += .125
            with self.assertRaises(AssertionError):
                reference.section_integral(candidate)


if __name__ == '__main__':
    unittest.main()
