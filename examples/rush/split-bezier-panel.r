// @rush/1
panel = bezier_surface(points:[[[0mm,0mm,0mm],[0mm,8mm,0mm]],[[10mm,0mm,0mm],[10mm,8mm,4mm]]])
edited = panel.surface_edit(insert_knots_u:[{value:0.5,count:1}])
patch = edited.surface_bezier_patch(index:1).surface_transpose()
show patch.surface_split(direction:"u",parameter:0.25,part:1).tessellate(8,8)
