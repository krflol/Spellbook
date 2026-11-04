import * as fs from 'node:fs/promises';
import * as path from 'node:path';
import { randomUUID } from 'node:crypto';
import { document, MacroDocument, object, bytes } from './model';

export interface Revision {
  number: number;
  at: string;
  document: MacroDocument;
}
export interface SavedMacro {
  id: string;
  revision: number;
  createdAt: string;
  updatedAt: string;
  archived: boolean;
  document: MacroDocument;
  history: Revision[];
}
interface State {
  format: 'spellbook-storage';
  version: 1;
  records: SavedMacro[];
}
const EMPTY = (): State => ({ format: 'spellbook-storage', version: 1, records: [] });
const STORE_LIMIT = 64 * 1024 * 1024;
const sleep = (ms: number): Promise<void> => new Promise((resolve) => setTimeout(resolve, ms));
export class ConflictError extends Error {}
function missing(error: unknown): boolean {
  return (error as NodeJS.ErrnoException).code === 'ENOENT';
}
export class LibraryStore {
  readonly file: string;
  constructor(readonly directory: string) {
    this.file = path.join(directory, 'library.json');
  }
  private async state(): Promise<State> {
    let text: string;
    try {
      const stat = await fs.stat(this.file);
      if (stat.size > STORE_LIMIT) throw new Error('Library exceeds the 64 MiB storage limit.');
      text = await fs.readFile(this.file, 'utf8');
    } catch (error) {
      if (missing(error)) return EMPTY();
      throw error;
    }
    const parsed: unknown = JSON.parse(text);
    object(parsed);
    if (
      parsed.format !== 'spellbook-storage' ||
      parsed.version !== 1 ||
      !Array.isArray(parsed.records)
    )
      throw new Error('Unsupported or damaged Spellbook library. The file has not been changed.');
    const ids = new Set<string>();
    const records = parsed.records.map((value: unknown): SavedMacro => {
      object(value);
      if (
        typeof value.id !== 'string' ||
        !/^[a-f0-9-]{36}$/.test(value.id) ||
        ids.has(value.id) ||
        !Number.isSafeInteger(value.revision) ||
        Number(value.revision) < 1 ||
        typeof value.archived !== 'boolean' ||
        typeof value.createdAt !== 'string' ||
        typeof value.updatedAt !== 'string' ||
        !Array.isArray(value.history)
      )
        throw new Error('Invalid library record. The file has not been changed.');
      ids.add(value.id);
      const history = value.history.map((r: unknown): Revision => {
        object(r);
        if (!Number.isSafeInteger(r.number) || typeof r.at !== 'string')
          throw new Error('Invalid library history.');
        return { number: Number(r.number), at: r.at, document: document(r.document) };
      });
      return {
        id: value.id,
        revision: Number(value.revision),
        createdAt: value.createdAt,
        updatedAt: value.updatedAt,
        archived: value.archived,
        document: document(value.document),
        history,
      };
    });
    return { format: 'spellbook-storage', version: 1, records };
  }
  async list(): Promise<SavedMacro[]> {
    return (await this.state()).records.sort((a, b) => b.updatedAt.localeCompare(a.updatedAt));
  }
  async get(id: string): Promise<SavedMacro> {
    const record = (await this.state()).records.find((r) => r.id === id);
    if (!record) throw new Error('Macro not found.');
    return record;
  }
  private async lock(): Promise<fs.FileHandle> {
    await fs.mkdir(this.directory, { recursive: true });
    const file = path.join(this.directory, 'library.lock'),
      deadline = Date.now() + 2500;
    while (true) {
      try {
        const handle = await fs.open(file, 'wx');
        try {
          await handle.writeFile(JSON.stringify({ pid: process.pid, at: Date.now() }));
          return handle;
        } catch (error) {
          await handle.close();
          await fs.unlink(file).catch(() => {});
          throw error;
        }
      } catch (error) {
        if ((error as NodeJS.ErrnoException).code !== 'EEXIST') throw error;
        // Only recover a stale lock whose recorded owner no longer exists.
        try {
          const owner = JSON.parse(await fs.readFile(file, 'utf8')) as { pid: number; at: number };
          if (Number.isInteger(owner.pid) && owner.pid > 0 && Date.now() - owner.at > 10000) {
            try {
              process.kill(owner.pid, 0);
            } catch (e) {
              if ((e as NodeJS.ErrnoException).code === 'ESRCH') {
                await fs.unlink(file);
                continue;
              }
            }
          }
        } catch {
          /* An active or partially written lock must not be stolen. */
        }
        if (Date.now() >= deadline)
          throw new Error(
            'Another Spellbook window is saving this library. Try again in a moment; your draft is retained.',
          );
        await sleep(50);
      }
    }
  }
  private async change<T>(mutate: (state: State) => T): Promise<T> {
    const handle = await this.lock(),
      temporary = this.file + '.' + randomUUID() + '.tmp';
    try {
      const state = await this.state(),
        result = mutate(state),
        text = JSON.stringify(state);
      if (bytes(text) > STORE_LIMIT)
        throw new Error(
          'Library storage would exceed 64 MiB. Export macros or reduce large notes; the current library is unchanged.',
        );
      const output = await fs.open(temporary, 'wx');
      try {
        await output.writeFile(text, 'utf8');
        await output.sync();
      } finally {
        await output.close();
      }
      await fs.rename(temporary, this.file);
      return result;
    } finally {
      await fs.unlink(temporary).catch(() => {});
      await handle.close();
      await fs.unlink(path.join(this.directory, 'library.lock')).catch(() => {});
    }
  }
  async createMany(values: MacroDocument[]): Promise<SavedMacro[]> {
    if (values.length > 2000) throw new Error('Import contains more than 2,000 macros.');
    const docs = values.map(document);
    return this.change((state) => {
      const at = new Date().toISOString();
      const records = docs.map((doc) => ({
        id: randomUUID(),
        revision: 1,
        createdAt: at,
        updatedAt: at,
        archived: false,
        document: doc,
        history: [{ number: 1, at, document: doc }],
      }));
      state.records.push(...records);
      return records;
    });
  }
  async update(id: string, value: MacroDocument, expected: number): Promise<SavedMacro> {
    const doc = document(value);
    return this.change((state) => {
      const record = state.records.find((r) => r.id === id);
      if (!record || record.archived)
        throw new ConflictError(
          'This macro was removed or archived. Save As or duplicate your draft to retain it.',
        );
      if (record.revision !== expected)
        throw new ConflictError(
          'This macro changed in another window. Your draft is retained. Compare/reload the saved document, or Save As to keep both versions.',
        );
      if (JSON.stringify(record.document) === JSON.stringify(doc)) return record;
      record.document = doc;
      record.revision++;
      record.updatedAt = new Date().toISOString();
      record.history.push({ number: record.revision, at: record.updatedAt, document: doc });
      record.history = record.history.slice(-100);
      return record;
    });
  }
  async archive(id: string, archived: boolean, expected: number): Promise<SavedMacro> {
    return this.change((state) => {
      const record = state.records.find((r) => r.id === id);
      if (!record || record.revision !== expected)
        throw new ConflictError('This macro changed in another window. Refresh the library first.');
      record.archived = archived;
      record.revision++;
      record.updatedAt = new Date().toISOString();
      return record;
    });
  }
  async restore(id: string, number: number, expected: number): Promise<SavedMacro> {
    const record = await this.get(id),
      revision = record.history.find((r) => r.number === number);
    if (!revision) throw new Error('Saved revision not found.');
    return this.update(id, revision.document, expected);
  }
}
