#!/usr/bin/env python3
"""Independent OCCT interference check; numerical STEP evidence, not native exact proof."""
import argparse
import hashlib
import io
import json
from pathlib import Path
import OCP
from OCP.BOPAlgo import BOPAlgo_CheckerSI
from OCP.IFSelect import IFSelect_RetDone
from OCP.STEPControl import STEPControl_Reader


def inspect_shape(shape):
    checker = BOPAlgo_CheckerSI()
    checker.AddArgument(shape)
    checker.SetLevelOfCheck(5)  # V/V through F/F, including nonadjacent faces
    checker.Perform()
    errors, warnings = io.BytesIO(), io.BytesIO()
    checker.DumpErrors(errors)
    checker.DumpWarnings(warnings)
    # DS is safe to inspect only after Perform, and never after an error.
    if checker.HasErrors():
        return dict(completed=False, noInterferencesFound=False,
                    errors=errors.getvalue().decode(errors='replace'),
                    warnings=warnings.getvalue().decode(errors='replace'))
    ds = checker.DS()
    count = ds.Interferences().Extent()
    return dict(completed=True, interferencePairs=count,
                noInterferencesFound=count == 0 and not checker.HasWarnings(),
                errors=errors.getvalue().decode(errors='replace'),
                warnings=warnings.getvalue().decode(errors='replace'),
                # FF records can include normal adjacent-boundary bookkeeping;
                # the interference map, not the raw FF vector, defines defects.
                faceFaceRecords=ds.InterfFF().Length())


def self_test():
    from OCP.BRep import BRep_Builder
    from OCP.BRepPrimAPI import BRepPrimAPI_MakeBox
    from OCP.TopoDS import TopoDS_Compound
    from OCP.gp import gp_Pnt
    cube = BRepPrimAPI_MakeBox(10., 10., 10.).Shape()
    assert inspect_shape(cube)['noInterferencesFound']
    builder, compound = BRep_Builder(), TopoDS_Compound()
    builder.MakeCompound(compound)
    builder.Add(compound, cube)
    builder.Add(compound, BRepPrimAPI_MakeBox(gp_Pnt(5., 5., 5.), 10., 10., 10.).Shape())
    crossing = inspect_shape(compound)
    assert crossing['completed'] and crossing['interferencePairs'] > 0, crossing
    print('control cube accepted; overlapping boxes detected')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('path', nargs='?', type=Path)
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    if args.self_test:
        self_test()
    if args.path is None:
        if not args.self_test:
            parser.error('STEP path required')
        return 0
    reader = STEPControl_Reader()
    if reader.ReadFile(str(args.path)) != IFSelect_RetDone or reader.TransferRoots() != 1:
        raise ValueError('Expected one transferred STEP root')
    report = inspect_shape(reader.OneShape())
    report.update(occtVersion=OCP.__version__, levelOfCheck=5,
                  inputSha256=hashlib.sha256(args.path.read_bytes()).hexdigest(),
                  nativeExactAbsenceProven=False,
                  scope='Numerical OCCT STEP V/V, V/E, E/E, V/F, E/F and F/F interference search; independent evidence, not exact native embedding or material proof.')
    args.path.with_suffix('.self-intersections.occt.json').write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(report, indent=2))
    return 0 if report['noInterferencesFound'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
