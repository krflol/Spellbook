import * as vscode from 'vscode';
import { LibraryStore, SavedMacro } from './store';
import { document, normalize } from './model';

export const macroUri = (record: SavedMacro): vscode.Uri =>
  vscode.Uri.from({
    scheme: 'spellbook',
    path: `/${record.id}/${record.document.name.replace(/[\\/\x00-\x1f]/g, '_') || 'Untitled macro'}.wowmacro`,
  });
export const macroId = (uri: vscode.Uri): string | undefined =>
  uri.scheme === 'spellbook' ? uri.path.split('/')[1] : undefined;
export const notesUri = (record: SavedMacro): vscode.Uri =>
  vscode.Uri.from({ scheme: 'spellbook', path: `/${record.id}/notes.md` });
const contents = (uri: vscode.Uri, record: SavedMacro): string =>
  uri.path.endsWith('/notes.md') ? record.document.notes : record.document.body;
export class LibraryFileSystem implements vscode.FileSystemProvider, vscode.Disposable {
  private emitter = new vscode.EventEmitter<vscode.FileChangeEvent[]>();
  readonly onDidChangeFile = this.emitter.event;
  private expected = new Map<string, number>();
  private known = new Map<string, SavedMacro>();
  constructor(
    readonly store: LibraryStore,
    private changed: () => Promise<void>,
  ) {}
  dispose(): void {
    this.emitter.dispose();
  }
  watch(): vscode.Disposable {
    return new vscode.Disposable(() => {});
  }
  async stat(uri: vscode.Uri): Promise<vscode.FileStat> {
    if (uri.path === '/') return { type: vscode.FileType.Directory, ctime: 0, mtime: 0, size: 0 };
    const record = await this.record(uri);
    if (uri.path.split('/').filter(Boolean).length === 1)
      return {
        type: vscode.FileType.Directory,
        ctime: Date.parse(record.createdAt),
        mtime: Date.parse(record.updatedAt),
        size: 0,
      };
    return {
      type: vscode.FileType.File,
      ctime: Date.parse(record.createdAt),
      mtime: Date.parse(record.updatedAt),
      size: Buffer.byteLength(contents(uri, record)),
    };
  }
  async readFile(uri: vscode.Uri): Promise<Uint8Array> {
    const record = await this.record(uri);
    this.expected.set(uri.toString(), record.revision);
    return Buffer.from(contents(uri, record), 'utf8');
  }
  async writeFile(
    uri: vscode.Uri,
    content: Uint8Array,
    _options: { create: boolean; overwrite: boolean },
  ): Promise<void> {
    const record = await this.record(uri),
      expected = this.expected.get(uri.toString());
    if (expected === undefined)
      throw vscode.FileSystemError.NoPermissions(
        'Open this library macro before editing it. Use Spellbook: New Macro to create a document.',
      );
    try {
      const field = uri.path.endsWith('/notes.md') ? 'notes' : 'body';
      const saved = await this.store.update(
        record.id,
        document({ ...record.document, [field]: normalize(Buffer.from(content).toString('utf8')) }),
        expected,
      );
      this.expected.set(uri.toString(), saved.revision);
      await this.changed();
    } catch (error) {
      throw vscode.FileSystemError.Unavailable(
        error instanceof Error ? error.message : String(error),
      );
    }
  }
  async readDirectory(uri: vscode.Uri): Promise<[string, vscode.FileType][]> {
    if (uri.path === '/')
      return (await this.store.list())
        .filter((r) => !r.archived)
        .map((r) => [r.id, vscode.FileType.Directory]);
    const r = await this.record(uri);
    return [
      [macroUri(r).path.split('/').pop()!, vscode.FileType.File],
      ['notes.md', vscode.FileType.File],
    ];
  }
  createDirectory(): void {
    throw vscode.FileSystemError.NoPermissions('Use Spellbook library commands.');
  }
  delete(): void {
    throw vscode.FileSystemError.NoPermissions('Use Spellbook: Move Macro to Trash.');
  }
  rename(): void {
    throw vscode.FileSystemError.NoPermissions(
      'Use Spellbook: Edit Macro Details to rename a macro.',
    );
  }
  async record(uri: vscode.Uri): Promise<SavedMacro> {
    const id = macroId(uri);
    if (!id) throw vscode.FileSystemError.FileNotFound(uri);
    try {
      const record = await this.store.get(id);
      if (record.archived) throw new Error('Archived');
      return record;
    } catch {
      throw vscode.FileSystemError.FileNotFound(uri);
    }
  }
  synchronize(records: SavedMacro[]): void {
    const next = new Map(records.map((r) => [r.id, r]));
    const events: vscode.FileChangeEvent[] = [];
    for (const record of records) {
      const prior = this.known.get(record.id);
      if (prior && prior.revision !== record.revision)
        for (const uri of [macroUri(prior), notesUri(prior)])
          events.push({
            type: record.archived ? vscode.FileChangeType.Deleted : vscode.FileChangeType.Changed,
            uri,
          });
    }
    this.known = next;
    if (events.length) this.emitter.fire(events);
  }
  setExpected(uri: vscode.Uri, revision: number): void {
    this.expected.set(uri.toString(), revision);
  }
}
export class RevisionContent implements vscode.TextDocumentContentProvider {
  constructor(private store: LibraryStore) {}
  async provideTextDocumentContent(uri: vscode.Uri): Promise<string> {
    const [, id, revision] = uri.path.split('/'),
      record = await this.store.get(id),
      saved = record.history.find((r) => r.number === Number(revision));
    if (!saved) throw new Error('Revision is no longer retained.');
    return saved.document.body;
  }
}
