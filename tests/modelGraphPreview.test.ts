import { expect, it } from 'vitest'
import { inflateSync } from 'node:zlib'
import { HeadlessGeometryService } from '../src/mcp/geometryService'
import { renderModelGraphPreviews } from '../src/mcp/modelGraphPreview'
it('renders real transformed geometry as three valid nonempty PNG views', async () => {
  const built = await new HeadlessGeometryService().compile('translate([100,20,3]) difference(){cube([20,10,5]);translate([10,5,-1])cylinder(r=2,h=7,$fn=32);}')
  const previews = renderModelGraphPreviews(built.meshes)
  expect(previews.map(p => p.view)).toEqual(['front', 'top', 'isometric'])
  for (const { png } of previews) {
    expect(png.subarray(0, 8)).toEqual(Buffer.from([137,80,78,71,13,10,26,10]))
    expect(png.readUInt32BE(16)).toBe(192)
    const length = png.readUInt32BE(33)
    const rows = inflateSync(png.subarray(41, 41 + length))
    expect(rows.length).toBe(192 * 577)
    expect(rows.some(value => value !== 245 && value !== 0)).toBe(true)
  }
  expect(previews[0]!.png.equals(previews[1]!.png)).toBe(false)
})
