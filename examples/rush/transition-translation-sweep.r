// @rush/1
// Fixed-orientation translation along a quintic corner transition.
profile = bezier_curve(points: [[0,0,0],[0,0,1mm]])
path = transition_polyline_curve(points: [[0,0,0],[10mm,0,2mm],[10mm,10mm,5mm]], setback: 2mm)
show surface_sweep(profile,path).tessellate(segments_u: 4, segments_v: 32)
