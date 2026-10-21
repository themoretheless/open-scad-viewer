import copy,importlib.util,json,unittest
from pathlib import Path
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('companion_contracts',HERE/'check_companion27_contracts.py');checker=importlib.util.module_from_spec(spec);spec.loader.exec_module(checker)
STAGE=Path('/private/tmp/osv-sweep-common-source-v11-20261018')
class CompanionContracts(unittest.TestCase):
 @classmethod
 def setUpClass(cls):
  data=json.loads((HERE/'companion27-historical-checker-fixture-20261018.json').read_text());assert data['geometryWasmSha256'].startswith('b376');assert 'not current qualification' in data['scope']
  cls.cases=data['cases'];cls.contracts=checker.expected_contracts(STAGE)
 def source(self,q):
  c=self.contracts[q['file']];return (STAGE/c['source'] if c['spatial'] else STAGE/'qualification-law-sources'/c['source']).read_bytes()
 def check(self,q,text=None):
  original=self.source(q);return checker.validate_case(q,self.contracts[q['file']],original if text is None else text,original)
 def test_all27_historical_native_rows_satisfy_the_contract(self):
  checker.check_inventory(self.cases,self.contracts)
  self.assertEqual(len([self.check(q) for q in self.cases]),27)
 def test_raised_deviation_budget_refused_for_every_companion(self):
  for source in self.cases:
   with self.subTest(file=source['file']):
    q=copy.deepcopy(source);b=q['boundaryError'] if 'boundaryError' in q else q['owner'];b['budget']*=2
    with self.assertRaisesRegex(ValueError,'budget changed'):self.check(q)
 def test_station_G1_promotion_refused_for_every_companion(self):
  for source in self.cases:
   with self.subTest(file=source['file']):
    q=copy.deepcopy(source);q['nativeProfileSmoothness']['station']['stationG1Certified']=True
    with self.assertRaisesRegex(ValueError,'station was promoted'):self.check(q)
 def test_missing_G2_and_whole_boundary_promotion_refused(self):
  for source in self.cases:
   for which in ['G2','scope']:
    with self.subTest(file=source['file'],which=which):
     q=copy.deepcopy(source)
     if which=='G2':q['nativeProfileSmoothness']['profile']['exactG1G2Certified']=False
     else:q['nativeProfileSmoothness']['fullBoundarySmoothnessCertified']=True
     with self.assertRaises(ValueError):self.check(q)
 def test_missing_or_outside_correction_contribution_refused(self):
  for source in self.cases:
   if 'boundaryError' not in source:continue
   for error in [None,0,-1,2e-9,True]:
    with self.subTest(file=source['file'],error=error):
     q=copy.deepcopy(source);q['owner']['profileCorrectionErrorUpper']=error
     with self.assertRaisesRegex(ValueError,'correction contribution'):self.check(q)
 def test_missing_caps_and_closed_fabrications_refused(self):
  for source in self.cases:
   if 'boundaryError' in source:continue
   with self.subTest(file=source['file']):
    q=copy.deepcopy(source);q['owner']['filledCapErrorUpper']=[] if q['owner']['closed'] else None
    with self.assertRaisesRegex(ValueError,'caps|two-cap'):self.check(q)
 def test_joint_law_flags_and_numeric_booleans_refused(self):
  for source in self.cases:
   if 'boundaryError' in source:continue
   for which in ['native','numeric']:
    with self.subTest(file=source['file'],which=which):
     q=copy.deepcopy(source)
     if which=='native':q['constructionFlags']['affineLawsApplied']=not q['constructionFlags']['affineLawsApplied']
     else:q['lawMatrixMode']['affine']=int(q['lawMatrixMode']['affine'])
     with self.assertRaises(ValueError):self.check(q)
 def test_source_change_missing_duplicate_or_extra_inventory_refused(self):
  q=self.cases[0]
  with self.assertRaisesRegex(ValueError,'source differs'):self.check(q,self.source(q)+b'// changed source')
  for cases in [self.cases[:-1],self.cases+[q],self.cases[:-1]+[q]]:
   with self.assertRaisesRegex(ValueError,'inventory'):checker.check_inventory(cases,self.contracts)
if __name__=='__main__':unittest.main(verbosity=2)
