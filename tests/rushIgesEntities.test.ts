import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { compileRushFrontend } from '../src/language/rushFrontend'
import { buildTextNurbsScene } from '../src/language/rushFrontendScene'

async function readEntity(fixture: string, kind: 'curve' | 'surface', fromEnd = 1) {
  const text = readFileSync('tests/fixtures/rush-iges/' + fixture + '.igs', 'utf8')
  const type = kind === 'curve' ? 126 : 128
  const entries = text.split(/\r?\n/).filter(line => line[72] === 'D')
    .filter((line, index) => index % 2 === 0 && Number(line.slice(0, 8)) === type)
  const entry = Number(entries.at(-fromEnd)!.slice(73))
  const show = kind === 'curve' ? 'entity.surface_extrude(vector:[0mm,0mm,1mm])' : 'entity'
  const source = '// @rush/1\nentity = open_iges_' + kind + '(iges_text:' + JSON.stringify(text) +
    ',directory_entry:' + entry + ')\nshow ' + show + '.tessellate(8,8)\n'
  const scene = await buildTextNurbsScene(compileRushFrontend(source).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements!.find(row => row.operation === 'open_iges_' + kind)!.report)
}

it('preserves periodic rational and open polynomial curve flags', async () => {
  expect(await readEntity('curve-flags', 'curve')).toMatchObject({periodic:true,domain:[0,3],weights:[1,0.5,1,1]})
  expect(await readEntity('curve-flags', 'curve', 2)).toMatchObject({periodic:false,weights:[1,1]})
}, 180000)

it('preserves independent surface periodicity', async () => {
  expect(await readEntity('surface-flags', 'surface')).toMatchObject({periodicU:false,periodicV:true})
  expect(await readEntity('surface-flags', 'surface', 2)).toMatchObject({periodicU:true,periodicV:false})
}, 180000)

it('uses declared rational curve and surface parameter intervals', async () => {
  expect(await readEntity('curve-interval', 'curve')).toMatchObject({domain:[0.25,1.5]})
  expect(await readEntity('surface-band', 'surface')).toMatchObject({domainU:[0,3],domainV:[0.25,0.75],periodicU:true})
  expect(await readEntity('surface-patch', 'surface')).toMatchObject({domainU:[0.5,2.25],domainV:[0.25,0.75],periodicU:false})
}, 180000)
