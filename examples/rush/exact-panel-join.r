// @rush/1
panel = bezier_surface(points:[[[0mm,0mm,0mm],[0mm,8mm,0mm]],[[10mm,0mm,0mm],[10mm,8mm,4mm]]])
left = panel.surface_split(direction:"v",parameter:0.25,part:0)
right = panel.surface_split(direction:"v",parameter:0.25,part:1)
joint = left.merge_surface_exact(target_geometry:right,direction:"v")
show joint.tessellate(8,8)
