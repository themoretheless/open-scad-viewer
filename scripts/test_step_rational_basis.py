import math
import unittest
from step_rational_basis import same_positive_projective_weights as same


class ProjectiveWeights(unittest.TestCase):
    def test_exact_common_scale_and_occt_polynomial_normalization(self):
        for a, b in [([.75] * 6, [1.] * 6), ([.75, .5, .75], [3., 2., 3.])]:
            self.assertTrue(same(a, b))
            self.assertTrue(same(b, a))

    def test_unequal_or_rounded_ratios_and_invalid_weights_refuse(self):
        for a, b in [([.75, .5], [1., 1.]), ([1., 2.], [1., math.nextafter(2., 3.)]),
                     ([1.], [0.]), ([1.], [-1.]), ([1.], [math.inf]),
                     ([1.], [math.nan]), ([], []), ([1.], [1., 1.])]:
            self.assertFalse(same(a, b))


if __name__ == '__main__':
    unittest.main()
