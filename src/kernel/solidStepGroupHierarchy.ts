import type { DirectGroup } from '../app/directModeling'

/** Reparent the identity-placed group occurrences emitted by the native STEP writer.
 * Bodies already carry their edited geometry and occurrence transforms in world axes.
 */
export function nestSharedStepGroups(text: string, names: readonly string[], groups: readonly DirectGroup[]): string {
  const nested = groups.filter(group => group.parent !== undefined)
  if (!nested.length) return text
  const entities = new Map(Array.from(text.matchAll(/^#(\d+)=(.*);$/gm), ([, id, value]) => [id!, value!]))
  const required = (id: string | undefined) => {
    const value = id && entities.get(id)
    if (!value) throw Error('Unexpected native STEP group hierarchy.')
    return value
  }
  const placements = new Map<string, {usage: string; definition: string; prefix: string}>()
  const shapes = new Map<string, string>()
  const representations = new Map<string, string>()
  const relations = new Map<string, string>()
  for (const [id, value] of entities) {
    const usage = /^(NEXT_ASSEMBLY_USAGE_OCCURRENCE\('group-(\d+)','(?:[^']|'')*','',)#\d+,(#\d+,\$\))$/.exec(value)
    if (usage) {
      const name = names[Number(usage[2])]
      if (name !== undefined) placements.set(name, {usage:id, definition:usage[3]!.slice(1,-3), prefix:usage[1]!})
    }
    const shape = /^PRODUCT_DEFINITION_SHAPE\('','',#(\d+)\)$/.exec(value)
    if (shape) shapes.set(shape[1]!,id)
    const representation = /^SHAPE_DEFINITION_REPRESENTATION\(#(\d+),#(\d+)\)$/.exec(value)
    if (representation) representations.set(representation[1]!,representation[2]!)
    const relation = /^CONTEXT_DEPENDENT_SHAPE_REPRESENTATION\(#(\d+),#(\d+)\)$/.exec(value)
    if (relation) relations.set(relation[2]!,relation[1]!)
  }
  const replacements = new Map<string, string>()
  for (const group of nested) {
    const child = placements.get(group.name)
    const parent = placements.get(group.parent!)
    if (!child || !parent) throw Error('Native STEP omitted a group occurrence.')
    const representation = representations.get(shapes.get(parent.definition) ?? '')
    const relation = relations.get(shapes.get(child.usage) ?? '')
    const row = required(relation)
    const updated = row.replace(/^(\(REPRESENTATION_RELATIONSHIP\('','',#\d+,)#\d+(\))/, `$1#${representation}$2`)
    if (!representation || updated === row) throw Error('Unexpected native STEP group relationship.')
    replacements.set(child.usage, `${child.prefix}#${parent.definition},#${child.definition},$)`)
    replacements.set(relation!, updated)
  }
  return text.replace(/^#(\d+)=(.*);$/gm, (row, id: string) =>
    replacements.has(id) ? `#${id}=${replacements.get(id)};` : row)
}
