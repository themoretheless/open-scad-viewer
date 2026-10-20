// @rush/1
// Original planar cubic inflection; original principal-phase affine/center transport, filled caps and hole.
a = line_curve(start: [0.03125mm,-0.03125mm,-0.03125mm],end: [-0.03125mm,0.03125mm,-0.03125mm])
b = line_curve(start: [-0.03125mm,0.03125mm,-0.03125mm],end: [-0.03125mm,0.03125mm,0.03125mm])
c = line_curve(start: [-0.03125mm,0.03125mm,0.03125mm],end: [0.03125mm,-0.03125mm,0.03125mm])
d = line_curve(start: [0.03125mm,-0.03125mm,0.03125mm],end: [0.03125mm,-0.03125mm,-0.03125mm])
h0 = line_curve(start: [0.015625mm,-0.015625mm,-0.015625mm],end: [0.015625mm,-0.015625mm,0.015625mm])
h1 = line_curve(start: [0.015625mm,-0.015625mm,0.015625mm],end: [-0.015625mm,0.015625mm,0.015625mm])
h2 = line_curve(start: [-0.015625mm,0.015625mm,0.015625mm],end: [-0.015625mm,0.015625mm,-0.015625mm])
h3 = line_curve(start: [-0.015625mm,0.015625mm,-0.015625mm],end: [0.015625mm,-0.015625mm,-0.015625mm])
path = bezier_curve(points: [[0,0,0],[1mm,1mm,0],[2mm,-1mm,0],[3mm,0,0]])
show brep_progressive_sweep([[a,b,c,d],[h0,h1,h2,h3]],path,
 cap_correction_tolerance: 0.000000001mm,cap_correction_quantum: 0.0000000000009094947017729282mm,cap_correction_max_work: 1000000,
 orientation: "corrected_frenet",normal: [0,0,1],spacing: "parameter",
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 axis_scale: {degree: 1,knots: [0,0,1,1],values: [[1.25,0.75,1],[1.25,0.75,1]],weights: [1,1]},
 center_law: {degree: 1,knots: [0,0,1,1],values: [[0.002mm,-0.003mm,0],[0.002mm,-0.003mm,0]],weights: [1,1]},
 initial_sections: 3,max_sections: 65,length_tolerance: 0.001mm,length_max_cells: 100000,max_deviation: 0.1mm
).brep_tessellate(4)
