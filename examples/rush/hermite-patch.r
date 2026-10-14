// @rush/1
panel = hermite_patch(corners: [[[0mm,0mm,0mm],[0mm,20mm,0mm]],[[30mm,0mm,0mm],[30mm,20mm,8mm]]],tangent_u: [[[30mm,0mm,0mm],[30mm,0mm,0mm]],[[30mm,0mm,0mm],[30mm,0mm,12mm]]],tangent_v: [[[0mm,20mm,0mm],[0mm,20mm,0mm]],[[0mm,20mm,0mm],[0mm,20mm,12mm]]],twist: [[[0mm,0mm,0mm],[0mm,0mm,0mm]],[[0mm,0mm,0mm],[0mm,0mm,18mm]]])
show panel.tessellate(segments_u: 24,segments_v: 24)
