import {importStepForWorkbench, exportRetainedStepForWorkbench} from './cadStepRouting'
import {loadProjectStepModel, saveProjectStepModel} from './cadStepIndexedDb'
import {parseDirectDocument, type DirectDocument} from './directModeling'
import {warmGeometryKernel} from './geometry/kernel'

/** Validate the complete addition before replacing the independently retained original. */
export async function prepareSolidStepImport(text: string, current: DirectDocument) {
  const snapshot = structuredClone(current)
  await warmGeometryKernel()
  const imported = importStepForWorkbench(text)
  const document = parseDirectDocument(JSON.stringify({
    ...snapshot,
    bodies: [...snapshot.bodies, ...imported.bodies],
    interchange: {...snapshot.interchange, step: imported.report},
  }))
  if (imported.retainedModel) {
    await saveProjectStepModel(imported.retainedModel, imported.report, imported.retainedDocument)
  }
  return {document, report: imported.report, selected: imported.bodies[0]?.id ?? ''}
}

/** Export the saved original, never imply that later Solid edits update its assembly graph. */
export async function exportSolidStepOriginal() {
  await warmGeometryKernel()
  const stored = await loadProjectStepModel()
  if (!stored) throw Error('No saved AP242 original. Import a retained STEP model first.')
  return exportRetainedStepForWorkbench(stored.model, stored.document).text
}
