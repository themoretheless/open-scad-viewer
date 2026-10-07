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
path = line_curve(start: [0,0,0],end: [0,0,10mm])
show brep_progressive_sweep([[a,b,c,d],[h0,h1,h2,h3]],path,
 orientation: "rmf",normal: [1,0,0],
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 initial_sections: 3,max_sections: 3,max_deviation: 0.01mm
).brep_tessellate(4)
