// @rush/1
// Automatic orientation reverses the guide before certifying original dP/dt fields.
a = line_curve(start: [0mm,0mm,0mm],end: [20mm,0mm,0mm])
b = line_curve(start: [0mm,0mm,40mm],end: [20mm,0mm,40mm])
g = bezier_curve(points: [[10mm,0mm,40mm],[10mm,0mm,0mm]],weights: [3,1])
show auto_guided_loft_surface(a,b,parameters: [2,7],guides: [g],budget: 0.000001mm,construction: "cartesian",max_cells: 50000,max_map_evaluations: 200000,start_tangents: [[0mm,0mm,24mm],[0mm,0mm,24mm]],end_tangents: [[0mm,0mm,2.6666666666666665mm],[0mm,0mm,2.6666666666666665mm]]).tessellate(segments_u: 24,segments_v: 32)
