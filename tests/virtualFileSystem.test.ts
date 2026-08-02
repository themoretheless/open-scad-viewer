import { describe, expect, it } from 'vitest'
import {
  MAX_PROJECT_FILES,
  VirtualFileSystem,
  normalizeVirtualPath,
  resolveVirtualPath,
} from '../src/services/virtualFileSystem'

describe('virtual project filesystem', () => {
  it('resolves deterministic project-relative dependencies without host access', () => {
    const vfs = new VirtualFileSystem([
      { path: 'main.scad', source: 'use <lib/part.scad>' },
      { path: 'lib/part.scad', source: 'module part() {}' },
    ])
    expect(vfs.resolve('main.scad', './lib/part.scad')).toEqual({ path: 'lib/part.scad', source: 'module part() {}' })
    expect(vfs.resolve('lib/part.scad', '../main.scad')?.path).toBe('main.scad')
    expect(vfs.list()).toEqual(['lib/part.scad', 'main.scad'])
  })

  it('rejects traversal, absolute/host paths, duplicates and file-count exhaustion', () => {
    expect(() => resolveVirtualPath('main.scad', '../secret')).toThrow('escapes')
    expect(() => normalizeVirtualPath('/etc/passwd')).toThrow('project-relative')
    expect(() => normalizeVirtualPath('C:\\secret.scad')).toThrow('project-relative')
    expect(() => normalizeVirtualPath('https://example.com/a.scad')).toThrow('project-relative')
    expect(() => normalizeVirtualPath('a//b.scad')).toThrow('project-relative')
    expect(() => new VirtualFileSystem([{ path: 'a.scad', source: '' }, { path: './a.scad', source: '' }])).toThrow('Duplicate')
    expect(() => new VirtualFileSystem([{ path: 'café.scad', source: '' }, { path: 'café.scad', source: '' }])).toThrow('Duplicate')
    expect(() => new VirtualFileSystem(Array.from({ length: MAX_PROJECT_FILES + 1 }, (_, index) => ({ path: `${index}.scad`, source: '' })))).toThrow('exceeds')
  })
})
