"""Exact endpoint containment of unresolved parameter products, not CAD admission."""
import argparse
import hashlib
import json
from collections import defaultdict
from fractions import Fraction
from pathlib import Path


def compare(before, after):
    for field in ('method', 'minimum_mm', 'max_sine_squared', 'tolerance_uv', 'plane_controls', 'total_pairs'):
        assert before[field] == after[field], f'Changed request: {field}'
    def group(report):
        result = defaultdict(list)
        for region in report['pending']:
            box = tuple(tuple(interval) for domain in region['domains'] for interval in domain)
            assert len(box) == 4 and all(len(d) == 2 and d[0] < d[1] for d in box)
            assert all(Fraction.from_float(x).denominator for d in box for x in d)
            result[tuple(region['faces'])].append(box)
        return result
    old, new = group(before), group(after)
    changes = []
    for pair in sorted(old.keys() | new.keys()):
        if old[pair] == new[pair]:
            continue
        assert old[pair], f'New unresolved face pair: {pair}'
        boxes = old[pair]
        if pair[0] == pair[1]:
            # Same immutable face: exchanging endpoints preserves the query.
            boxes = boxes + [box[2:] + box[:2] for box in boxes]
        lo, hi = min(b[0][0] for b in boxes), max(b[0][1] for b in boxes)
        def bucket(x):
            return max(0, min(63, int((x-lo)/(hi-lo)*64)))
        bins = [[] for _ in range(64)]
        for box in boxes:
            for index in range(bucket(box[0][0]), bucket(box[0][1])+1):
                bins[index].append(box)
        for box in new[pair]:
            assert any(all(a[0] <= b[0] and b[1] <= a[1] for a,b in zip(candidate,box))
                       for candidate in bins[bucket(box[0][0])]), f'Uncontained pending product: {pair} {box}'
        def volume(boxes):
            total = Fraction(0)
            for box in boxes:
                product = Fraction(1)
                for a,b in box:
                    product *= Fraction.from_float(b)-Fraction.from_float(a)
                total += product
            return {'numerator': total.numerator, 'denominator': total.denominator, 'value': float(total)}
        changes.append({'faces':pair,'beforeRegions':len(old[pair]),'afterRegions':len(new[pair]),
                        'beforeRawParameterVolume':volume(old[pair]),'afterRawParameterVolume':volume(new[pair])})
    return {'passed':True,'geometryQualified':False,'pendingContainedInPrevious':True,'changes':changes,
            'scope':'Exact endpoint containment, allowing same-face endpoint exchange. Raw parameter volume sums are not physical volume or thickness.'}


if __name__ == '__main__':
    parser=argparse.ArgumentParser()
    parser.add_argument('before',type=Path);parser.add_argument('after',type=Path);parser.add_argument('output',type=Path)
    parser.add_argument('--before-request',type=Path)
    parser.add_argument('--after-request',type=Path)
    args=parser.parse_args()
    assert bool(args.before_request) == bool(args.after_request), 'Provide both request sidecars'
    requests=None
    if args.before_request:
        old_request=json.loads(args.before_request.read_text())
        new_request=json.loads(args.after_request.read_text())
        for field in ('test','cells','spans','trimCells'):
            assert field in old_request and field in new_request, f'Missing request field: {field}'
            assert old_request[field] == new_request[field], f'Changed work request: {field}'
        requests={}
        for path,request,report in ((args.before_request,old_request,args.before),(args.after_request,new_request,args.after)):
            assert request['report'] == report.name, 'Request does not identify compared report'
            manifest=path.parent/request['sourceManifest']
            requests[str(path)]={'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),
                'sourceManifest':str(manifest),'sourceManifestSha256':hashlib.sha256(manifest.read_bytes()).hexdigest()}
    result=compare(json.loads(args.before.read_text()),json.loads(args.after.read_text()))
    if requests is not None:
        result['matchedRequestSidecars']=requests
    result['sha256']={str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in (args.before,args.after)}
    args.output.write_text(json.dumps(result,indent=2)+'\n')
