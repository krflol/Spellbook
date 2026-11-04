import * as vscode from 'vscode';
import * as fs from 'node:fs/promises';
import { watch } from 'node:fs';
import * as path from 'node:path';
import { homedir } from 'node:os';
import { LibraryStore, SavedMacro } from './store';
import { LibraryFileSystem, RevisionContent, macroId, macroUri, notesUri } from './library-fs';
import { LibraryTree, SampleTree, HistoryTree, Node } from './trees';
import { LanguageFeatures } from './providers';
import {
  document,
  MacroDocument,
  refs,
  Sample,
  Name,
  fromSample,
  decodeLibrary,
  encodeLibrary,
  decodeCatalogue,
  mergeNames,
  normalize,
  IMPORT_LIMIT,
  CLASSES,
} from './model';

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
function selectedRecord(value: unknown): SavedMacro | undefined {
  if (value && typeof value === 'object' && 'kind' in value && (value as Node).kind === 'macro')
    return (value as Extract<Node, { kind: 'macro' }>).record;
  return undefined;
}
function selectedSample(value: unknown): Sample | undefined {
  if (value && typeof value === 'object' && 'kind' in value && (value as Node).kind === 'sample')
    return (value as Extract<Node, { kind: 'sample' }>).sample;
  return undefined;
}
async function readText(uri: vscode.Uri): Promise<string> {
  const stat = await vscode.workspace.fs.stat(uri);
  if (stat.size > IMPORT_LIMIT) throw new Error('File exceeds 8 MiB.');
  const bytes = await vscode.workspace.fs.readFile(uri);
  if (bytes.length > IMPORT_LIMIT) throw new Error('File exceeds 8 MiB.');
  return new TextDecoder('utf-8', { fatal: true }).decode(bytes);
}

