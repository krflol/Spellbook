import { bytes, chars, normalize, refs, Name, Reference } from './model';

export interface Suggestion {
  label: string;
  insert: string;
  start: number;
  end: number;
  detail: string;
  kind: 'command' | 'condition' | 'unit' | 'value' | 'spell' | 'item' | 'lua';
}
export interface Issue {
  start: number;
  end: number;
  severity: 'error' | 'warning';
  message: string;
  code: string;
  fix?: { start: number; end: number; text: string; title: string };
}
const plain = new Set([
  '/say',
  '/s',
  '/party',
  '/p',
  '/raid',
  '/ra',
  '/rw',
  '/raidwarning',
  '/yell',
  '/y',
  '/emote',
  '/e',
  '/me',
  '/whisper',
  '/w',
  '/tell',
  '/t',
  '/reply',
  '/r',
  '/guild',
  '/g',
  '/officer',
  '/o',
  '/console',
  '/reload',
  '/readycheck',
]);
const required = new Set([
  '/cast',
  '/use',
  '/castsequence',
  '/castrandom',
  '/userandom',
  '/equip',
  '/equipslot',
  '/cancelaura',
  '/run',
  '/script',
]);
export const command = (name: string): Reference | undefined =>
  refs.commands.find((r) => r.name === name.toLowerCase());
const conditional = (name: string): boolean =>
  !plain.has(name) && !['/run', '/script'].includes(name) && !!command(name);
const isLua = (name: string): boolean => name === '/run' || name === '/script';

