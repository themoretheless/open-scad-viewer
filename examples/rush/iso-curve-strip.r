// @rush/1
patch = bezier_surface(points:[[[0mm,0mm,0mm],[0mm,8mm,0mm]],[[10mm,0mm,0mm],[10mm,8mm,4mm]]])
section = patch.iso_curve(direction:"u",parameter:0.25)
show section.surface_extrude(vector:[0mm,0mm,2mm]).tessellate(4,4)
