// @rush/1
// Original curved planar RMF arc-length boundary; native Solid audit is separate.
a = line_curve(start: [0,0,0],end: [0.1mm,0,0])
b = line_curve(start: [0.1mm,0,0],end: [0.1mm,0.1mm,0])
c = line_curve(start: [0.1mm,0.1mm,0],end: [0,0.1mm,0])
d = line_curve(start: [0,0.1mm,0],end: [0,0,0])
h0 = line_curve(start: [0.025mm,0.025mm,0],end: [0.025mm,0.075mm,0])
h1 = line_curve(start: [0.025mm,0.075mm,0],end: [0.075mm,0.075mm,0])
h2 = line_curve(start: [0.075mm,0.075mm,0],end: [0.075mm,0.025mm,0])
h3 = line_curve(start: [0.075mm,0.025mm,0],end: [0.025mm,0.025mm,0])
path = bezier_curve(points: [[0,0,0],[0,0,0.5mm],[0,1mm,1mm]])
show brep_progressive_sweep([[a,b,c,d],[h0,h1,h2,h3]],path,
 cap_correction_tolerance: 0.000000001mm,cap_correction_quantum: 0.0000000000009094947017729282mm,cap_correction_max_work: 1000000,
 orientation: "rmf",normal: [1,0,0],spacing: "arc_length",
 axis_scale: {degree: 1,knots: [0,0,1,1],values: [[2,3,1],[2,3,1]],weights: [1,1]},
 center_law: {degree: 1,knots: [0,0,1,1],values: [[0,0,0],[0,0,0]],weights: [1,1]},
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 initial_sections: 3,max_sections: 17,length_tolerance: 0.001mm,length_max_cells: 100000,max_deviation: 2mm
).brep_tessellate(4)
