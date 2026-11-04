import data from './data.json';

export const CLASSES = [
  'Any class',
  'Druid',
  'Hunter',
  'Mage',
  'Paladin',
  'Priest',
  'Rogue',
  'Shaman',
  'Warlock',
  'Warrior',
];
export const DRAFT_LIMIT = 65536;
export const IMPORT_LIMIT = 8 * 1024 * 1024;
export interface MacroDocument {
  name: string;
  body: string;
  class: string;
  character: string;
  tags: string;
  notes: string;
  favorite: boolean;
}
export interface Reference {
  name: string;
  description: string;
  example: string;
}
export interface Sample {
  name: string;
  class: string;
  category: string;
  description: string;
  body: string;
  notes: string;
}
export interface Name {
  name: string;
  kind: 'spell' | 'item';
  id: number;
}
export const refs = data as {
  commands: Reference[];
  conditions: Reference[];
  targets: Reference[];
  lua: Reference[];
  samples: Sample[];
  names: Name[];
};
export const normalize = (text: string): string =>
  text.replace(/^\uFEFF/, '').replace(/\r\n?/g, '\n');
export const chars = (text: string): number => Array.from(text).length;
export const bytes = (text: string): number => Buffer.byteLength(text, 'utf8');
export function object(value: unknown): asserts value is Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value))
    throw new Error('Expected a JSON object.');
}
export function document(value: unknown): MacroDocument {
  object(value);
  const defaults: MacroDocument = {
    name: 'Untitled macro',
    body: '',
    class: 'Any class',
    character: '',
    tags: '',
    notes: '',
    favorite: false,
  };
  for (const key of Object.keys(value))
    if (!Object.hasOwn(defaults, key)) throw new Error(`Unknown macro field: ${key}`);
  const doc = { ...defaults, ...value };
  for (const key of ['name', 'body', 'class', 'character', 'tags', 'notes'] as const)
    if (typeof doc[key] !== 'string') throw new Error(`Macro ${key} must be text.`);
  if (typeof doc.favorite !== 'boolean') throw new Error('Macro favorite must be a boolean.');
  doc.body = normalize(doc.body);
  for (const [key, limit] of [
    ['name', 512],
    ['body', DRAFT_LIMIT],
    ['notes', DRAFT_LIMIT],
    ['class', 128],
    ['character', 512],
    ['tags', 2048],
  ] as const) {
    if (bytes(doc[key]) > limit) throw new Error(`Macro ${key} exceeds ${limit} bytes.`);
  }
  if (doc.body.includes('\0')) throw new Error('Macro text contains a NUL character.');
  return doc;
}
export function fromSample(sample: Sample): MacroDocument {
  return document({
    name: sample.name,
    body: sample.body,
    class: sample.class,
    tags: sample.category.toLowerCase(),
    notes: sample.notes,
  });
}
export function decodeLibrary(text: string, name = 'Imported macro'): MacroDocument[] {
  if (bytes(text) > IMPORT_LIMIT) throw new Error('Import exceeds 8 MiB.');
  text = normalize(text);
  if (!text.trimStart().startsWith('{')) return [document({ name, body: text })];
  const value: unknown = JSON.parse(text);
  object(value);
  if (
    !['spellbook-library', 'forever-atlas-macros'].includes(String(value.format)) ||
    value.version !== 1
  )
    throw new Error('Unsupported macro library format or version.');
  if (typeof value.build !== 'string' || !Array.isArray(value.macros) || value.macros.length > 2000)
    throw new Error('Invalid library, or more than 2,000 macros.');
  if (Object.keys(value).some((key) => !['format', 'version', 'build', 'macros'].includes(key)))
    throw new Error('Unknown library field.');
  return value.macros.map(document);
}
export function encodeLibrary(documents: MacroDocument[]): string {
  const text =
    JSON.stringify(
      {
        format: 'spellbook-library',
        version: 1,
        build: 'wow-forever',
        macros: documents.map(document),
      },
      null,
      2,
    ) + '\n';
  if (documents.length > 2000 || bytes(text) > IMPORT_LIMIT)
    throw new Error(
      'Library exceeds the transferable 8 MiB / 2,000 macro limit. Export smaller groups.',
    );
  return text;
}
export function decodeCatalogue(text: string): Name[] {
  if (bytes(text) > IMPORT_LIMIT) throw new Error('Catalogue exceeds 8 MiB.');
  const value: unknown = JSON.parse(normalize(text));
  object(value);
  if (
    value.format !== 'spellbook-catalogue' ||
    value.version !== 1 ||
    !Array.isArray(value.names) ||
    value.names.length > 50000
  )
    throw new Error('Unsupported catalogue or more than 50,000 names.');
  if (Object.keys(value).some((k) => !['format', 'version', 'names'].includes(k)))
    throw new Error('Unknown catalogue field.');
  return value.names.map((entry: unknown) => {
    object(entry);
    if (Object.keys(entry).some((k) => !['name', 'kind', 'id'].includes(k)))
      throw new Error('Unknown catalogue entry field.');
    if (typeof entry.name !== 'string' || !['spell', 'item'].includes(String(entry.kind)))
      throw new Error('Invalid catalogue name or kind.');
    const name = entry.name.trim(),
      id = entry.id ?? 0;
    if (
      !name ||
      bytes(name) > 256 ||
      /[\x00-\x1f\x7f]/.test(name) ||
      !Number.isSafeInteger(id) ||
      Number(id) < 0
    )
      throw new Error('Invalid catalogue name or ID.');
    return { name, kind: entry.kind as Name['kind'], id: Number(id) };
  });
}
export function mergeNames(custom: Name[]): Name[] {
  const names = new Map<string, Name>();
  for (const entry of [...refs.names, ...custom])
    names.set(entry.kind + ':' + entry.name.toLocaleLowerCase(), entry);
  return [...names.values()];
}