export class Spellbook implements vscode.Disposable {
  readonly store: LibraryStore;
  readonly library = new LibraryTree();
  readonly samples = new SampleTree();
  readonly history = new HistoryTree();
  readonly filesystem: LibraryFileSystem;
  readonly features: LanguageFeatures;
  private names: Name[] = mergeNames([]);
  private disposables: vscode.Disposable[] = [];
  private autosaves = new Map<string, ReturnType<typeof setTimeout>>();
  private refreshTimer?: ReturnType<typeof setTimeout>;
  private output = vscode.window.createOutputChannel('Spellbook');
  private historyId?: string;
  private catalogueVersion = 0;
  constructor(readonly context: vscode.ExtensionContext) {
    this.store = new LibraryStore(context.globalStorageUri.fsPath);
    this.filesystem = new LibraryFileSystem(this.store, () => this.refresh());
    this.disposables.push(
      this.output,
      this.library,
      this.samples,
      this.history,
      this.filesystem,
      vscode.workspace.registerFileSystemProvider('spellbook', this.filesystem, {
        isCaseSensitive: true,
      }),
      vscode.workspace.registerTextDocumentContentProvider(
        'spellbook-revision',
        new RevisionContent(this.store),
      ),
      vscode.workspace.registerTextDocumentContentProvider('spellbook-sample', {
        provideTextDocumentContent: (uri) => {
          const sample = refs.samples[Number(uri.path.split('/')[1])];
          if (!sample) throw new Error('Sample not found.');
          return (
            '# ' +
            sample.name +
            '\n\n' +
            sample.class +
            ' · ' +
            sample.category +
            '\n\n' +
            sample.notes +
            '\n\n```wowmacro\n' +
            sample.body +
            '\n```\n\nUse **Spellbook: New Macro from Sample** to create an editable copy.\n'
          );
        },
      }),
      vscode.window.createTreeView('spellbook.library', {
        treeDataProvider: this.library,
        showCollapseAll: true,
      }),
      vscode.window.createTreeView('spellbook.samples', {
        treeDataProvider: this.samples,
        showCollapseAll: true,
      }),
      vscode.window.createTreeView('spellbook.history', {
        treeDataProvider: this.history,
        showCollapseAll: false,
      }),
    );
    this.features = new LanguageFeatures(
      () => this.names,
      (doc) =>
        this.library.records.find((r) => r.id === macroId(doc.uri))?.document.name ??
        path.basename(doc.fileName).replace(/\.(?:wowmacro|macro)$/i, ''),
    );
    this.disposables.push(this.features);
    this.commands();
    this.disposables.push(
      vscode.workspace.onDidChangeTextDocument((event) => {
        if (event.document.uri.scheme !== 'spellbook' || !event.contentChanges.length) return;
        const key = event.document.uri.toString(),
          pending = this.autosaves.get(key);
        if (pending) clearTimeout(pending);
        if (vscode.workspace.getConfiguration('spellbook').get('autoSaveLibrary', true))
          this.autosaves.set(
            key,
            setTimeout(() => {
              this.autosaves.delete(key);
              if (!event.document.isClosed && event.document.isDirty)
                void event.document.save().then(undefined, (e) => this.report(e));
            }, 1000),
          );
      }),
      vscode.workspace.onDidCloseTextDocument((doc) => {
        const pending = this.autosaves.get(doc.uri.toString());
        if (pending) clearTimeout(pending);
        this.autosaves.delete(doc.uri.toString());
      }),
      vscode.window.onDidChangeActiveTextEditor((editor) => {
        const id = editor && macroId(editor.document.uri);
        if (id) {
          this.historyId = id;
          this.history.refresh(this.library.records.find((r) => r.id === id));
        }
      }),
      vscode.workspace.onDidChangeConfiguration((event) => {
        if (event.affectsConfiguration('spellbook.cataloguePath'))
          void this.loadCatalogue().catch((e) => this.report(e));
        if (
          event.affectsConfiguration('spellbook.autoSaveLibrary') &&
          !vscode.workspace.getConfiguration('spellbook').get('autoSaveLibrary', true)
        ) {
          for (const timer of this.autosaves.values()) clearTimeout(timer);
          this.autosaves.clear();
        }
      }),
    );
  }
  async initialize(): Promise<void> {
    await fs.mkdir(this.context.globalStorageUri.fsPath, { recursive: true });
    await this.refresh();
    await this.loadCatalogue().catch((e) => this.report(e));
    const watcher = watch(
      this.context.globalStorageUri.fsPath,
      { persistent: false },
      (_event, name) => {
        if (name?.toString() !== 'library.json') return;
        if (this.refreshTimer) clearTimeout(this.refreshTimer);
        this.refreshTimer = setTimeout(() => void this.refresh().catch((e) => this.report(e)), 120);
      },
    );
    watcher.on('error', (error) => this.report(error));
    this.disposables.push(new vscode.Disposable(() => watcher.close()));
  }
  report(error: unknown): void {
    this.output.appendLine(message(error));
    void vscode.window.showErrorMessage('Spellbook: ' + message(error));
  }
  async refresh(): Promise<void> {
    this.library.records = await this.store.list();
    this.library.refresh();
    this.filesystem.synchronize(this.library.records);
    const active = vscode.window.activeTextEditor?.document.uri;
    this.historyId = (active && macroId(active)) ?? this.historyId;
    this.history.refresh(this.library.records.find((r) => r.id === this.historyId));
    this.features.refresh();
  }
  async loadCatalogue(): Promise<void> {
    const version = ++this.catalogueVersion,
      configured = vscode.workspace
        .getConfiguration('spellbook')
        .get<string>('cataloguePath', '')
        .trim();
    let custom: Name[] = [];
    if (configured) {
      if (!path.isAbsolute(configured))
        throw new Error('Autocomplete catalogue path must be absolute.');
      custom = decodeCatalogue(await readText(vscode.Uri.file(configured)));
    } else {
      const file = vscode.Uri.joinPath(this.context.globalStorageUri, 'catalogue.json');
      try {
        custom = decodeCatalogue(await readText(file));
      } catch (error) {
        if (
          (error as { code?: string }).code !== 'FileNotFound' &&
          (error as { code?: string }).code !== 'ENOENT'
        )
          throw error;
      }
    }
    if (version === this.catalogueVersion) {
      this.names = mergeNames(custom);
      this.features.refresh();
    }
  }
  async open(record: SavedMacro): Promise<vscode.TextEditor> {
    return vscode.window.showTextDocument(
      await vscode.workspace.openTextDocument(macroUri(record)),
      { preview: false },
    );
  }
  async create(doc: MacroDocument): Promise<SavedMacro> {
    const [saved] = await this.store.createMany([doc]);
    await this.refresh();
    await this.open(saved);
    return saved;
  }
  async saveOpen(id?: string): Promise<boolean> {
    for (const doc of vscode.workspace.textDocuments.filter(
      (d) => d.uri.scheme === 'spellbook' && d.isDirty && (!id || macroId(d.uri) === id),
    ))
      if (!(await doc.save())) return false;
    return true;
  }
  async current(value?: unknown): Promise<SavedMacro | undefined> {
    const selected = selectedRecord(value),
      uri = value instanceof vscode.Uri ? value : vscode.window.activeTextEditor?.document.uri,
      id = selected?.id ?? (uri && macroId(uri));
    if (id) return this.store.get(id);
    const picked = await vscode.window.showQuickPick(
      this.library.records
        .filter((r) => !r.archived)
        .map((record) => ({
          label: record.document.name,
          description: record.document.class,
          record,
        })),
      { title: 'Choose a library macro', matchOnDescription: true },
    );
    return picked?.record;
  }
  async pickSample(value?: unknown): Promise<Sample | undefined> {
    const selected = selectedSample(value);
    if (selected) return selected;
    const picked = await vscode.window.showQuickPick(
      refs.samples
        .filter(
          (s) =>
            this.samples.classFilter === 'All classes' ||
            s.class === this.samples.classFilter ||
            s.class === 'Any class',
        )
        .map((sample) => ({
          label: sample.name,
          description: sample.class + ' · ' + sample.category,
          detail: sample.description,
          sample,
        })),
      { title: 'Spellbook samples', matchOnDescription: true, matchOnDetail: true },
    );
    return picked?.sample;
  }
  private register(id: string, callback: (...args: unknown[]) => unknown): void {
    this.disposables.push(
      vscode.commands.registerCommand('spellbook.' + id, async (...args: unknown[]) => {
        try {
          return await callback(...args);
        } catch (error) {
          this.report(error);
          return undefined;
        }
      }),
    );
  }
  private commands(): void {
    this.register('newMacro', async () => {
      const name = await vscode.window.showInputBox({
        title: 'New WoW macro',
        prompt: 'Macro name (16 characters recommended)',
        value: 'Untitled macro',
        validateInput: (v) =>
          !v.trim()
            ? 'Enter a name.'
            : Buffer.byteLength(v) > 512
              ? 'Name is too long.'
              : undefined,
      });
      if (name !== undefined) return this.create(document({ name }));
    });
    this.register('openMacro', async (value) => {
      const r = await this.current(value);
      if (r) return this.open(r);
    });
    this.register('createFromSample', async (value) => {
      const sample = await this.pickSample(value);
      if (sample) return this.create(fromSample(sample));
    });
    this.register('previewSample', async (value) => {
      const sample = await this.pickSample(value);
      if (!sample) return;
      const uri = vscode.Uri.from({
        scheme: 'spellbook-sample',
        path: `/${refs.samples.indexOf(sample)}/sample.md`,
      });
      return vscode.commands.executeCommand('markdown.showPreview', uri);
    });
    this.register('insertSample', async (value) => {
      const editor = vscode.window.activeTextEditor;
      if (!editor || editor.document.languageId !== 'wowmacro')
        throw new Error('Open a WoW Macro editor before inserting a sample.');
      const sample = await this.pickSample(value);
      if (sample) await editor.edit((edit) => edit.replace(editor.selection, sample.body));
    });
    this.register('filterLibrary', async () => {
      const value = await vscode.window.showQuickPick(['All classes', ...CLASSES], {
        title: 'Filter macro library',
      });
      if (value) {
        this.library.classFilter = value;
        this.library.refresh();
      }
    });
    this.register('filterSamples', async () => {
      const value = await vscode.window.showQuickPick(['All classes', ...CLASSES], {
        title: 'Filter sample library',
      });
      if (value) {
        this.samples.classFilter = value;
        this.samples.refresh();
      }
    });
    this.register('searchLibrary', async () => {
      const value = await vscode.window.showInputBox({
        title: 'Search macro library',
        prompt: 'Search names, code, tags, notes, or character. Clear to show all.',
        value: this.library.query,
      });
      if (value !== undefined) {
        this.library.query = value;
        this.library.refresh();
      }
    });
    this.register('copyMacro', async (value) => {
      const selected = selectedRecord(value),
        editor = vscode.window.activeTextEditor;
      let body: string | undefined;
      if (selected) {
        const open = vscode.workspace.textDocuments.find(
          (d) => macroId(d.uri) === selected.id && d.languageId === 'wowmacro',
        );
        body = open?.getText() ?? (await this.store.get(selected.id)).document.body;
      } else if (editor?.document.languageId === 'wowmacro') body = editor.document.getText();
      if (body === undefined) throw new Error('Open a WoW macro or select one in the library.');
      await vscode.env.clipboard.writeText(normalize(body));
      vscode.window.setStatusBarMessage('Spellbook: Macro copied. Paste it into WoW.', 3000);
      return body;
    });
    this.register('addToLibrary', async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor || editor.document.languageId !== 'wowmacro')
        throw new Error('Open a WoW Macro file first.');
      const name = await vscode.window.showInputBox({
        title: 'Save macro to library',
        value: path.basename(editor.document.fileName).replace(/\.(?:wowmacro|macro)$/i, ''),
      });
      if (name !== undefined)
        return this.create(document({ name, body: editor.document.getText() }));
    });
    this.register('importLibrary', async () => {
      const files = await vscode.window.showOpenDialog({
        title: 'Import macros or a Spellbook / Atlas library',
        canSelectMany: true,
        filters: { Macros: ['json', 'wowmacro', 'macro', 'txt', 'lua'], 'All files': ['*'] },
      });
      if (!files) return;
      let count = 0;
      for (const file of files) {
        count += (
          await this.importText(
            await readText(file),
            path.basename(file.path).replace(/\.[^.]+$/, ''),
          )
        ).length;
      }
      void vscode.window.showInformationMessage(`Spellbook: Imported ${count} macros.`);
    });
    this.register('exportLibrary', async () => {
      if (!(await this.saveOpen())) return;
      const content = encodeLibrary(
        (await this.store.list()).filter((r) => !r.archived).map((r) => r.document),
      );
      return this.saveExport('Spellbook-library.json', content, { JSON: ['json'] });
    });
    this.register('exportMacro', async (value) => {
      const r = await this.current(value);
      if (!r || !(await this.saveOpen(r.id))) return;
      const record = await this.store.get(r.id);
      return this.saveExport(
        record.document.name.replace(/[<>:"/\\|?*]/g, '_') + '.wowmacro',
        record.document.body,
        { 'WoW macro': ['wowmacro'], 'Plain text': ['txt'] },
      );
    });
    this.register('toggleFavorite', async (value) => {
      const r = await this.current(value);
      if (!r || !(await this.saveOpen(r.id))) return;
      const current = await this.store.get(r.id);
      await this.store.update(
        r.id,
        { ...current.document, favorite: !current.document.favorite },
        current.revision,
      );
      await this.refresh();
    });
    this.register('editDetails', (value) => this.editDetails(value));
    this.register('duplicateMacro', async (value) => {
      const r = await this.current(value);
      if (!r) return;
      const editor = vscode.workspace.textDocuments.find(
        (d) => macroId(d.uri) === r.id && d.languageId === 'wowmacro',
      );
      return this.create({
        ...r.document,
        name: r.document.name + ' copy',
        body: editor?.getText() ?? r.document.body,
      });
    });
    this.register('archiveMacro', async (value) => {
      const r = await this.current(value);
      if (!r || !(await this.saveOpen(r.id))) return;
      const current = await this.store.get(r.id);
      await this.store.archive(r.id, true, current.revision);
      await this.closeTabs(r.id);
      await this.refresh();
    });
    this.register('restoreMacro', async (value) => {
      const r = await this.current(value);
      if (!r) return;
      const saved = await this.store.archive(r.id, false, r.revision);
      await this.refresh();
      await this.open(saved);
    });
    this.register('showHistory', async (value) => {
      const r = await this.current(value);
      if (!r) return;
      this.historyId = r.id;
      this.history.refresh(r);
      await vscode.commands.executeCommand('spellbook.history.focus');
    });
    this.register('compareRevision', async (value) => {
      if (!value || typeof value !== 'object' || !('kind' in value) || value.kind !== 'revision')
        return;
      const node = value as Extract<Node, { kind: 'revision' }>;
      const old = vscode.Uri.from({
        scheme: 'spellbook-revision',
        path: `/${node.record.id}/${node.revision.number}/revision.wowmacro`,
      });
      await vscode.commands.executeCommand(
        'vscode.diff',
        old,
        macroUri(await this.store.get(node.record.id)),
        `${node.record.document.name}: revision ${node.revision.number} ↔ current`,
      );
    });
    this.register('restoreRevision', async (value) => {
      if (!value || typeof value !== 'object' || !('kind' in value) || value.kind !== 'revision')
        return;
      const node = value as Extract<Node, { kind: 'revision' }>;
      if (!(await this.saveOpen(node.record.id))) return;
      const current = await this.store.get(node.record.id);
      const restored = await this.store.restore(current.id, node.revision.number, current.revision);
      await this.refresh();
      await this.open(restored);
    });
    this.register('reference', async () =>
      vscode.commands.executeCommand(
        'markdown.showPreview',
        vscode.Uri.joinPath(this.context.extensionUri, 'docs', 'REFERENCE.md'),
      ),
    );
    this.register('importCatalogue', async () => {
      const files = await vscode.window.showOpenDialog({
        title: 'Import autocomplete catalogue',
        filters: { JSON: ['json'] },
      });
      if (!files?.length) return;
      const names = decodeCatalogue(await readText(files[0]));
      const target = vscode.Uri.joinPath(this.context.globalStorageUri, 'catalogue.json');
      await this.atomicWrite(
        target,
        JSON.stringify({ format: 'spellbook-catalogue', version: 1, names }, null, 2),
      );
      await vscode.workspace
        .getConfiguration('spellbook')
        .update('cataloguePath', '', vscode.ConfigurationTarget.Global);
      await this.loadCatalogue();
      void vscode.window.showInformationMessage(
        `Spellbook: ${names.length} custom names merged with the bundled vocabulary.`,
      );
    });
    this.register('resetCatalogue', async () => {
      await fs
        .unlink(path.join(this.context.globalStorageUri.fsPath, 'catalogue.json'))
        .catch((error) => {
          if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error;
        });
      await vscode.workspace
        .getConfiguration('spellbook')
        .update('cataloguePath', '', vscode.ConfigurationTarget.Global);
      await this.loadCatalogue();
    });
    this.register('refresh', () => this.refresh());
  }
  async importText(text: string, name = 'Imported macro'): Promise<SavedMacro[]> {
    const records = await this.store.createMany(decodeLibrary(text, name));
    this.library.query = '';
    this.library.classFilter = 'All classes';
    await this.refresh();
    if (records[0]) await this.open(records[0]);
    return records;
  }
  private async saveExport(
    name: string,
    content: string,
    filters: Record<string, string[]>,
  ): Promise<void> {
    const folder = vscode.workspace.workspaceFolders?.[0]?.uri ?? vscode.Uri.file(homedir());
    const uri = await vscode.window.showSaveDialog({
      title: 'Export macro',
      defaultUri: vscode.Uri.joinPath(folder, name),
      filters,
    });
    if (!uri) return;
    // showSaveDialog owns the native overwrite confirmation for an existing file.
    await vscode.workspace.fs.writeFile(uri, Buffer.from(content, 'utf8'));
    vscode.window.setStatusBarMessage('Spellbook: Exported ' + uri.fsPath, 4000);
  }
  private async atomicWrite(uri: vscode.Uri, content: string): Promise<void> {
    const temp = uri.fsPath + '.' + process.pid + '.tmp';
    try {
      await fs.writeFile(temp, content, 'utf8');
      await fs.rename(temp, uri.fsPath);
    } finally {
      await fs.unlink(temp).catch(() => {});
    }
  }
  private async closeTabs(id: string): Promise<void> {
    const tabs = vscode.window.tabGroups.all
      .flatMap((g) => g.tabs)
      .filter((t) => t.input instanceof vscode.TabInputText && macroId(t.input.uri) === id);
    if (tabs.length) await vscode.window.tabGroups.close(tabs, true);
  }
  private async editDetails(value?: unknown): Promise<void> {
    const r = await this.current(value);
    if (!r || !(await this.saveOpen(r.id))) return;
    const field = await vscode.window.showQuickPick(
      ['Name', 'Class', 'Character', 'Tags', 'Notes'],
      { title: 'Edit ' + r.document.name },
    );
    if (!field) return;
    if (field === 'Notes') {
      await vscode.window.showTextDocument(await vscode.workspace.openTextDocument(notesUri(r)), {
        preview: false,
      });
      return;
    }
    const current = await this.store.get(r.id),
      key = field.toLowerCase() as 'name' | 'class' | 'character' | 'tags';
    const text =
      field === 'Class'
        ? await vscode.window.showQuickPick(CLASSES, { title: 'Macro class' })
        : await vscode.window.showInputBox({
            title: 'Macro ' + field.toLowerCase(),
            value: current.document[key],
            prompt:
              field === 'Character'
                ? 'Blank means account-wide.'
                : field === 'Name'
                  ? '16 characters recommended for the game.'
                  : undefined,
          });
    if (text === undefined) return;
    const saved = await this.store.update(
      r.id,
      document({ ...current.document, [key]: text }),
      current.revision,
    );
    await this.refresh();
    if (field === 'Name') {
      await this.closeTabs(r.id);
      await this.open(saved);
    }
  }
  async flush(): Promise<void> {
    if (vscode.workspace.getConfiguration('spellbook').get('autoSaveLibrary', true))
      await this.saveOpen();
  }
  dispose(): void {
    if (this.refreshTimer) clearTimeout(this.refreshTimer);
    for (const timer of this.autosaves.values()) clearTimeout(timer);
    for (const d of this.disposables) d.dispose();
  }
}
let active: Spellbook | undefined;
export async function activate(context: vscode.ExtensionContext): Promise<Spellbook> {
  active = new Spellbook(context);
  context.subscriptions.push(active);
  await active.initialize();
  return active;
}
export async function deactivate(): Promise<void> {
  await active?.flush();
}
