import test from 'node:test';
import assert from 'node:assert/strict';
import * as fs from 'node:fs/promises';
import * as path from 'node:path';
import * as os from 'node:os';
import { complete, diagnose, luaMask, referenceAt } from '../language';
import {
  document,
  fromSample,
  refs,
  decodeLibrary,
  encodeLibrary,
  decodeCatalogue,
  mergeNames,
} from '../model';
import { LibraryStore, ConflictError } from '../store';
import { Registry, parseRawGrammar, INITIAL } from 'vscode-textmate';
import { loadWASM, OnigScanner, OnigString } from 'vscode-oniguruma';

test('contextual completions replace full tokens and preserve alternative modifier values', () => {
  const names = mergeNames([]);
  for (const [source, expected] of [
    ['/ca', '/cast'],
    ['/cast [@m', '@mouseover'],
    ['/cast [nofo', 'noform'],
    ['/cast [mod:sh', 'shift'],
    ['/run C_Spell.GetS', 'C_Spell.GetSpellName'],
    ['/cast !Cat F', 'Cat Form'],
    ['/use Healing P', 'Healing Potion'],
    ['/castsequence reset=target/10 Corruption, Curse of A', 'Curse of Agony'],
  ])
    assert.ok(
      complete(source, source.length, names).some((s) => s.label === expected),
      source,
    );
  const source = '/cast [mod:sh/ctrl] Fireball',
    cursor = source.indexOf('sh') + 2,
    s = complete(source, cursor, names).find((s) => s.label === 'shift')!;
  assert.equal(
    source.slice(0, s.start) + s.insert + source.slice(s.end),
    '/cast [mod:shift/ctrl] Fireball',
  );
  const middle = '/cast Firebxxx',
    suggestion = complete(middle, 11, names).find((s) => s.label === 'Fireball')!;
  assert.equal(
    middle.slice(0, suggestion.start) + suggestion.insert + middle.slice(suggestion.end),
    '/cast Fireball',
  );
  assert.equal(complete('/say [hello]', 12, names).length, 0);
  assert.equal(complete('/run print("Unit', 16, names).length, 0);
  assert.equal(complete('/cast 0', 7, names).length, 0);
});
test('diagnostics separate conditional macros, chat, Lua strings and UTF-16 positions', () => {
  assert.equal(diagnose('chat', '/say [hello]').length, 0);
  assert.equal(diagnose('Lua', '/run print("(") -- [ comment').length, 0);
  assert.equal(diagnose('Lua', '/run print([=[(]=])').length, 0);
  assert.ok(diagnose('Lua', '/run print("x"').some((i) => i.code === 'lua'));
  assert.ok(diagnose('Lua', '/run print("x').some((i) => i.code === 'lua-string'));
  const source = '/say 🐈\r\n/cast [harm Fireball',
    issue = diagnose('draft', source).find((i) => i.code === 'bracket')!;
  assert.equal(source[issue.start], '[');
  assert.equal(issue.start, source.indexOf('['));
  const crlf = '/cast [harm Fireball\r\n/run print("Hello"';
  assert.ok(diagnose('Windows', crlf).some((i) => i.code === 'bracket'));
  assert.ok(diagnose('Windows', crlf).some((i) => i.code === 'lua'));
  assert.ok(diagnose('limit', 'x'.repeat(256)).some((i) => i.code === 'budget'));
  assert.ok(diagnose('limit', 'é'.repeat(200)).some((i) => i.code === 'bytes'));
  const typo = diagnose('typo', '/castt Fireball').find((i) => i.code === 'command')!;
  assert.equal(typo.fix?.text, '/cast');
  assert.equal(luaMask('print("hi\\\"(")').code.includes('hi'), false);
  assert.ok(referenceAt('/cast [noform:3] Cat Form', 12, mergeNames([]))?.description);
});
test('all shipped samples fit and have no structural errors', () => {
  assert.equal(refs.samples.length, 36);
  for (const sample of refs.samples) {
    const issues = diagnose(sample.name, sample.body);
    assert.deepEqual(
      issues.filter((i) => i.severity === 'error'),
      [],
      sample.name,
    );
    assert.ok(fromSample(sample).notes.includes(sample.description));
  }
});
test('Atlas and Spellbook imports preserve metadata and reject invalid input atomically', () => {
  const original = document({
    name: '猫 form',
    body: '/cast !Cat Form\r\n',
    class: 'Druid',
    notes: 'Keep these notes',
    favorite: true,
  });
  for (const format of ['forever-atlas-macros', 'spellbook-library'])
    assert.deepEqual(
      decodeLibrary(JSON.stringify({ format, version: 1, build: 'old-build', macros: [original] })),
      [original],
    );
  assert.deepEqual(decodeLibrary(encodeLibrary([original])), [original]);
  assert.throws(() =>
    decodeLibrary('{"format":"spellbook-library","version":99,"build":"a","macros":[]}'),
  );
  assert.throws(() => document({ body: 'x'.repeat(65537) }));
  assert.throws(() => document(JSON.parse('{"toString":"bad"}')));
  assert.throws(() => document({ favorite: 'yes' }));
});
test('catalogue names merge without duplicate kinds and validate before installation', () => {
  const custom = decodeCatalogue(
    JSON.stringify({
      format: 'spellbook-catalogue',
      version: 1,
      names: [
        { name: 'Fireball', kind: 'spell', id: 133 },
        { name: 'Éclat lunaire', kind: 'spell' },
      ],
    }),
  );
  const names = mergeNames(custom);
  assert.equal(names.filter((n) => n.name === 'Fireball').length, 1);
  assert.equal(names.find((n) => n.name === 'Fireball')?.id, 133);
  assert.ok(complete('/cast Éc', 8, names).some((n) => n.label === 'Éclat lunaire'));
  assert.throws(() =>
    decodeCatalogue(
      JSON.stringify({
        format: 'spellbook-catalogue',
        version: 1,
        names: [{ name: 'bad\nname', kind: 'spell' }],
      }),
    ),
  );
});
test('persistent library survives restart, rejects concurrent edits, restores history and trash', async () => {
  const dir = await fs.mkdtemp(path.join(os.tmpdir(), 'spellbook-store-'));
  try {
    const first = new LibraryStore(dir),
      second = new LibraryStore(dir);
    const [original] = await first.createMany([document({ name: 'Test', body: '/cast Fireball' })]);
    const saved = await second.update(
      original.id,
      { ...original.document, body: '/cast Frostbolt' },
      1,
    );
    await assert.rejects(
      first.update(original.id, { ...original.document, body: '/cast Blink' }, 1),
      ConflictError,
    );
    const restored = await first.restore(saved.id, 1, 2);
    assert.equal(restored.revision, 3);
    assert.equal(restored.history[1].document.body, '/cast Frostbolt');
    await first.archive(restored.id, true, 3);
    assert.equal((await second.get(restored.id)).archived, true);
    await second.archive(restored.id, false, 4);
    assert.equal((await new LibraryStore(dir).get(restored.id)).archived, false);
    const before = await fs.readFile(first.file, 'utf8');
    await assert.rejects(
      first.createMany([
        document({ name: 'valid' }),
        { ...original.document, body: 'x'.repeat(65537) },
      ]),
    );
    assert.equal(await fs.readFile(first.file, 'utf8'), before);
    await Promise.all([
      first.createMany([document({ name: 'A' })]),
      second.createMany([document({ name: 'B' })]),
    ]);
    assert.equal((await first.list()).length, 3);
  } finally {
    await fs.rm(dir, { recursive: true, force: true });
  }
});
test('a damaged local library is never silently replaced', async () => {
  const dir = await fs.mkdtemp(path.join(os.tmpdir(), 'spellbook-damaged-'));
  try {
    const store = new LibraryStore(dir);
    await fs.writeFile(store.file, '{bad json');
    await assert.rejects(store.createMany([document({ name: 'Test' })]));
    assert.equal(await fs.readFile(store.file, 'utf8'), '{bad json');
  } finally {
    await fs.rm(dir, { recursive: true, force: true });
  }
});
test('TextMate grammar highlights commands and conditions without treating chat as code', async () => {
  const wasm = await fs.readFile(require.resolve('vscode-oniguruma/release/onig.wasm'));
  await loadWASM(wasm.buffer.slice(wasm.byteOffset, wasm.byteOffset + wasm.byteLength));
  const grammarFile = path.resolve(__dirname, '../../syntaxes/wowmacro.tmLanguage.json');
  const registry = new Registry({
    onigLib: Promise.resolve({
      createOnigScanner: (patterns) => new OnigScanner(patterns),
      createOnigString: (content) => new OnigString(content),
    }),
    loadGrammar: async (scope) =>
      scope === 'source.wowmacro'
        ? parseRawGrammar(await fs.readFile(grammarFile, 'utf8'), grammarFile)
        : null,
  });
  const grammar = (await registry.loadGrammar('source.wowmacro'))!;
  const line = '/cast [@mouseover,help,nodead][] Flash Heal',
    tokens = grammar.tokenizeLine(line, INITIAL).tokens;
  assert.ok(tokens.some((t) => t.scopes.includes('keyword.control.command.wowmacro')));
  assert.ok(tokens.some((t) => t.scopes.includes('variable.language.unit.wowmacro')));
  assert.ok(tokens.some((t) => t.scopes.includes('support.function.condition.wowmacro')));
  assert.ok(
    grammar
      .tokenizeLine('/say [hello]', INITIAL)
      .tokens.some((t) => t.scopes.includes('string.unquoted.chat.wowmacro')),
  );
  assert.ok(
    !grammar
      .tokenizeLine('/say [hello]', INITIAL)
      .tokens.some((t) => t.scopes.includes('meta.condition.wowmacro')),
  );
  registry.dispose();
});
