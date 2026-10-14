// @rush/1
profile = lissajous_curve(center: [0mm,0mm,0mm],amplitudes: [5mm,3mm,2mm],frequencies: [0.125,0.25,0.375],phases_degrees: [0deg,30deg,60deg],max_deviation: 0.0001mm)
show profile.surface_extrude([0mm,0mm,1mm]).tessellate(segments_u: 32,segments_v: 8)
