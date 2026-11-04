const { runTests } = require('@vscode/test-electron');
const path = require('node:path');
const fs = require('node:fs');
const root = path.resolve(__dirname, '..');
const run = path.join(root, '.test-host', String(Date.now()));
const userData = path.join(run, 'user-data');
const workspace = path.join(run, 'workspace');
fs.mkdirSync(path.join(userData, 'User'), { recursive: true });
fs.mkdirSync(workspace, { recursive: true });
fs.writeFileSync(
  path.join(userData, 'User', 'settings.json'),
  JSON.stringify({
    'workbench.startupEditor': 'none',
    'security.workspace.trust.enabled': false,
    'update.mode': 'none',
    'telemetry.telemetryLevel': 'off',
    'extensions.ignoreRecommendations': true,
    'chat.disableAIFeatures': true,
    'spellbook.autoSaveLibrary': false,
  }),
);
const report = path.join(run, 'results.json');
runTests({
  extensionDevelopmentPath: root,
  extensionTestsPath: path.join(root, 'out/test/host.js'),
  vscodeExecutablePath: process.env.SPELLBOOK_VSCODE_EXECUTABLE || undefined,
  extensionTestsEnv: { SPELLBOOK_TEST_REPORT: report },
  launchArgs: [
    workspace,
    '--user-data-dir',
    userData,
    '--extensions-dir',
    path.join(run, 'extensions'),
    '--disable-extensions',
    '--disable-workspace-trust',
    '--skip-welcome',
    '--skip-release-notes',
    '--disable-updates',
    '--disable-gpu',
    '--log',
    'error',
  ],
}).then(
  () => {
    console.log(fs.readFileSync(report, 'utf8'));
    console.log('Extension host report: ' + report);
  },
  (error) => {
    if (fs.existsSync(report)) console.error(fs.readFileSync(report, 'utf8'));
    console.error(error);
    process.exitCode = 1;
  },
);