/** Preserve UTF-16 offsets while masking Lua strings and comments. No evaluation. */
export function luaMask(text: string): {
  code: string;
  protected: Array<[number, number]>;
  unclosed: Array<[number, string]>;
} {
  const code = text.split(''),
    protectedRanges: Array<[number, number]> = [],
    unclosed: Array<[number, string]> = [];
  let i = 0;
  const hide = (start: number, end: number): void => {
    protectedRanges.push([start, end]);
    for (let j = start; j < end; j++) code[j] = ' ';
  };
  while (i < text.length) {
    const start = i;
    const comment = text.startsWith('--', i);
    const content = comment ? i + 2 : i;
    const long = /^\[(=*)\[/.exec(text.slice(content));
    if (long) {
      const close = ']' + long[1] + ']';
      const end = text.indexOf(close, content + long[0].length);
      i = end < 0 ? text.length : end + close.length;
      hide(start, i);
      if (end < 0) unclosed.push([start, comment ? 'comment' : 'string']);
      continue;
    }
    if (comment) {
      hide(i, text.length);
      break;
    }
    if (text[i] === '"' || text[i] === "'") {
      const quote = text[i++];
      let closed = false;
      while (i < text.length) {
        if (text[i] === '\\') {
          i += 2;
          continue;
        }
        if (text[i++] === quote) {
          closed = true;
          break;
        }
      }
      i = Math.min(i, text.length);
      hide(start, i);
      if (!closed) unclosed.push([start, 'string']);
      continue;
    }
    i++;
  }
  return { code: code.join(''), protected: protectedRanges, unclosed };
}
function suggest(
  list: Reference[],
  query: string,
  start: number,
  end: number,
  kind: Suggestion['kind'],
): Suggestion[] {
  return list
    .filter((r) => r.name.toLowerCase().startsWith(query.toLowerCase()))
    .map((r) => ({
      label: r.name,
      insert: r.name,
      start,
      end,
      detail: r.description + '\n' + r.example,
      kind,
    }));
}
export function complete(text: string, cursor: number, names: Name[]): Suggestion[] {
  cursor = Math.max(0, Math.min(cursor, text.length));
  const lineStart = text.lastIndexOf('\n', cursor - 1) + 1;
  const nl = text.indexOf('\n', cursor),
    lineEnd = nl < 0 ? text.length : nl;
  const line = text.slice(lineStart, lineEnd),
    prefix = text.slice(lineStart, cursor);
  const match = /^(\s*)(\S*)/.exec(line)!;
  const startCommand = lineStart + match[1].length,
    commandEnd = startCommand + match[2].length;
  if (cursor <= commandEnd)
    return suggest(
      refs.commands,
      text.slice(Math.min(startCommand, cursor), cursor),
      Math.min(startCommand, cursor),
      commandEnd,
      'command',
    );
  const cmd = match[2].toLowerCase();
  if (isLua(cmd)) {
    const code = prefix.slice(match[0].length),
      mask = luaMask(code);
    if (mask.protected.some(([start, end]) => code.length > start && code.length <= end)) return [];
    const word = /[\p{L}\p{N}_.]*$/u.exec(prefix)![0];
    const after = /^[\p{L}\p{N}_.]*/u.exec(text.slice(cursor, lineEnd))![0];
    return suggest(refs.lua, word, cursor - word.length, cursor + after.length, 'lua');
  }
  if (!conditional(cmd)) return [];
  const open = prefix.lastIndexOf('[');
  if (open >= 0 && !prefix.slice(open).includes(']')) {
    const begin = Math.max(open + 1, prefix.lastIndexOf(',') + 1);
    const raw = prefix.slice(begin),
      query = raw.trimStart().toLowerCase();
    const start = lineStart + begin + raw.length - raw.trimStart().length;
    const rest = text.slice(cursor, lineEnd),
      stop = rest.search(/[,\]]/),
      end = cursor + (stop < 0 ? rest.length : stop);
    const colon = query.indexOf(':');
    if (colon >= 0) {
      const key = query.slice(0, colon).replace(/^no/, ''),
        part = query
          .slice(colon + 1)
          .split('/')
          .pop()!;
      const options: Record<string, string[]> = {
        mod: ['shift', 'ctrl', 'alt'],
        modifier: ['shift', 'ctrl', 'alt'],
        btn: ['1', '2', '3', '4', '5'],
        button: ['1', '2', '3', '4', '5'],
        form: ['0', '1', '2', '3', '4', '5', '6'],
        stance: ['0', '1', '2', '3', '4', '5', '6'],
        spec: ['1', '2'],
        group: ['party', 'raid'],
        actionbar: ['1', '2', '3', '4', '5', '6'],
        bonusbar: ['0', '1', '2', '3', '4', '5'],
      };
      const slash = rest.slice(0, end - cursor).indexOf('/'),
        valueEnd = slash < 0 ? end : cursor + slash;
      return (options[key] ?? [])
        .filter((s) => s.startsWith(part))
        .map((s) => ({
          label: s,
          insert: s,
          start: cursor - part.length,
          end: valueEnd,
          detail: key + ':' + s,
          kind: 'value',
        }));
    }
    if (query.startsWith('target='))
      return refs.targets
        .filter((r) => r.name.slice(1).startsWith(query.slice(7)))
        .map((r) => ({
          label: 'target=' + r.name.slice(1),
          insert: 'target=' + r.name.slice(1),
          start,
          end,
          detail: r.description,
          kind: 'unit',
        }));
    const matches = [
      ...suggest(refs.conditions, query, start, end, 'condition'),
      ...suggest(refs.targets, query, start, end, 'unit'),
    ];
    if (query.startsWith('no'))
      for (const r of refs.conditions.filter((r) => !r.name.startsWith('no'))) {
        const negative = 'no' + r.name;
        if (negative.startsWith(query) && !matches.some((s) => s.label === negative))
          matches.push({
            label: negative,
            insert: negative,
            start,
            end,
            kind: 'condition',
            detail: 'Negate: ' + r.description,
          });
      }
    return matches;
  }
  const sequence = ['/castsequence', '/castrandom', '/userandom'].includes(cmd);
  let start = commandEnd;
  for (let i = commandEnd; i < cursor; i++)
    if (text[i] === ']' || text[i] === ';' || (sequence && text[i] === ',')) start = i + 1;
  while (start < cursor && /\s/.test(text[start])) start++;
  if (text.slice(start, cursor).startsWith('reset=')) {
    const space = text.slice(start, cursor).search(/\s/);
    if (space < 0)
      return ['reset=target ', 'reset=combat ', 'reset=target/10 ', 'reset=10 ']
        .filter((s) => s.startsWith(text.slice(start, cursor)))
        .map((s) => ({
          label: s.trim(),
          insert: s,
          start,
          end: cursor,
          detail: 'Reset on target/combat change or after this many idle seconds.',
          kind: 'value',
        }));
    start += space + 1;
  }
  if (cmd === '/equipslot') {
    const space = text.slice(start, cursor).search(/\s/);
    if (space < 0) return [];
    start += space + 1;
  }
  while (start < cursor && /\s/.test(text[start])) start++;
  if (text[start] === '!') start++;
  let end = cursor;
  while (end < lineEnd && text[end] !== ';' && (!sequence || text[end] !== ',')) end++;
  while (end > cursor && /\s/.test(text[end - 1])) end--;
  const query = text.slice(start, cursor).toLowerCase();
  const items = ['/use', '/userandom', '/equip', '/equipslot'].includes(cmd);
  const spells = [
    '/cast',
    '/castsequence',
    '/castrandom',
    '/cancelaura',
    '/use',
    '#show',
    '#showtooltip',
    '/petautocaston',
    '/petautocastoff',
    '/petautocasttoggle',
  ].includes(cmd);
  return names
    .filter(
      (n) =>
        ((items && n.kind === 'item') || (spells && n.kind === 'spell')) &&
        (n.name.toLowerCase().includes(query) || (n.id > 0 && String(n.id) === query)),
    )
    .sort(
      (a, b) =>
        Number(!a.name.toLowerCase().startsWith(query)) -
          Number(!b.name.toLowerCase().startsWith(query)) ||
        a.name.length - b.name.length ||
        a.name.localeCompare(b.name),
    )
    .slice(0, 50)
    .map((n) => ({
      label: n.name,
      insert: n.name,
      start,
      end,
      kind: n.kind,
      detail: `${n.kind}${n.id ? ' #' + n.id : ''}. Reference name; verify availability in your character's spellbook.`,
    }));
}
function distance(a: string, b: string): number {
  const row = Array.from({ length: b.length + 1 }, (_, i) => i);
  for (let i = 1; i <= a.length; i++) {
    let previous = row[0];
    row[0] = i;
    for (let j = 1; j <= b.length; j++) {
      const old = row[j];
      row[j] = Math.min(row[j] + 1, row[j - 1] + 1, previous + Number(a[i - 1] !== b[j - 1]));
      previous = old;
    }
  }
  return row[b.length];
}
export function diagnose(name: string, text: string, limit = 255): Issue[] {
  const issues: Issue[] = [];
  const add = (
    start: number,
    end: number,
    severity: Issue['severity'],
    message: string,
    code: string,
    fix?: Issue['fix'],
  ): void => {
    issues.push({ start, end, severity, message, code, fix });
  };
  const normalized = normalize(text),
    length = chars(normalized);
  if (length > limit)
    add(
      0,
      Math.min(1, text.length),
      'error',
      `${length} characters: ${length - limit} over the ${limit}-character macro limit. The full draft is preserved.`,
      'budget',
    );
  else if (bytes(normalized) > limit)
    add(
      0,
      Math.min(1, text.length),
      'warning',
      `${bytes(normalized)} UTF-8 bytes. Non-ASCII text may exceed the client's byte budget; verify after pasting.`,
      'bytes',
    );
  if (chars(name) > 16)
    add(
      0,
      Math.min(1, text.length),
      'warning',
      'Name exceeds the 16-character in-game macro name field.',
      'name',
    );
  let offset = 0;
  for (const line of text.split('\n')) {
    const m = /^(\s*)(\S+)([^\r\n]*)\r?$/.exec(line);
    if (!m) {
      offset += line.length + 1;
      continue;
    }
    const cmd = m[2].toLowerCase(),
      base = offset + m[1].length;
    if (!command(cmd) && !plain.has(cmd)) {
      let fix: Issue['fix'];
      if (cmd.startsWith('/')) {
        const nearest = refs.commands
          .map((r) => ({ name: r.name, d: distance(cmd, r.name) }))
          .sort((a, b) => a.d - b.d)[0];
        if (nearest.d <= 2)
          fix = {
            start: base,
            end: base + cmd.length,
            text: nearest.name,
            title: `Change to ${nearest.name}`,
          };
      }
      add(
        base,
        base + cmd.length,
        'warning',
        cmd.startsWith('/')
          ? `Unrecognized command ${cmd}. It may come from an addon or another client.`
          : cmd.startsWith('#')
            ? `Unrecognized directive ${cmd}.`
            : 'This line has no slash command; WoW may send it as chat text.',
        'command',
        fix,
      );
    }
    if (required.has(cmd) && !m[3].trim())
      add(base, base + cmd.length, 'warning', `${cmd} needs an argument.`, 'argument');
    if (conditional(cmd)) {
      let opening = -1;
      for (let i = m[1].length + m[2].length; i < line.length; i++) {
        if (line[i] === '[') {
          if (opening >= 0)
            add(
              offset + i,
              offset + i + 1,
              'error',
              'Nested conditional brackets are not supported.',
              'nested',
            );
          opening = i;
        }
        if (line[i] === ']') {
          if (opening < 0)
            add(offset + i, offset + i + 1, 'error', 'Closing ] has no matching [.', 'closing');
          else {
            for (const match of line.slice(opening + 1, i).matchAll(/[^,]+/g)) {
              const value = match[0].trim(),
                key = value.split(':')[0].toLowerCase();
              const start =
                offset + opening + 1 + match.index! + match[0].length - match[0].trimStart().length;
              if (
                value &&
                !value.startsWith('@') &&
                !value.startsWith('target=') &&
                !refs.conditions.some((r) => r.name === key || r.name === key.replace(/^no/, ''))
              )
                add(
                  start,
                  start + key.length,
                  'warning',
                  `Unrecognized condition '${key}'. Check spelling and client support.`,
                  'condition',
                );
              if (value.endsWith(':'))
                add(
                  start,
                  start + value.length,
                  'error',
                  `'${value}' needs a value after the colon.`,
                  'value',
                );
            }
          }
          opening = -1;
        }
      }
      if (opening >= 0) {
        const end = offset + line.trimEnd().length;
        add(
          offset + opening,
          offset + opening + 1,
          'error',
          'Unclosed conditional group: add ].',
          'bracket',
          { start: end, end, text: ']', title: 'Insert closing ] at end of line' },
        );
      }
    }
    if (isLua(cmd)) {
      const begin = m[1].length + m[2].length,
        mask = luaMask(line.slice(begin));
      const stack: Array<[string, number]> = [];
      for (let i = 0; i < mask.code.length; i++) {
        const ch = mask.code[i];
        if ('([{'.includes(ch)) stack.push([ch, i]);
        else if (')]}'.includes(ch)) {
          const expected = { '(': ')', '[': ']', '{': '}' }[stack.at(-1)?.[0] ?? ''];
          if (expected === ch) stack.pop();
          else
            add(
              offset + begin + i,
              offset + begin + i + 1,
              'error',
              `Unmatched Lua delimiter '${ch}'.`,
              'lua',
            );
        }
      }
      for (const [ch, i] of stack)
        add(
          offset + begin + i,
          offset + begin + i + 1,
          'error',
          `Unclosed Lua delimiter '${ch}'.`,
          'lua',
        );
      for (const [i, kind] of mask.unclosed)
        add(
          offset + begin + i,
          offset + begin + i + 1,
          'error',
          `Unclosed Lua ${kind}. Each /run command is one line.`,
          'lua-string',
        );
    }
    offset += line.length + 1;
  }
  return issues;
}
export function referenceAt(
  text: string,
  offset: number,
  names: Name[],
): { title: string; description: string; example?: string } | undefined {
  const start = text.lastIndexOf('\n', offset - 1) + 1,
    end = text.indexOf('\n', offset),
    line = text.slice(start, end < 0 ? text.length : end);
  for (const m of line.matchAll(/[@/#]?[\p{L}\p{N}_]+(?:[.:=][\p{L}\p{N}_/]*)*/gu))
    if (offset >= start + m.index! && offset <= start + m.index! + m[0].length) {
      const word = m[0],
        key = word.split(':')[0].toLowerCase();
      const found = [...refs.commands, ...refs.conditions, ...refs.targets, ...refs.lua].find(
        (r) => r.name === word || r.name === key,
      );
      if (found) return { title: word, description: found.description, example: found.example };
      const positive = refs.conditions.find((r) => r.name === key.replace(/^no/, ''));
      if (key.startsWith('no') && positive)
        return {
          title: word,
          description: 'Negate: ' + positive.description,
          example: positive.example,
        };
    }
  for (const entry of names) {
    const i = line.toLowerCase().indexOf(entry.name.toLowerCase());
    if (i >= 0 && offset >= start + i && offset <= start + i + entry.name.length)
      return {
        title: entry.name,
        description: `${entry.kind}${entry.id ? ' #' + entry.id : ''}. Check your character's spellbook for availability.`,
      };
  }
  return undefined;
}
