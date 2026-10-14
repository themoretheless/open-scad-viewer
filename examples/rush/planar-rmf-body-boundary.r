// @rush/1
// Original open planar RMF; native body boundary and Solid are separate audits.
a = line_curve(start: [0,-0.1mm,-0.1mm],end: [0,0.1mm,-0.1mm])
b = line_curve(start: [0,0.1mm,-0.1mm],end: [0,0.1mm,0.1mm])
c = line_curve(start: [0,0.1mm,0.1mm],end: [0,-0.1mm,0.1mm])
d = line_curve(start: [0,-0.1mm,0.1mm],end: [0,-0.1mm,-0.1mm])
path = bezier_curve(points: [[0,0,0],[0.5mm,0,0],[1mm,1mm,0]])
show brep_progressive_sweep([[a,b,c,d]],path,
 orientation: "rmf",normal: [0,0,1],
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 initial_sections: 3,max_sections: 129,max_deviation: 0.01mm
).brep_tessellate(4)
