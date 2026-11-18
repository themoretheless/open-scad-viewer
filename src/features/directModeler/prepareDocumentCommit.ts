import type { DirectDocument } from '../../app/directModeling'
import { sameMeshJson } from '../../exchange/sameMeshJson'
/** `commitInWorker` of the modeler: `build` runs after the previous queued commit, the worker validates,
 * a stale result is dropped, `after` runs only once the commit landed, errors go to `onError` or `error`. */
export type CommitInWorker = (
  build: () => DirectDocument | undefined,
  after?: (next: DirectDocument) => void,
  onError?: (message: string) => void,
) => Promise<boolean>
const documentObjects = (d: DirectDocument) => [...d.bodies, ...d.sketches, ...(d.curves ?? []), ...(d.surfaces ?? [])]
export function prepareDocumentCommit(
  next: DirectDocument,
  previous: DirectDocument | undefined,
  fallbackIds: Iterable<string>,
  lockedIds: string[],
  activeGroup: string,
  origin: 'edit' | 'file',
  label: (ru: string, en: string) => string,
  resolved = false,
) {
  const existing = new Set(previous ? documentObjects(previous).map(b => b.id) : fallbackIds)
  const sourceChanges = new Map<string, boolean>()
  const previousBodies = new Map(previous?.bodies.map(body => [body.id, body]) ?? [])
  const nextBodies = new Map(next.bodies.map(body => [body.id, body]))
  if (origin === 'edit')
    for (const source of previous?.bodies ?? [])
      if (!nextBodies.has(source.id)) {
        const linked = next.bodies.filter(body => body.instance?.sourceId === source.id)
        if (linked.length)
          throw Error(
            label('Нельзя удалить источник «', 'Cannot delete source “') +
              source.name +
              label('»: остаются связанные экземпляры — ', '”: linked instances remain — ') +
              linked.length +
              '. ' +
              label(
                'Сделайте их независимыми или удалите вместе с источником.',
                'Make them independent or delete them together with the source.',
              ),
          )
      }
  // Imported caches are checked and rebuilt by the authoritative history parser.
  // The edit-only guard detects attempts to modify a live linked body directly.
  // `resolved`: the candidate already came out of that parser (a worker result), which derives
  // every instance from its source, so a linked body cannot differ from its source there.
  if (!resolved)
    for (const body of previous?.bodies ?? [])
      if (origin === 'edit' && body.instance) {
        const candidate = nextBodies.get(body.id)
        // The same object as before (edits built from the published document share it): unchanged.
        if (candidate === body) continue
        const sourceId = body.instance.sourceId
        if (!sourceChanges.has(sourceId)) {
          const oldSource = previousBodies.get(sourceId),
            newSource = nextBodies.get(sourceId)
          sourceChanges.set(
            sourceId,
            oldSource !== newSource &&
              !sameMeshJson(
                { mesh: oldSource?.mesh, brep: oldSource?.brep },
                { mesh: newSource?.mesh, brep: newSource?.brep },
              ),
          )
        }
        if (
          !sourceChanges.get(sourceId) &&
          candidate?.instance &&
          sameMeshJson(candidate.instance, body.instance) &&
          !sameMeshJson({ mesh: candidate.mesh, brep: candidate.brep }, { mesh: body.mesh, brep: body.brep })
        )
          throw Error(
            label('Измените источник или отсоедините экземпляр: ', 'Edit the source or detach the instance: ') +
              body.name,
          )
      }
  const lockedObjects = (previous ? documentObjects(previous) : []).filter(object => lockedIds.includes(object.id))
  const validateLocked = lockedObjects.length
    ? (resolved: DirectDocument) => {
        const nextObjects = new Map(documentObjects(resolved).map(o => [o.id, o]))
        for (const object of lockedObjects) {
          const replacement = nextObjects.get(object.id)
          if (!replacement || !sameMeshJson(object, replacement))
            throw Error(label('Объект заблокирован: ', 'Object is locked: ') + object.name)
        }
      }
    : undefined
  for (const body of documentObjects(next))
    if (!existing.has(body.id) && (origin === 'file' ? body.group === undefined : !body.group) && activeGroup)
      body.group = activeGroup
  const added = documentObjects(next)
    .filter(b => !existing.has(b.id))
    .map(b => b.id)
  return { validateLocked, added }
}
