// @rush/1
// Nested rational section maps, unequal weights and a curved endpoint guide.
a = bezier_curve(points: [[0mm,0mm,0mm],[20mm,0mm,0mm]],weights: [1,2])
b = bezier_curve(points: [[0mm,0mm,40mm],[20mm,0mm,40mm]],weights: [3,1])
g = bezier_curve(points: [[0mm,0mm,0mm],[0mm,8mm,20mm],[0mm,0mm,40mm]],weights: [1,2,1])
m = loft_parameter_map(domain: [0,1],range: [0,1],control_values: [0,0.2,1],weights: [1,0.75,1])
nested = loft_parameter_map(composition: [m,m])
show guided_loft_surface(a,b,parameters: [2,7],guides: [g],guide_parameters: [0],section_mappings: [nested,nested],construction: "cartesian",error_budget: 0.000001mm,max_cells: 50000,max_map_evaluations: 200000).tessellate(segments_u: 24,segments_v: 32)
