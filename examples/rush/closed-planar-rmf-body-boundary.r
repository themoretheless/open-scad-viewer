// @rush/1
// Complete closed RMF boundary; Solid separately requires all material audits.
// Native hull groups cover all 256 walls within the default audit budgets.
a = line_curve(start: [4.9mm,0,-0.1mm],end: [5.1mm,0,-0.1mm])
b = line_curve(start: [5.1mm,0,-0.1mm],end: [5.1mm,0,0.1mm])
c = line_curve(start: [5.1mm,0,0.1mm],end: [4.9mm,0,0.1mm])
d = line_curve(start: [4.9mm,0,0.1mm],end: [4.9mm,0,-0.1mm])
path = circle_curve(center: [0,0,0],normal: [0,0,1],radius: 5mm)
show brep_progressive_sweep([[a,b,c,d]],path,
 orientation: "rmf",normal: [0,0,1],
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,360deg],weights: [1,1]},
 initial_sections: 5,max_sections: 129,max_deviation: 0.01mm
).brep_tessellate(4)
