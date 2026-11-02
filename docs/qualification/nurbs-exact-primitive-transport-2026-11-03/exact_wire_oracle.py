"""Independently verify actual JSON-returned geometry against the parsed request."""
import copy
import hashlib
import json
import math
import runpy
import sys
from pathlib import Path

base=Path('docs/qualification')
paths={
 'curve_exact_ellipse':base/'nurbs-exact-conics-2026-11-03/exact_oracle.py',
 'surface_exact_ellipsoid':base/'nurbs-exact-quadrics-2026-11-03/exact_oracle.py',
 'surface_exact_ring_torus':base/'nurbs-exact-tori-2026-11-03/exact_oracle.py',
 'surface_exact_frustum_side':base/'nurbs-exact-frusta-2026-11-03/exact_oracle.py',
 'surface_exact_planar_region':base/'nurbs-exact-planar-regions-2026-11-03/exact_oracle.py',
}
verifiers={op:runpy.run_path(str(path))['verify'] for op,path in paths.items()}
def verify(record):
    q,r=record['request'],record['response'];op=q['op']
    if op not in verifiers or r['status']!='constructed' or r['refusal'] is not None or r['topologyAuthority'] is not False:return False
    kind='curve' if op=='curve_exact_ellipse' else 'surface'
    if r['geometryKind']!=kind or r['scope']!='literal-'+kind+'-geometry' or not 0<r['workUsed']<=q['maxWork']<=1000000:return False
    g=r['geometry'];meta=r['metadata'];row={'center':q['center']}
    if kind=='curve':
        if g['periodic']:return False
        row.update(axisU=q['axisU'],axisV=q['axisV'],curve=g)
    else:
        if g['periodicU'] or g['periodicV']:return False
        row.update(axes=q['axes'],surface=g)
    if op=='surface_exact_ring_torus':row.update(major=q['majorRadius'],minor=q['minorRadius'])
    if op=='surface_exact_frustum_side':
        end=meta['collapsedEnd']
        if end not in [None,'start','end'] or meta['intrinsicApex']!=(end is not None):return False
        row.update(radii=q['radii'],collapsedEnd=None if end is None else 0 if end=='start' else 1)
    if op=='surface_exact_planar_region':
        if meta['regionKind'] not in ['filledDisk','annulus'] or meta['collapsedCenter']!=(meta['regionKind']=='filledDisk') or meta['regularPlaneSupport'] is not True:return False
        row.update(radii=q['radii'],kind=meta['regionKind'])
    return verifiers[op](row)
def rows(path):
    d=json.JSONDecoder()
    return sorted([d.raw_decode(line.split('exact-transport-oracle=',1)[1])[0] for line in Path(path).read_text().splitlines() if 'exact-transport-oracle=' in line],key=lambda r:json.dumps(r,sort_keys=True))
if __name__=='__main__':
    a,b=rows(sys.argv[1]),rows(sys.argv[2]);assert a==b and len(a)==11
    assert all(verify(r) for r in a)
    circle=next(r for r in a if r['request']['op']=='curve_exact_ellipse')
    wrong=copy.deepcopy(circle);wrong['response']['geometry']['controlPoints'][1][0]=math.nextafter(1.,math.inf);assert not verify(wrong)
    wrong=copy.deepcopy(circle);wrong['request']['axisU'][0]=2.;assert not verify(wrong)
    wrong=copy.deepcopy(circle);wrong['response']['geometryKind']='surface';assert not verify(wrong)
    wrong=copy.deepcopy(circle);wrong['response']['topologyAuthority']=True;assert not verify(wrong)
    wrong=copy.deepcopy(circle);wrong['response']['status']='refused';assert not verify(wrong)
    wrong=copy.deepcopy(circle);wrong['response']['workUsed']=1000001;assert not verify(wrong)
    plane=next(r for r in a if r['request']['op']=='surface_exact_planar_region' and r['request']['radii']==[0.,1.])
    wrong=copy.deepcopy(plane);wrong['response']['metadata']['regionKind']='annulus';assert not verify(wrong)
    cone=next(r for r in a if r['request']['op']=='surface_exact_frustum_side' and r['request']['radii']==[0.,2.])
    wrong=copy.deepcopy(cone);wrong['response']['metadata']['collapsedEnd']='end';assert not verify(wrong)
    print(json.dumps({'acceptedWireDefinitions':len(a),'operations':len({r['request']['op'] for r in a}),'negativeControls':8,
      'definitionsSha256':hashlib.sha256(json.dumps(a,sort_keys=True).encode()).hexdigest(),
      'oracleSources':{str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in paths.values()},
      'scope':'Actual serialized JSON geometry independently verified by exact Fraction full-domain primitive oracles against parsed input, plus source/metadata/authority/budget negative controls. No installed WASM, editor, STEP, B-rep or whole-app claim.'},indent=2))
