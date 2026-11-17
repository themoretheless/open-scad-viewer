import { beforeAll, describe, expect, it } from 'vitest'
import { createRenderer, h, nextTick, ref } from 'svet'
import ProjectFilesPanel from '../src/components/ProjectFilesPanel.svt'
import { useProjectFiles } from '../src/composables/useProjectFiles'
import { createWorkspaceDocument, type WorkspaceDocumentSnapshot } from '../src/app/workspaceDocument'
import { BuildCoordinator, geometryProjectSourcesEqual, type WorkerLike } from '../src/app/buildCoordinator'
import { GeometryBuildEngine } from '../src/kernel/geometryBuildEngine'
import type { GeometryBuildRequest, GeometryWorkerRequest } from '../src/kernel/geometryWorkerProtocol'
import { presentBuildError } from '../src/i18n/kernelErrors'
import { translate, type Language } from '../src/i18n/appStrings'

/** Minimal host element: props keep handlers and attributes; text keeps rendered text. */
class El {
  parent: El | null = null
  children: El[] = []
  props: Record<string, any> = {} // eslint-disable-line @typescript-eslint/no-explicit-any -- host props hold handlers and attributes
  text = ''
  constructor(public tag: string) {}
  get textContent(): string {
    return this.text + this.children.map(child => child.textContent).join('')
  }
}
const renderer = createRenderer<El, El>({
  createElement: tag => new El(tag),
  createText: text => Object.assign(new El('#text'), { text }),
  createComment: () => new El('#comment'),
  setText: (node, text) => {
    node.text = text
  },
  setElementText: (node, text) => {
    node.children = []
    node.text = text
  },
  parentNode: node => node.parent,
  nextSibling: node => node.parent?.children[node.parent.children.indexOf(node) + 1] ?? null,
  patchProp: (node, key, _old, value) => {
    node.props[key] = value
  },
  insert: (node, parent, anchor) => {
    node.parent = parent
    const index = anchor ? parent.children.indexOf(anchor) : -1
    if (index < 0) parent.children.push(node)
    else parent.children.splice(index, 0, node)
  },
  remove: node => {
    if (node.parent) node.parent.children = node.parent.children.filter(child => child !== node)
  },
  setScopeId: () => {},
  insertStaticContent: () => {
    throw new Error('Unexpected static content')
  },
})

function find(root: El, testId: string): El | undefined {
  if (root.props['data-testid'] === testId) return root
  for (const child of root.children) {
    const hit = find(child, testId)
    if (hit) return hit
  }
  return undefined
}
function get(root: El, testId: string): El {
  const hit = find(root, testId)
  if (!hit) throw new Error(`missing ${testId}`)
  return hit
}
function typeInto(element: El, value: string) {
  element.props.onInput({ target: { value } })
}
function click(element: El) {
  element.props.onClick({})
}

class RecordingWorker implements WorkerLike {
  readonly messages: GeometryWorkerRequest[] = []
  postMessage(message: GeometryWorkerRequest) {
    this.messages.push(message)
  }
  terminate() {}
  addEventListener() {}
  removeEventListener() {}
  get builds(): GeometryBuildRequest[] {
    return this.messages.filter((message): message is GeometryBuildRequest => message.type === 'build')
  }
}

/**
 * Mounts the project panel with the App wiring: project edits persist, advance
 * the build generation and request a build that carries `project`.
 */
