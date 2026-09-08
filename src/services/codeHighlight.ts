/** Lexical highlighting only: never evaluates source or inserts unescaped markup. */
export function highlightCode(source: string, selectedName = ''): string {
  const escape = (s: string) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;').replace(/'/g, '&#39;');
  const tokens = /\/\*[\s\S]*?(?:\*\/|$)|\/\/[^\n]*|"(?:\\[\s\S]|[^"\\])*(?:"|$)|\b(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?(?:mm|cm|in|deg|rad|m)?\b|\$?[A-Za-z_][\w$]*|\.\.<|\.\.|[+*/%=!<>?:&|^-]+/g;
  const keywords = new Set(['fn', 'ret', 'struct', 'int', 'f32', 'f64', 'str', 'Geometry', 'in', 'by', 'count', 'where', 'validate', 'param', 'range', 'show', 'module', 'function', 'let', 'each', 'for', 'if', 'else', 'assert', 'echo', 'include', 'use', 'true', 'false', 'undef']);
  let result = '', end = 0;
  for (const match of source.matchAll(tokens)) {
    const value = match[0], index = match.index!;
    result += escape(source.slice(end, index));
    const rest = source.slice(index + value.length);
    const kind = value.startsWith('//') || value.startsWith('/*') ? 'comment'
      : value.startsWith('"') ? (/^\s*:/.test(rest) ? 'property' : 'string')
      : /^\d|^\.\d/.test(value) ? 'number'
      : keywords.has(value) ? 'keyword'
      : /^[A-Za-z_$]/.test(value) ? (/^\s*\(/.test(rest) ? 'function' : /^\s*=(?!=)/.test(rest) ? 'property' : '')
      : 'operator';
    const occurrence = value === selectedName && /^\$?[A-Za-z_][\w$]*$/.test(value) && kind !== 'comment' && kind !== 'string'
    const classes = [kind ? `syntax-${kind}` : '', occurrence ? 'syntax-occurrence' : ''].filter(Boolean).join(' ')
    result += classes ? `<span class="${classes}">${escape(value)}</span>` : escape(value);
    end = index + value.length;
  }
  return result + escape(source.slice(end)) + '\n';
}
