import { beforeAll, describe, expect, it } from 'vitest'
import { createWorkspaceDocument } from '../src/app/workspaceDocument'
import { GeometryBuildEngine, type GeometryProjectSources } from '../src/kernel/geometryBuildEngine'
import { GEOMETRY_WORKER_PROTOCOL_VERSION } from '../src/kernel/geometryWorkerProtocol'
import { isGeometryWorkerRequest } from '../src/kernel/geometryWorkerProtocol/isGeometryEvaluationResultPayload'
import { sha256Hex } from '../src/core/sha256'

const engine = new GeometryBuildEngine()
const request = { quality: 'preview', purpose: 'preview' } as const
beforeAll(() => engine.initializeSource('cube(1);', request), 120000)

function projectOf(document: ReturnType<typeof createWorkspaceDocument>): GeometryProjectSources {
  return { entrypoint: document.fileName, files: document.projectFiles, libraryPaths: document.libraryPaths }
}

describe('building a workspace project with include and use', () => {
  it('builds geometry from relative includes and library uses', async () => {
    const document = createWorkspaceDocument('include <parts/body.scad>\nuse <bolt.scad>\nbody();\nbolt();', {
      fileName: 'main.scad',
      projectFiles: [
        { path: 'parts/body.scad', source: 'module body() cube([4, 4, 1]);' },
        { path: 'libraries/bolt.scad', source: 'module bolt() translate([10, 0, 0]) cylinder(h = 3, r = 1);' },
      ],
      libraryPaths: ['libraries'],
    })
    const { result } = await engine.buildSource(document.source, { ...request, project: projectOf(document) })
    expect(result.quality).toBe('preview')
    expect(result.meshes.length).toBeGreaterThan(0)
    expect(result.volume).toBeGreaterThan(16)
  }, 120000)

  it('fails a cyclic project with the registered cycle code', async () => {
    const document = createWorkspaceDocument('include <a.scad>', {
      fileName: 'main.scad',
      projectFiles: [{ path: 'a.scad', source: 'include <main.scad>' }],
    })
    await expect(
      engine.buildSource(document.source, { ...request, project: projectOf(document) }),
    ).rejects.toMatchObject({ code: 'E_PROJECT_DEPENDENCY_CYCLE' })
  }, 120000)

  it('fails a missing include with the registered missing code', async () => {
    const document = createWorkspaceDocument('include <gone.scad>', { fileName: 'main.scad' })
    await expect(
      engine.buildSource(document.source, { ...request, project: projectOf(document) }),
    ).rejects.toMatchObject({ code: 'E_PROJECT_DEPENDENCY_MISSING' })
  }, 120000)

  it('admits project context in worker build requests and rejects malformed project shapes', () => {
    const source = 'include <a.scad>'
    const message = {
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      type: 'build',
      documentRevision: 1,
      jobId: 1,
      source,
      sourceSha256: sha256Hex(source),
      quality: 'preview',
      project: { entrypoint: 'main.scad', files: [{ path: 'a.scad', source: 'cube(1);' }], libraryPaths: ['lib'] },
    }
    expect(isGeometryWorkerRequest(message)).toBe(true)
    expect(isGeometryWorkerRequest({ ...message, project: { entrypoint: 'main.scad', files: [{ path: 1 }] } })).toBe(
      false,
    )
    expect(isGeometryWorkerRequest({ ...message, project: { entrypoint: 'main.scad', files: [], extra: 1 } })).toBe(
      false,
    )
  })
})
