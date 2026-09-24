export interface EditorBlock { start: number; end: number; column: number; depth: number }

/**
 * Mask strings/comments while keeping columns and line breaks.
 * Single manual scan — equivalent to the previous
 * /\/\*[\s\S]*?(?:\*\/|$)|\/\/[^\n]*|"(?:\\[\s\S]|[^"\\])*(?:"|$)/g pass, but
 * without regex-callback allocations per match. Returns the source unchanged
 * when there is nothing to mask.
 */
function maskStringsAndComments(source: string): string {
  if (!/\/\/|\/\*|"/.test(source)) return source
  const chars = source.split(''), n = source.length
  let masked = false, i = 0
  while (i < n) {
    const char = source[i]
    if (char === '/' && source[i + 1] === '/') {
      while (i < n && source[i] !== '\n') { chars[i] = ' '; i++ }
      masked = true
    } else if (char === '/' && source[i + 1] === '*') {
      chars[i] = ' '; chars[i + 1] = ' '
      let j = i + 2
      while (j < n && !(source[j] === '*' && source[j + 1] === '/')) {
        if (source[j] !== '\n') chars[j] = ' '
        j++
      }
      if (j < n) { chars[j] = ' '; chars[j + 1] = ' '; j += 2 }
      masked = true; i = j
    } else if (char === '"') {
      // A string ending in a lone backslash at EOF does not match the regex
      // (the escape needs a second char), so leave such a quote untouched.
      let j = i + 1, terminated = false, valid = true
      while (j < n) {
        const inner = source[j]
        if (inner === '\\') {
          if (j + 1 >= n) { valid = false; break }
          j += 2
        } else if (inner === '"') { terminated = true; j++; break }
        else j++
      }
      if (!valid) { i++; continue }
      for (let k = i; k < j; k++) if (source[k] !== '\n') chars[k] = ' '
      masked = true; i = terminated ? j : n
    } else i++
  }
  return masked ? chars.join('') : source
}

// Identity cache: Vue may re-evaluate callers with an unchanged string.
let cachedSource: string | null = null
let cachedBlocks: EditorBlock[] = []

export function editorBlocks(source: string): EditorBlock[] {
  if (source === cachedSource) return cachedBlocks
  const lines = maskStringsAndComments(source).split('\n'), count = lines.length
  // Indents and emptiness once per line instead of repeated regex matches.
  const indents = new Array<number>(count), nonEmpty = new Array<boolean>(count)
  for (let i = 0; i < count; i++) {
    const line = lines[i]!
    let width = 0, k = 0
    for (; k < line.length; k++) {
      const code = line.charCodeAt(k)
      if (code === 32) width++
      else if (code === 9) width += 2
      else break
    }
    indents[i] = width
    nonEmpty[i] = /\S/.test(line)
  }
  const blocks: EditorBlock[] = [], stack: {line:number; column:number; char:number}[] = []
  for (let i = 0; i < count; i++) {
    const line = lines[i]!
    for (let col=0; col<line.length; col++) {
      const char=line.charCodeAt(col)
      if (char===40||char===91||char===123) stack.push({line:i,column:indents[i]!,char})
      else if (char===41||char===93||char===125) {
        const open=stack.at(-1)
        if (open && ((open.char===40&&char===41)||(open.char===91&&char===93)||(open.char===123&&char===125))) {
          stack.pop(); if(i>open.line) blocks.push({start:open.line,end:i,column:open.column,depth:0})
        }
      }
    }
  }
  const indentStack: {line:number;column:number}[]=[]
  let previous=-1
  for(let i=0;i<count;i++){
    if(!nonEmpty[i])continue
    const column=indents[i]!
    while(indentStack.length && column<=indentStack.at(-1)!.column){
      const open=indentStack.pop()!; blocks.push({start:open.line,end:previous,column:open.column,depth:0})
    }
    if(previous>=0 && column>indents[previous]!)indentStack.push({line:previous,column:indents[previous]!})
    previous=i
  }
  for(const open of indentStack)blocks.push({start:open.line,end:previous,column:open.column,depth:0})
  const unique = new Map<number,EditorBlock>()
  for(const b of blocks)if(!unique.has(b.start)||unique.get(b.start)!.end<b.end)unique.set(b.start,b)
  const sorted=[...unique.values()].sort((a,b)=>a.start-b.start||b.end-a.end)
  const parents:EditorBlock[]=[]
  for(const b of sorted){while(parents.length&&parents.at(-1)!.end<=b.start)parents.pop();b.depth=parents.length;parents.push(b)}
  cachedSource = source
  cachedBlocks = sorted
  return sorted
}
export function indentSelection(source:string,start:number,end:number,outdent=false) {
  if(start===end&&!outdent){const column=source.slice(source.lastIndexOf('\n',start-1)+1,start).replace(/\t/g,'  ').length;const text=' '.repeat(2-column%2);return {start,end,text,selectionStart:start+text.length,selectionEnd:start+text.length}}
  const from=source.lastIndexOf('\n',start-1)+1
  const last=end>start&&source[end-1]==='\n'?end-1:end
  const newline=source.indexOf('\n',last),to=newline<0?source.length:newline
  const lines=source.slice(from,to).split('\n');let delta=0,firstDelta=0
  const text=lines.map((line,i)=>{const remove=outdent?(line.startsWith('\t')?1:Math.min(2,line.match(/^ */)![0].length)):0;const d=outdent?-remove:2;delta+=d;if(!i)firstDelta=d;return outdent?line.slice(remove):'  '+line}).join('\n')
  return {start:from,end:to,text,selectionStart:Math.max(from,start+firstDelta),selectionEnd:Math.max(from,end+delta)}
}

/** Guides occupy whitespace only, never a closing delimiter or other code. */
export function guideFitsIndent(line: string, column: number): boolean {
  if (!line.trim()) return true
  let width = 0
  for (const char of line) {
    if (char === ' ') width++
    else if (char === '\t') width += 2 - width % 2
    else break
  }
  return column < width
}
