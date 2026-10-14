// @rush/1
// G2 certificate covers internal parts of this profile; caps and profile closure are separate.
profile = polyline_curve(points: [[0mm,0mm,0mm],[0.03125mm,0mm,0mm],[0.0625mm,0mm,0mm],[0.09375mm,0mm,0mm],[0.125mm,0mm,0mm],[0.15625mm,0mm,0mm],[0.1875mm,0mm,0mm],[0.21875mm,0mm,0mm],[0.25mm,0mm,0mm],[0.28125mm,0mm,0mm],[0.3125mm,0mm,0mm],[0.34375mm,0mm,0mm],[0.375mm,0mm,0mm],[0.40625mm,0mm,0mm],[0.4375mm,0mm,0mm],[0.46875mm,0mm,0mm],[0.5mm,0mm,0mm],[0.53125mm,0mm,0mm],[0.5625mm,0mm,0mm],[0.59375mm,0mm,0mm],[0.625mm,0mm,0mm],[0.65625mm,0mm,0mm],[0.6875mm,0mm,0mm],[0.71875mm,0mm,0mm],[0.75mm,0mm,0mm],[0.78125mm,0mm,0mm],[0.8125mm,0mm,0mm],[0.84375mm,0mm,0mm],[0.875mm,0mm,0mm],[0.90625mm,0mm,0mm],[0.9375mm,0mm,0mm],[0.96875mm,0mm,0mm],[1mm,0mm,0mm]])
path = line_curve(start: [0mm,0mm,0mm],end: [0mm,0mm,4mm])
show progressive_sweep(profile,path,
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 normal: [1,0,0],orientation: "fixed",initial_sections: 2,max_sections: 2,max_deviation: 0.01mm
).nurbs_patches_tessellate(segments: 2)
