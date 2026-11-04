// Render illustrated README previews from the extension's actual language data.
// Requires: cd extension && npm run compile; ImageMagick 7 on PATH.
const fs = require('node:fs');
const path = require('node:path');
const { execFileSync } = require('node:child_process');
const { complete } = require('../extension/out/language');
const { refs, mergeNames, chars } = require('../extension/out/model');
const root = path.resolve(__dirname, '..');
const frames = path.join(root, '.capture-data/readme-media');
const output = path.join(root, 'docs/screenshots');
const magick = process.env.MAGICK_BIN || 'magick';
fs.mkdirSync(frames, { recursive: true });
fs.mkdirSync(output, { recursive: true });

const xml = (value) =>
  String(value).replace(
    /[&<>"']/g,
    (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&apos;' })[c],
  );
const text = (x, y, value, size = 18, color = '#dce4eb', extra = '') =>
  `<text x="${x}" y="${y}" fill="${color}" font-family="Segoe UI,Arial,sans-serif" font-size="${size}" ${extra}>${xml(value)}</text>`;
const syntax = (value) =>
  value
    .split(
      /(#[a-zA-Z]+|\/[a-zA-Z]+|@\w+|\b(?:nostance|noform|form|mod|help|harm|nodead|nocombat|nostealth)\b|\b\d+\b)/g,
    )
    .map((part) => {
      const color = /^[#/]/.test(part)
        ? '#8dc8f1'
        : /^@/.test(part)
          ? '#7cdbbe'
          : /^(nostance|noform|form|mod|help|harm|nodead|nocombat|nostealth)$/.test(part)
            ? '#c1b0f7'
            : /^\d+$/.test(part)
              ? '#e1bc70'
              : '#dce4eb';
      return `<tspan fill="${color}">${xml(part)}</tspan>`;
    })
    .join('');
const code = (x, y, value, size = 21) =>
  `<text x="${x}" y="${y}" font-family="Consolas,monospace" font-size="${size}" xml:space="preserve">${syntax(value)}</text>`;
const svg = (height, description, content) =>
  `<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="${height}" viewBox="0 0 1200 ${height}" role="img" aria-label="${xml(description)}"><rect width="1200" height="${height}" rx="16" fill="#111820"/>${content}</svg>`;

const scenes = [
  { line: '/c', stage: 0, delay: 70 },
  { line: '/ca', stage: 0, delay: 130 },
  { line: '/cast [@', stage: 1, delay: 70 },
  { line: '/cast [@m', stage: 1, delay: 150 },
  { line: '/cast [@mouseover,he', stage: 2, delay: 180 },
  { line: '/cast [@mouseover,help,nodead][] Fl', stage: 3, delay: 70 },
  { line: '/cast [@mouseover,help,nodead][] Flash', stage: 3, delay: 160 },
  { line: '/cast [@mouseover,help,nodead][] Flash Heal', stage: 4, delay: 280 },
];
const labels = ['Command', 'Target', 'Conditions', 'Spell', 'Ready to paste'];
const animation = [];
for (const [index, scene] of scenes.entries()) {
  const source = '#showtooltip\n' + scene.line;
  const suggestions =
    scene.stage === 4 ? [] : complete(source, source.length, mergeNames([])).slice(0, 5);
  if (scene.stage !== 4 && !suggestions.length)
    throw new Error('Expected real completion results for ' + scene.line);
  let content =
    text(40, 52, 'Build a mouseover heal', 28, '#eef2f4', 'font-weight="600"') +
    text(
      1158,
      46,
      'ILLUSTRATED WORKFLOW',
      12,
      '#87a4b4',
      'text-anchor="end" letter-spacing="1.4"',
    ) +
    '<rect x="40" y="87" width="1120" height="372" rx="10" fill="#17212c" stroke="#30414e"/>' +
    '<path d="M40 135h1120" stroke="#30414e"/>' +
    text(65, 117, 'Mouseover heal.wowmacro', 16, '#c6d0d8') +
    text(1135, 117, `${chars(source)} / 255 chars`, 14, '#7cdbbe', 'text-anchor="end"') +
    text(70, 187, '1', 17, '#5f7485') +
    text(70, 226, '2', 17, '#5f7485') +
    code(113, 187, '#showtooltip') +
    code(113, 226, scene.line) +
    `<rect x="${113 + scene.line.length * 12}" y="208" width="2" height="23" fill="#e1bc70"/>`;
  if (suggestions.length) {
    const x = Math.min(680, 113 + Math.max(0, scene.line.lastIndexOf('[') + 1) * 11.55);
    const y = 247,
      height = 37 * suggestions.length + 15;
    content += `<rect x="${x}" y="${y}" width="440" height="${height}" rx="7" fill="#202e3c" stroke="#43637b"/>`;
    suggestions.forEach((suggestion, i) => {
      if (i === 0)
        content += `<rect x="${x + 5}" y="${y + 6}" width="430" height="34" rx="4" fill="#274f65"/>`;
      content +=
        text(x + 19, y + 30 + i * 37, suggestion.label, 19, '#e5edf1') +
        text(x + 422, y + 29 + i * 37, suggestion.kind, 12, '#8eb0c2', 'text-anchor="end"');
    });
  } else {
    content +=
      '<rect x="113" y="283" width="1000" height="104" rx="8" fill="#15332e" stroke="#286052"/>' +
      text(139, 321, 'Friendly mouseover → current target', 23, '#a2e1cb', 'font-weight="600"') +
      text(139, 355, 'Copy the macro, then paste into WoW.', 18, '#9fbfb7');
  }
  labels.forEach((label, i) => {
    const x = 53 + i * 226,
      active = i === scene.stage;
    content +=
      `<circle cx="${x}" cy="501" r="12" fill="${active ? '#e1bc70' : '#283845'}"/>` +
      text(
        x,
        506,
        i + 1,
        13,
        active ? '#17212c' : '#92a6b5',
        'text-anchor="middle" font-weight="600"',
      ) +
      text(x + 23, 507, label, 16, active ? '#f1dfb4' : '#91a6b4');
  });
  const filename = path.join(frames, `frame-${String(index).padStart(2, '0')}`);
  fs.writeFileSync(
    filename + '.svg',
    svg(540, 'Illustrated Spellbook autocomplete workflow: ' + labels[scene.stage], content),
  );
  execFileSync(magick, ['-background', 'none', filename + '.svg', filename + '.png']);
  animation.push('-delay', String(scene.delay), filename + '.png');
}
execFileSync(magick, [
  ...animation,
  '-loop',
  '0',
  '-layers',
  'Optimize',
  path.join(output, 'autocomplete-walkthrough.gif'),
]);

const samples = [
  { name: 'Charge dance', heading: 'Stance dancing', color: '#e1bc70' },
  { name: 'Cat Prowl', heading: 'Shapeshifting', color: '#70d7bd' },
  { name: 'Form attack', heading: 'Form-sensitive actions', color: '#b6a4f1' },
];
let gallery =
  text(40, 51, 'Start from a sample. Make it yours.', 28, '#eef2f4', 'font-weight="600"') +
  text(
    1158,
    47,
    '3 OF 36 INCLUDED SAMPLES',
    12,
    '#87a4b4',
    'text-anchor="end" letter-spacing="1.3"',
  );
samples.forEach((item, index) => {
  const sample = refs.samples.find((s) => s.name === item.name);
  if (!sample) throw new Error('Missing sample ' + item.name);
  const y = 87 + index * 137;
  gallery +=
    `<rect x="40" y="${y}" width="1120" height="123" rx="9" fill="#17212c" stroke="#2b3d4b"/><rect x="40" y="${y + 17}" width="3" height="89" rx="1" fill="${item.color}"/>` +
    text(64, y + 45, item.heading, 21, item.color, 'font-weight="600"') +
    text(64, y + 76, sample.class + ' · ' + sample.name, 15, '#9db1bf');
  sample.body.split('\n').forEach((line, i) => {
    gallery += code(350, y + 45 + i * 36, line, 18);
  });
});
gallery += text(
  40,
  531,
  'Read the sample notes and verify your character’s stance / form slots with Inspect forms.',
  16,
  '#9db1bf',
);
fs.writeFileSync(
  path.join(output, 'macro-samples.svg'),
  svg(
    560,
    'Three included Spellbook samples: Charge dance, Cat Prowl, and Form attack. Illustrated code cards.',
    gallery,
  ),
);
execFileSync(magick, [
  '-background',
  'none',
  path.join(output, 'macro-samples.svg'),
  path.join(frames, 'samples-preview.png'),
]);
execFileSync(magick, [
  '-background',
  'none',
  path.join(output, 'macro-samples.svg'),
  path.join(output, 'macro-samples.png'),
]);
execFileSync(magick, [
  '-background',
  'none',
  path.join(output, 'spellbook-banner.svg'),
  path.join(output, 'spellbook-banner.png'),
]);
console.log(
  'Rendered the README GIF, sample cards, and PNG previews from Spellbook language data.',
);
