import { expect, it } from 'vitest'
import { sourceFileExtension, withSourceExtension } from '../src/services/modelGraphTextDetect'

it('names ModelGraph Text documents .mg and OpenSCAD documents .scad', () => {
  expect(sourceFileExtension('// @modelgraph-text/1\nshow box(1mm,1mm,1mm)')).toBe('.mg')
  expect(sourceFileExtension('cube(10);')).toBe('.scad')
  expect(withSourceExtension('spinner', '// @modelgraph-text/1\n')).toBe('spinner.mg')
  expect(withSourceExtension('spinner.scad', '// @modelgraph-text/1\n')).toBe('spinner.scad')
  expect(withSourceExtension('part.MG', 'cube(1);')).toBe('part.MG')
  expect(withSourceExtension('part', 'cube(1);')).toBe('part.scad')
})
