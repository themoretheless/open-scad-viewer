import copy
import runpy
import unittest
from pathlib import Path
compare=runpy.run_path(str(Path(__file__).with_name('compare-expression-wall-pending.py')))['compare']

class PendingCoverage(unittest.TestCase):
    def fixture(self, faces=(28,28)):
        return dict(method='expression-support-aligned-chord-coverage',minimum_mm=1.,max_sine_squared=.0001,
                    tolerance_uv=.05,plane_controls=4,total_pairs=1,
                    pending=[dict(faces=list(faces),domains=[[[0.,.5],[0.,1.]],[[.5,1.],[0.,1.]]])])
    def test_subdivision_and_same_face_exchange(self):
        before=self.fixture();after=copy.deepcopy(before)
        after['pending'][0]['domains']=[[[.6,.7],[.2,.3]],[[.1,.2],[.4,.5]]]
        self.assertTrue(compare(before,after)['pendingContainedInPrevious'])
    def test_different_faces_cannot_exchange(self):
        before=self.fixture((2,3));after=copy.deepcopy(before)
        after['pending'][0]['domains'].reverse()
        with self.assertRaises(AssertionError):compare(before,after)
    def test_expanded_region_rejected(self):
        before=self.fixture();after=copy.deepcopy(before)
        after['pending'][0]['domains'][0][0]=[-.1,.5]
        with self.assertRaises(AssertionError):compare(before,after)
    def test_request_change_rejected(self):
        before=self.fixture();after=copy.deepcopy(before);after['minimum_mm']=2.
        with self.assertRaises(AssertionError):compare(before,after)

if __name__=='__main__':unittest.main()