function mountProjectApp(document: WorkspaceDocumentSnapshot, lang: Language = 'en') {
  const workspaceDocument = ref(document)
  const worker = new RecordingWorker()
  const coordinator = new BuildCoordinator({ workerFactory: () => worker, onPublish: () => {} })
  let revision = 0
  let persisted = 0
  const projectFiles = useProjectFiles({
    workspaceDocument,
    persist: () => persisted++,
    onProjectChanged: () => build(),
  })
  function build() {
    revision++
    coordinator.requestBuild({
      documentRevision: revision,
      source: workspaceDocument.value.source,
      quality: 'preview',
      project: projectFiles.project.value,
    })
  }
  const root = new El('root')
  renderer
    .createApp({
      render: () =>
        h(ProjectFilesPanel, {
          locale: lang,
          mainFileName: workspaceDocument.value.fileName,
          files: projectFiles.files.value,
          activePath: projectFiles.activePath.value,
          activeSource: projectFiles.activeSource.value,
          libraryPaths: projectFiles.libraryPaths.value,
          error: projectFiles.error.value,
          onSelect: (path: string | null) => (projectFiles.activePath.value = path),
          onAdd: projectFiles.addFile,
          onRename: projectFiles.renameFile,
          onRemove: projectFiles.removeFile,
          'onUpdate-source': projectFiles.updateActiveSource,
          'onLibrary-paths': projectFiles.setLibraryPaths,
        }),
    })
    .mount(root)
  return { root, worker, coordinator, workspaceDocument, projectFiles, build, persisted: () => persisted }
}

const engine = new GeometryBuildEngine()
beforeAll(() => engine.initializeSource('cube(1);', { quality: 'preview', purpose: 'preview' }), 120000)

function runBuild(request: GeometryBuildRequest) {
  return engine.buildSource(request.source, { quality: 'preview', purpose: 'preview', project: request.project })
}

