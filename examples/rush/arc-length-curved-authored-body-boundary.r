// @rush/1
// Original curved arc-length boundary; native Solid audit is separate.
a = line_curve(start: [0,0,0],end: [2mm,0,0])
b = line_curve(start: [2mm,0,0],end: [2mm,2mm,0])
c = line_curve(start: [2mm,2mm,0],end: [0,2mm,0])
d = line_curve(start: [0,2mm,0],end: [0,0,0])
h0 = line_curve(start: [0.5mm,0.5mm,0],end: [0.5mm,1.5mm,0])
h1 = line_curve(start: [0.5mm,1.5mm,0],end: [1.5mm,1.5mm,0])
h2 = line_curve(start: [1.5mm,1.5mm,0],end: [1.5mm,0.5mm,0])
h3 = line_curve(start: [1.5mm,0.5mm,0],end: [0.5mm,0.5mm,0])
path = bezier_curve(points: [[0,0,0],[0,0,0.5mm],[0,1mm,1mm]])
show brep_progressive_sweep([[a,b,c,d],[h0,h1,h2,h3]],path,
 orientation: "authored",normal: [1,0,0],spacing: "arc_length",
 frame_axis: {degree: 1,knots: [0,0,1,1],values: [[0,0,1],[0,0,1]],weights: [1,1]},
 frame_normal: {degree: 1,knots: [0,0,1,1],values: [[1,0,0],[1,0,0]],weights: [1,1]},
 axis_scale: {degree: 1,knots: [0,0,1,1],values: [[2,3,1],[2,3,1]],weights: [1,1]},
 center_law: {degree: 1,knots: [0,0,1,1],values: [[0,0,0],[0,0,0]],weights: [1,1]},
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 initial_sections: 3,max_sections: 9,length_tolerance: 0.001mm,length_max_cells: 100000,max_deviation: 0.2mm
).brep_tessellate(4)
