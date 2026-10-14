// @rush/1
// Positions and dP/dt tangents are authored at t=[-2,1,5]; output u is normalized.
profile = hermite_curve(points: [[-12mm,0mm,0mm],[0mm,8mm,0mm],[15mm,0mm,0mm]],tangents: [[4mm,0mm,0mm],[3mm,0mm,0mm],[2mm,-2mm,0mm]],parameters: [-2,1,5])
show profile.surface_extrude([0mm,0mm,10mm]).tessellate(segments_u: 16,segments_v: 32)
