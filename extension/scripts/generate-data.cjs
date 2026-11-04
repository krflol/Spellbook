// Generate the VS Code vocabulary from the desktop source of truth.
const fs = require('node:fs');
const path = require('node:path');
const root = path.resolve(__dirname, '../..');
const extension = path.join(root, 'extension');
const check = process.argv.includes('--check');
const literal = '"(?:[^"\\\\]|\\\\.)*"';
function tuples(source, fields) {
  const expression = new RegExp(
    '\\(\\s*' +
      Array(fields)
        .fill('(' + literal + ')')
        .join('\\s*,\\s*') +
      '\\s*,?\\s*\\)',
    'g',
  );
  return [...source.matchAll(expression)].map((m) => m.slice(1).map((s) => JSON.parse(s)));
}
function save(name, text) {
  const file = path.join(extension, name);
  if (check) {
    if (!fs.existsSync(file) || fs.readFileSync(file, 'utf8').replaceAll('\r\n', '\n') !== text)
      throw new Error(`Regenerate ${name}: npm run generate`);
  } else {
    fs.mkdirSync(path.dirname(file), { recursive: true });
    fs.writeFileSync(file, text);
  }
}
const language = fs.readFileSync(path.join(root, 'src/macros/language.rs'), 'utf8');
const data = {};
for (const key of ['COMMANDS', 'CONDITIONS', 'TARGETS', 'LUA']) {
  const source = language
    .split(`pub const ${key}: &[Reference] = references![`)[1]
    ?.split('\n];')[0];
  if (!source) throw new Error(`Missing reference section ${key}`);
  data[key.toLowerCase()] = tuples(source, 3).map(([name, description, example]) => ({
    name,
    description,
    example,
  }));
  if (!data[key.toLowerCase()].length) throw new Error(`Empty ${key}`);
}
const snippets = fs.readFileSync(path.join(root, 'src/macros/snippets.rs'), 'utf8');
const guidance = {};
for (const [category, description] of [
  ...snippets.matchAll(new RegExp('(' + literal + ')\\s*=>\\s*Some\\((' + literal + ')\\)', 'g')),
].map((m) => m.slice(1).map((s) => JSON.parse(s))))
  guidance[category] = description;
data.samples = tuples(snippets, 5).map(([name, cls, category, description, body]) => ({
  name,
  class: cls,
  category,
  description,
  body,
  notes: [description, guidance[category]].filter(Boolean).join('\n\n'),
}));
data.names = fs
  .readFileSync(path.join(root, 'assets/names.txt'), 'utf8')
  .split(/\r?\n/)
  .filter((line) => line && !line.startsWith('#'))
  .map((line) => {
    const [kind, name] = line.split('|');
    return { name, kind, id: 0 };
  });
if (data.samples.length !== 36) throw new Error('Unexpected number of samples');
save('src/data.json', JSON.stringify(data, null, 2) + '\n');
const completions = {};
for (const sample of data.samples)
  completions[sample.name] = {
    prefix: 'sb-' + sample.name.toLowerCase().replace(/[^a-z0-9]+/g, '-'),
    body: sample.body.split('\n').map((line) => line.replace(/[$}\\]/g, '\\$&')),
    description: sample.notes,
  };
save('snippets/wowmacro.json', JSON.stringify(completions, null, 2) + '\n');
let reference =
  '# Spellbook macro reference\n\nUse Ctrl+Space for context-aware completions. Problems checks structure; it does not execute Lua or simulate gameplay. Samples target Forever / Classic-style mechanics. Verify learned form slots and available spells in game.\n\n';
for (const key of ['commands', 'conditions', 'targets', 'lua']) {
  reference += `## ${key === 'lua' ? 'Lua & API' : key[0].toUpperCase() + key.slice(1)}\n\n`;
  for (const r of data[key])
    reference += `### ${r.name}\n\n${r.description}\n\n\`\`\`wowmacro\n${r.example}\n\`\`\`\n\n`;
}
save('docs/REFERENCE.md', reference.trimEnd() + '\n');
if (!check) {
  fs.copyFileSync(
    path.join(root, 'assets/spellbook.png'),
    path.join(extension, 'media/spellbook.png'),
  );
  fs.copyFileSync(path.join(root, 'LICENSE'), path.join(extension, 'LICENSE'));
}
console.log(
  `Shared data: ${data.samples.length} samples, ${data.names.length} names, ${data.commands.length} commands.`,
);
