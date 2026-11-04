const { createVSIX } = require('@vscode/vsce');
const fs = require('node:fs/promises');
const path = require('node:path');
const crypto = require('node:crypto');
const manifest = require('../package.json');
const root = path.resolve(__dirname, '..');

async function main() {
  if (process.env.RELEASE_TAG && process.env.RELEASE_TAG !== `v${manifest.version}`) {
    throw new Error('Release tag must match extension/package.json version.');
  }
  const dist = path.resolve(root, '../dist');
  await fs.mkdir(dist, { recursive: true });
  const filename = `spellbook-${manifest.version}.vsix`;
  const packagePath = path.join(dist, filename);
  await createVSIX({ cwd: root, packagePath, dependencies: false });
  const digest = crypto
    .createHash('sha256')
    .update(await fs.readFile(packagePath))
    .digest('hex');
  await fs.writeFile(path.join(dist, 'spellbook-vsix-SHA256SUMS.txt'), `${digest}  ${filename}\n`);
  console.log(`Packaged ${packagePath}\nSHA-256 ${digest}`);
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
