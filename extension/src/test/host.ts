import * as vscode from 'vscode';
import assert from 'node:assert/strict';
import * as fs from 'node:fs/promises';
import { Spellbook } from '../extension';
import { refs, fromSample } from '../model';
import { macroUri, notesUri } from '../library-fs';

const pause = (ms: number): Promise<void> => new Promise((resolve) => setTimeout(resolve, ms));
async function until(predicate: () => boolean | Promise<boolean>, label: string): Promise<void> {
  const end = Date.now() + 8000;
  while (!(await predicate())) {
    if (Date.now() > end) throw new Error('Timed out: ' + label);
    await pause(40);
  }
}
export async function run(): Promise<void> {
  const results: Array<{ name: string; passed: boolean; error?: string }> = [];
  async function check(name: string, body: () => Promise<void>): Promise<void> {
    try {
      await body();
      results.push({ name, passed: true });
      console.log('PASS ' + name);
    } catch (error) {
      results.push({
        name,
        passed: false,
        error: error instanceof Error ? error.stack : String(error),
      });
      throw error;
    }
  }
  try {
    const extension = vscode.extensions.getExtension<Spellbook>('krflol.spellbook');
    assert.ok(extension);
    const app = await extension.activate();
    await check(
      'activation registers language, commands and isolated persistent library',
      async () => {
        assert.ok(extension.isActive);
        assert.ok(app.context.globalStorageUri.fsPath.includes('.test-host'));
        assert.equal((await app.store.list()).length, 0);
        assert.ok((await vscode.languages.getLanguages()).includes('wowmacro'));
        assert.ok((await vscode.commands.getCommands()).includes('spellbook.createFromSample'));
      },
    );
    await check('VS Code invokes contextual completion, hover and outline providers', async () => {
      const doc = await vscode.workspace.openTextDocument({
        language: 'wowmacro',
        content: '/cast [@m',
      });
      await vscode.window.showTextDocument(doc);
      const completion = await vscode.commands.executeCommand<vscode.CompletionList>(
        'vscode.executeCompletionItemProvider',
        doc.uri,
        new vscode.Position(0, doc.getText().length),
      );
      assert.ok(completion.items.some((i) => i.label === '@mouseover'));
      const other = await vscode.workspace.openTextDocument({
        language: 'wowmacro',
        content: '#showtooltip\n/cast [form:3] Cat Form',
      });
      await vscode.window.showTextDocument(other);
      const hover = await vscode.commands.executeCommand<vscode.Hover[]>(
        'vscode.executeHoverProvider',
        other.uri,
        new vscode.Position(1, 9),
      );
      assert.ok(hover.length > 0);
      const symbols = await vscode.commands.executeCommand<vscode.DocumentSymbol[]>(
        'vscode.executeDocumentSymbolProvider',
        other.uri,
      );
      assert.equal(symbols.length, 2);
      assert.ok(app.features.status.text.includes('chars'));
    });
    await check('Problems and code actions update after real editor changes', async () => {
      const doc = await vscode.workspace.openTextDocument({
          language: 'wowmacro',
          content: '/cast Fireball',
        }),
        editor = await vscode.window.showTextDocument(doc);
      await editor.edit((edit) =>
        edit.replace(
          new vscode.Range(doc.positionAt(0), doc.positionAt(doc.getText().length)),
          '/cast [harm Fireball\n/run print("Hello"',
        ),
      );
      await until(
        () => !!app.features.diagnostics.get(doc.uri)?.some((d) => d.code === 'bracket'),
        'diagnostics',
      );
      assert.ok(app.features.diagnostics.get(doc.uri)?.some((d) => d.code === 'lua'));
      const actions = await vscode.commands.executeCommand<vscode.CodeAction[]>(
        'vscode.executeCodeActionProvider',
        doc.uri,
        doc.lineAt(0).range,
      );
      assert.ok(actions.some((a) => a.title.includes('closing ]')));
    });
    await check(
      'sample creation, editor undo/save, markdown notes and history persist',
      async () => {
        const sample = refs.samples.find((s) => s.name === 'Cat Prowl')!;
        const preview = await vscode.workspace.openTextDocument(
          vscode.Uri.from({
            scheme: 'spellbook-sample',
            path: `/${refs.samples.indexOf(sample)}/sample.md`,
          }),
        );
        assert.ok(preview.getText().includes(sample.body));
        assert.equal(preview.isDirty, false);
        await vscode.commands.executeCommand('spellbook.createFromSample', {
          kind: 'sample',
          sample,
        });
        const editor = vscode.window.activeTextEditor!;
        assert.equal(editor.document.languageId, 'wowmacro');
        assert.equal(editor.document.getText(), sample.body);
        const record = (await app.store.list())[0];
        assert.equal(record.document.notes, fromSample(sample).notes);
        const body = sample.body + '\n/say 🐈';
        await editor.edit((edit) =>
          edit.replace(
            new vscode.Range(
              editor.document.positionAt(0),
              editor.document.positionAt(sample.body.length),
            ),
            body,
          ),
        );
        await vscode.commands.executeCommand('undo');
        assert.equal(editor.document.getText(), sample.body);
        await editor.edit((edit) =>
          edit.replace(
            new vscode.Range(
              editor.document.positionAt(0),
              editor.document.positionAt(sample.body.length),
            ),
            body,
          ),
        );
        assert.equal(await editor.document.save(), true);
        assert.equal((await app.store.get(record.id)).document.body, body);
        assert.equal((await app.store.get(record.id)).revision, 2);
        const notes = await vscode.workspace.openTextDocument(notesUri(record)),
          notesEditor = await vscode.window.showTextDocument(notes);
        assert.equal(notes.languageId, 'markdown');
        await notesEditor.edit((edit) =>
          edit.insert(notes.positionAt(notes.getText().length), '\nCharacter: test druid.'),
        );
        assert.equal(await notes.save(), true);
        assert.ok(
          (await app.store.get(record.id)).document.notes.endsWith('Character: test druid.'),
        );
        const revision = vscode.Uri.from({
          scheme: 'spellbook-revision',
          path: `/${record.id}/1/revision.wowmacro`,
        });
        assert.equal((await vscode.workspace.openTextDocument(revision)).getText(), sample.body);
        await vscode.commands.executeCommand('spellbook.restoreRevision', {
          kind: 'revision',
          record,
          revision: record.history[0],
        });
        await until(
          () => vscode.window.activeTextEditor?.document.getText() === sample.body,
          'restored document text',
        );
        assert.equal((await app.store.get(record.id)).revision, 4);
        assert.equal((await app.store.get(record.id)).history[1].document.body, body);
      },
    );
    await check(
      'library autosave and recoverable trash work through registered commands',
      async () => {
        await vscode.workspace
          .getConfiguration('spellbook')
          .update('autoSaveLibrary', true, vscode.ConfigurationTarget.Global);
        const record = (await app.store.list())[0],
          editor = await app.open(record);
        await editor.edit((edit) =>
          edit.insert(
            editor.document.positionAt(editor.document.getText().length),
            '\n/say Autosaved',
          ),
        );
        await until(
          async () =>
            !vscode.window.activeTextEditor?.document.isDirty &&
            (await app.store.get(record.id)).document.body.includes('Autosaved'),
          'library autosave',
        );
        await vscode.commands.executeCommand('spellbook.archiveMacro', { kind: 'macro', record });
        assert.equal((await app.store.get(record.id)).archived, true);
        await vscode.commands.executeCommand('spellbook.restoreMacro', {
          kind: 'macro',
          record: await app.store.get(record.id),
        });
        assert.equal((await app.store.get(record.id)).archived, false);
        assert.equal(vscode.window.activeTextEditor?.document.uri.scheme, 'spellbook');
      },
    );
    await check(
      'Atlas import creates native library documents without changing metadata',
      async () => {
        const docs = await app.importText(
          JSON.stringify({
            format: 'forever-atlas-macros',
            version: 1,
            build: 'wowf:test',
            macros: [
              {
                name: 'Imported',
                body: '/cast !Bear Form',
                class: 'Druid',
                notes: 'Original notes',
                favorite: true,
              },
            ],
          }),
        );
        assert.equal(docs.length, 1);
        assert.equal(docs[0].document.notes, 'Original notes');
        assert.equal(docs[0].document.favorite, true);
        assert.equal(
          (await vscode.workspace.openTextDocument(macroUri(docs[0]))).getText(),
          '/cast !Bear Form',
        );
      },
    );
    // The isolated test host exits automatically; do not prompt to save fixtures.
  } finally {
    if (process.env.SPELLBOOK_TEST_REPORT)
      await fs.writeFile(
        process.env.SPELLBOOK_TEST_REPORT,
        JSON.stringify({ vscode: vscode.version, tests: results }, null, 2),
      );
  }
}
