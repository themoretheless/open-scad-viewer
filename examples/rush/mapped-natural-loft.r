// @rush/1
// One identity map and one two-piece rational map retain the original sections.
a = bezier_curve(points: [[0mm,0mm,0mm],[20mm,0mm,0mm]],weights: [1,2])
b = bezier_curve(points: [[0mm,0mm,40mm],[20mm,0mm,40mm]],weights: [3,1])
identity = loft_parameter_map()
left = loft_parameter_map(domain: [0,0.5],range: [0,0.4],control_values: [0,0.2,0.4],weights: [1,0.75,1])
right = loft_parameter_map(domain: [0.5,1],range: [0.4,1],control_values: [0.4,0.7,1],weights: [1,1.25,1])
m = loft_parameter_map(pieces: [left,right])
show natural_loft_surface(a,b,parameters: [2,7],section_mappings: [identity,m]).tessellate(segments_u: 24,segments_v: 32)