describe('project files panel', () => {
  it('builds a project with includes and rebuilds after an edit to an included file', async () => {
    const app = mountProjectApp(
      createWorkspaceDocument('include <parts/body.scad>\nuse <bolt.scad>\nbody();\nbolt();', {
        fileName: 'main.scad',
      }),
    )
    await nextTick()
    expect(get(app.root, 'project-tab-main').textContent).toBe('main.scad')

    typeInto(get(app.root, 'project-new-path'), 'parts/body.scad')
    click(get(app.root, 'project-add'))
    await nextTick()
    expect(app.projectFiles.activePath.value).toBe('parts/body.scad')
    typeInto(get(app.root, 'project-source'), 'module body() cube([4, 4, 1]);')

    typeInto(get(app.root, 'project-new-path'), 'libraries/bolt.scad')
    click(get(app.root, 'project-add'))
    await nextTick()
    typeInto(get(app.root, 'project-source'), 'module bolt() translate([10, 0, 0]) cylinder(h = 3, r = 1);')
    get(app.root, 'project-library-paths').props.onChange({ target: { value: 'libraries/\n' } })
    await nextTick()

    expect(app.workspaceDocument.value.libraryPaths).toEqual(['libraries'])
    expect(app.persisted()).toBeGreaterThan(0)
    const first = app.worker.builds.at(-1)!
    expect(first.project).toEqual({
      entrypoint: 'main.scad',
      files: [
        { path: 'parts/body.scad', source: 'module body() cube([4, 4, 1]);' },
        { path: 'libraries/bolt.scad', source: 'module bolt() translate([10, 0, 0]) cylinder(h = 3, r = 1);' },
      ],
      libraryPaths: ['libraries'],
    })
    const before = (await runBuild(first)).result
    expect(before.meshes.length).toBeGreaterThan(0)

    click(get(app.root, 'project-tab-parts/body.scad'))
    await nextTick()
    const buildsBeforeEdit = app.worker.builds.length
    typeInto(get(app.root, 'project-source'), 'module body() cube([4, 4, 5]);')
    await nextTick()
    expect(app.worker.builds.length).toBe(buildsBeforeEdit + 1)
    const rebuilt = app.worker.builds.at(-1)!
    expect(rebuilt.documentRevision).toBeGreaterThan(first.documentRevision)
    const after = (await runBuild(rebuilt)).result
    expect(after.volume).toBeGreaterThan(before.volume + 60)
  }, 120000)

  it('renames and deletes auxiliary files and rejects invalid or duplicate paths', async () => {
    const app = mountProjectApp(createWorkspaceDocument('include <a.scad>', { fileName: 'main.scad' }), 'ru')
    typeInto(get(app.root, 'project-new-path'), 'a.scad')
    click(get(app.root, 'project-add'))
    await nextTick()
    typeInto(get(app.root, 'project-new-path'), '../escape.scad')
    click(get(app.root, 'project-add'))
    await nextTick()
    expect(get(app.root, 'project-error').textContent).toBe(translate('ru', 'projectFileInvalidPath'))
    typeInto(get(app.root, 'project-new-path'), 'main.scad')
    click(get(app.root, 'project-add'))
    await nextTick()
    expect(get(app.root, 'project-error').textContent).toBe(translate('ru', 'projectFileExists'))

    typeInto(get(app.root, 'project-rename-path'), 'lib/b.scad')
    click(get(app.root, 'project-rename'))
    await nextTick()
    expect(app.workspaceDocument.value.projectFiles.map(file => file.path)).toEqual(['lib/b.scad'])
    expect(app.projectFiles.activePath.value).toBe('lib/b.scad')
    expect(find(app.root, 'project-error')).toBeUndefined()

    click(get(app.root, 'project-delete'))
    await nextTick()
    expect(app.workspaceDocument.value.projectFiles).toEqual([])
    expect(app.projectFiles.activePath.value).toBeNull()
    expect(find(app.root, 'project-source')).toBeUndefined()
  })

  it('reports a dependency cycle with the localized error', async () => {
    const app = mountProjectApp(createWorkspaceDocument('include <a.scad>', { fileName: 'main.scad' }), 'ru')
    typeInto(get(app.root, 'project-new-path'), 'a.scad')
    click(get(app.root, 'project-add'))
    await nextTick()
    typeInto(get(app.root, 'project-source'), 'include <main.scad>')
    const request = app.worker.builds.at(-1)!
    const failure = await runBuild(request).then(
      () => null,
      (caught: { code?: string; message: string }) => caught,
    )
    expect(failure?.code).toBe('E_PROJECT_DEPENDENCY_CYCLE')
    const t = (key: string) => translate('ru', key)
    expect(presentBuildError(failure!, 'ru', t)).toContain(t('languageError.dependencyCycle').slice(0, 12))
    expect(presentBuildError(failure!, 'ru', t)).not.toBe(failure!.message)
  }, 120000)

  it('reports a missing included file with the localized error', async () => {
    const app = mountProjectApp(createWorkspaceDocument('include <gone.scad>', { fileName: 'main.scad' }), 'en')
    get(app.root, 'project-library-paths').props.onChange({ target: { value: 'lib' } })
    const request = app.worker.builds.at(-1)!
    expect(request.project?.libraryPaths).toEqual(['lib'])
    const failure = await runBuild(request).then(
      () => null,
      (caught: { code?: string; message: string }) => caught,
    )
    expect(failure?.code).toBe('E_PROJECT_DEPENDENCY_MISSING')
    const t = (key: string) => translate('en', key)
    expect(presentBuildError(failure!, 'en', t)).toContain(t('languageError.dependencyMissing').slice(0, 12))
  }, 120000)
})

describe('build coordinator project dedupe', () => {
  it('dedupes equal project content and rebuilds when an included file changes', () => {
    const worker = new RecordingWorker()
    const coordinator = new BuildCoordinator({ workerFactory: () => worker, onPublish: () => {} })
    const project = () => ({
      entrypoint: 'main.scad',
      files: [{ path: 'a.scad', source: 'cube(1);' }],
      libraryPaths: [],
    })
    const first = coordinator.requestBuild({
      documentRevision: 1,
      source: 'include <a.scad>',
      quality: 'preview',
      project: project(),
    })
    const again = coordinator.requestBuild({
      documentRevision: 1,
      source: 'include <a.scad>',
      quality: 'preview',
      project: project(),
    })
    expect(again).toBe(first)
    const changed = coordinator.requestBuild({
      documentRevision: 1,
      source: 'include <a.scad>',
      quality: 'preview',
      project: { ...project(), files: [{ path: 'a.scad', source: 'cube(2);' }] },
    })
    expect(changed).not.toBe(first)
    expect(geometryProjectSourcesEqual(project(), { ...project(), libraryPaths: ['lib'] })).toBe(false)
    expect(geometryProjectSourcesEqual(undefined, project())).toBe(false)
  })
})
