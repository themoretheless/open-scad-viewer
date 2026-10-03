// @rush/1
// Tangents are dP/dt on [2,7]; the weighted guide has normalized derivatives 80 and 20.
a = bezier_curve(points: [[0mm,0mm,0mm],[20mm,0mm,0mm]],weights: [1,2])
b = bezier_curve(points: [[0mm,0mm,40mm],[20mm,0mm,40mm]],weights: [3,1])
g = bezier_curve(points: [[0mm,0mm,0mm],[0mm,0mm,40mm]],weights: [1,2])
show guided_loft_surface(a,b,parameters: [2,7],guides: [g],guide_parameters: [0],construction: "cartesian",error_budget: 0.000001mm,max_cells: 50000,max_map_evaluations: 200000,start_tangents: [[0mm,0mm,16mm],[0mm,0mm,24mm]],end_tangents: [[0mm,0mm,4mm],[0mm,0mm,8mm]]).tessellate(segments_u: 24,segments_v: 32)
