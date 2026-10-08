// @rush/1
a = line_curve(start: [0mm,0mm,0mm],end: [20mm,0mm,0mm])
b = line_curve(start: [0mm,0mm,40mm],end: [20mm,0mm,40mm])
g = bezier_curve(points: [[10mm,0mm,0mm],[10mm,20mm,20mm],[10mm,0mm,40mm]])
show guided_loft_surface(a,b,parameters: [0,1],guides: [g],guide_parameters: [0.5],start_tangents: [[0mm,40mm,40mm],[0mm,40mm,40mm]],end_tangents: [[0mm,-40mm,40mm],[0mm,-40mm,40mm]]).tessellate(segments_u: 24,segments_v: 32)
