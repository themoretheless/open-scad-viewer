import type { DirectGroup } from './directModeling'

export function validateDirectGroupHierarchy(groups: readonly DirectGroup[]): void {
  const parents = new Map(groups.map(group => [group.name, group.parent]))
  for (const group of groups) {
    const seen = new Set([group.name])
    let parent = group.parent
    while (parent !== undefined) {
      if (typeof parent !== 'string' || !parents.has(parent) || seen.has(parent))
        throw Error('Invalid group hierarchy: missing parent or cycle.')
      seen.add(parent)
      parent = parents.get(parent)
    }
  }
}

export function editDirectGroupParent(groups: readonly DirectGroup[], name: string, parent: string): DirectGroup[] {
  if (!groups.some(group => group.name === name)) throw Error('Select a saved group.')
  const next = groups.map(group => {
    if (group.name !== name) return group
    const { parent: _, ...rest } = group
    return parent ? { ...rest, parent } : rest
  })
  validateDirectGroupHierarchy(next)
  return next
}

export function directGroupBranch(groups: readonly DirectGroup[], name: string): Set<string> {
  const branch = new Set([name])
  for (let previous = 0; previous !== branch.size;) {
    previous = branch.size
    for (const group of groups) {
      if (group.parent && branch.has(group.parent)) branch.add(group.name)
    }
  }
  return branch
}

export function validateDirectGroups(groups: DirectGroup[] | undefined): void {
  // Absent rather than empty, so a document with no groups stays identical to one
  // written before groups existed.
  if (groups !== undefined && (!Array.isArray(groups) || groups.length > 64))
    throw new Error('Invalid group collection.')
  const groupNames = new Set<string>()
  for (const g of groups ?? []) {
    if (!g || typeof g.name !== 'string' || g.name.length === 0 || g.name.length > 100 || groupNames.has(g.name))
      throw new Error('Invalid group name.')
    if (typeof g.source !== 'string' || g.source.length > 100_000) throw new Error('Invalid group source.')
    groupNames.add(g.name)
  }
  validateDirectGroupHierarchy(groups ?? [])
}

export function orderDirectGroups(names: Iterable<string>, groups?: readonly DirectGroup[]) {
  const ordered: {name: string; depth: number}[] = []
  const children = new Map<string | undefined, string[]>()
  const parents = new Map(groups?.map(group => [group.name, group.parent]))
  for (const name of names) {
    const parent = parents.get(name)
    const siblings = children.get(parent) ?? []
    siblings.push(name)
    children.set(parent, siblings)
  }
  function visit(parent: string | undefined, depth: number) {
    for (const name of children.get(parent) ?? []) {
      ordered.push({name, depth})
      visit(name, depth + 1)
    }
  }
  visit(undefined, 0)
  return ordered
}
