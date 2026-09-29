import assert from 'node:assert/strict'
import {readFile,writeFile} from 'node:fs/promises'
import {resolve,dirname} from 'node:path'
import {createHash} from 'node:crypto'
import {svgContours,contoursSvg,contoursExtrusion,meshSvgContours} from '../src/services/svgGeometry'
import {HeadlessGeometryService} from '../src/mcp/geometryService'
const file=resolve(process.argv[2]??'/tmp/solid-svg-qualification/browser-edited.svg')
const source=await readFile(file,'utf8'),geometry=new HeadlessGeometryService()
async function measure(svg:string){return geometry.analyze(contoursExtrusion(await svgContours(svg),2),'full')}
const downloaded=await measure(source)
assert.ok(Math.abs(downloaded.volume!-168)<1e-5)
assert.deepEqual(downloaded.bounds,{min:[0,0,0],max:[14,6,2]})
const perforated='<svg xmlns="http://www.w3.org/2000/svg" width="20mm" height="20mm" viewBox="0 0 20 20"><path fill-rule="evenodd" d="M0 0H20V20H0Z M5 5H15V15H5Z"/></svg>'
const original=await measure(perforated)
const built=await geometry.compile(contoursExtrusion(await svgContours(perforated),2),'full')
const projected=contoursSvg(await meshSvgContours(built.meshes,{axis:'z'}))
const roundtrip=await measure(projected)
assert.ok(Math.abs(original.volume!-600)<1e-5)
assert.ok(Math.abs(roundtrip.volume!-600)<1e-5)
assert.deepEqual(roundtrip.bounds,original.bounds)
const report={input:file,sha256:createHash('sha256').update(source).digest('hex'),downloaded:{volumeMm3:downloaded.volume,bounds:downloaded.bounds},holeRoundtrip:{volumeMm3:roundtrip.volume,bounds:roundtrip.bounds},scope:'Native SVG reimport and extrusion; downloaded Solid XY projection and perforated profile'}
await writeFile(resolve(dirname(file),'roundtrip.json'),JSON.stringify(report,null,2)+'\n')
console.log(JSON.stringify(report,null,2))
