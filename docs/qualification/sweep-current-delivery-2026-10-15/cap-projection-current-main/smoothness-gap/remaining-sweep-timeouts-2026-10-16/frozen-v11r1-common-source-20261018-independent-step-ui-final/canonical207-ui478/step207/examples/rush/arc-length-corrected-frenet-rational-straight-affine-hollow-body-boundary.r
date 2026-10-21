// @rush/1
// Complete boundary error for this straight source; global embedding is separate.
a = line_curve(start: [0,0,0],end: [2mm,0,0])
b = line_curve(start: [2mm,0,0],end: [2mm,2mm,0])
c = line_curve(start: [2mm,2mm,0],end: [0,2mm,0])
d = line_curve(start: [0,2mm,0],end: [0,0,0])
h0 = line_curve(start: [0.5mm,0.5mm,0],end: [0.5mm,1.5mm,0])
h1 = line_curve(start: [0.5mm,1.5mm,0],end: [1.5mm,1.5mm,0])
h2 = line_curve(start: [1.5mm,1.5mm,0],end: [1.5mm,0.5mm,0])
h3 = line_curve(start: [1.5mm,0.5mm,0],end: [0.5mm,0.5mm,0])
path = nurbs_curve(degree: 3,knots: [2,2,2,2,5,5,5,5],control_points: [[0,0,0],[0,0,1mm],[0,0,7mm],[0,0,10mm]],weights: [1,2,2,1])
show brep_progressive_sweep([[a,b,c,d],[h0,h1,h2,h3]],path,
 orientation: "corrected_frenet",normal: [1,0,1],
 axis_scale: {degree: 1,knots: [0,0,1,1],values: [[2,3,1],[2,3,1]],weights: [1,1]},
 center_law: {degree: 1,knots: [0,0,1,1],values: [[0.125mm,-0.25mm,0],[0.125mm,-0.25mm,0]],weights: [1,1]},
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 spacing: "arc_length",length_tolerance: 0.001mm,length_max_cells: 100000,initial_sections: 3,max_sections: 129,max_deviation: 0.01mm
).brep_tessellate(4)
