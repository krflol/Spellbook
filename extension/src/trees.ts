import * as vscode from 'vscode';
import { refs, Sample } from './model';
import { SavedMacro, Revision } from './store';
import { macroUri } from './library-fs';

export type Node =
  | { kind: 'macro'; record: SavedMacro }
  | { kind: 'group'; label: string }
  | { kind: 'sample'; sample: Sample }
  | { kind: 'revision'; record: SavedMacro; revision: Revision };
export class LibraryTree implements vscode.TreeDataProvider<Node>, vscode.Disposable {
  private emitter = new vscode.EventEmitter<Node | undefined>();
  readonly onDidChangeTreeData = this.emitter.event;
  records: SavedMacro[] = [];
  query = '';
  classFilter = 'All classes';
  refresh(): void {
    this.emitter.fire(undefined);
  }
  dispose(): void {
    this.emitter.dispose();
  }
  getChildren(node?: Node): Node[] {
    const filtered = this.records.filter(
      (r) =>
        (this.classFilter === 'All classes' || r.document.class === this.classFilter) &&
        Object.values(r.document).join(' ').toLowerCase().includes(this.query.toLowerCase()),
    );
    if (!node) {
      const active = filtered.filter((r) => !r.archived);
      const favorites = active.filter((r) => r.document.favorite),
        others = active.filter((r) => !r.document.favorite);
      return [...favorites, ...others]
        .map((record) => ({ kind: 'macro', record }) as Node)
        .concat(filtered.some((r) => r.archived) ? [{ kind: 'group', label: 'Trash' }] : []);
    }
    if (node.kind === 'group')
      return filtered.filter((r) => r.archived).map((record) => ({ kind: 'macro', record }));
    return [];
  }
  getTreeItem(node: Node): vscode.TreeItem {
    if (node.kind === 'group') {
      const item = new vscode.TreeItem(node.label, vscode.TreeItemCollapsibleState.Collapsed);
      item.iconPath = new vscode.ThemeIcon('trash');
      item.id = 'trash';
      return item;
    }
    if (node.kind !== 'macro') throw new Error('Unexpected library item.');
    const r = node.record,
      item = new vscode.TreeItem(r.document.name || 'Untitled macro');
    item.id = r.id;
    item.description =
      r.document.class + (r.document.character ? ' · ' + r.document.character : '');
    item.contextValue = r.archived ? 'archived' : 'macro';
    item.iconPath = new vscode.ThemeIcon(
      r.archived ? 'trash' : r.document.favorite ? 'star-full' : 'symbol-method',
    );
    const tooltip = new vscode.MarkdownString();
    tooltip.appendText(
      r.document.name +
        '\n' +
        r.document.class +
        '\n' +
        r.document.tags +
        '\n\n' +
        r.document.notes,
    );
    tooltip.appendCodeblock(r.document.body, 'wowmacro');
    item.tooltip = tooltip;
    if (!r.archived) {
      item.resourceUri = macroUri(r);
      item.command = { command: 'spellbook.openMacro', title: 'Open macro', arguments: [node] };
    }
    return item;
  }
}
export class SampleTree implements vscode.TreeDataProvider<Node>, vscode.Disposable {
  private emitter = new vscode.EventEmitter<Node | undefined>();
  readonly onDidChangeTreeData = this.emitter.event;
  classFilter = 'All classes';
  dispose(): void {
    this.emitter.dispose();
  }
  refresh(): void {
    this.emitter.fire(undefined);
  }
  getChildren(node?: Node): Node[] {
    const samples = refs.samples.filter(
      (s) =>
        this.classFilter === 'All classes' ||
        s.class === this.classFilter ||
        s.class === 'Any class',
    );
    if (!node)
      return [...new Set(samples.map((s) => s.category))].map((label) => ({
        kind: 'group',
        label,
      }));
    return node.kind === 'group'
      ? samples
          .filter((s) => s.category === node.label)
          .map((sample) => ({ kind: 'sample', sample }))
      : [];
  }
  getTreeItem(node: Node): vscode.TreeItem {
    if (node.kind === 'group') {
      const item = new vscode.TreeItem(node.label, vscode.TreeItemCollapsibleState.Collapsed);
      item.id = 'samples:' + node.label;
      item.iconPath = new vscode.ThemeIcon('folder-library');
      return item;
    }
    if (node.kind !== 'sample') throw new Error('Unexpected sample item.');
    const s = node.sample,
      item = new vscode.TreeItem(s.name);
    item.id = 'sample:' + s.name;
    item.description = s.class;
    item.iconPath = new vscode.ThemeIcon('symbol-snippet');
    item.contextValue = 'sample';
    item.command = {
      command: 'spellbook.previewSample',
      title: 'Preview sample',
      arguments: [node],
    };
    item.tooltip = new vscode.MarkdownString()
      .appendText(s.notes)
      .appendCodeblock(s.body, 'wowmacro');
    return item;
  }
}
export class HistoryTree implements vscode.TreeDataProvider<Node>, vscode.Disposable {
  private emitter = new vscode.EventEmitter<Node | undefined>();
  readonly onDidChangeTreeData = this.emitter.event;
  record?: SavedMacro;
  dispose(): void {
    this.emitter.dispose();
  }
  refresh(record?: SavedMacro): void {
    this.record = record;
    this.emitter.fire(undefined);
  }
  getChildren(): Node[] {
    return this.record
      ? [...this.record.history]
          .reverse()
          .map((revision) => ({ kind: 'revision', record: this.record!, revision }))
      : [];
  }
  getTreeItem(node: Node): vscode.TreeItem {
    if (node.kind !== 'revision') throw new Error('Unexpected history item.');
    const item = new vscode.TreeItem('Revision ' + node.revision.number);
    item.id = node.record.id + ':' + node.revision.number;
    item.description = new Date(node.revision.at).toLocaleString();
    item.iconPath = new vscode.ThemeIcon('history');
    item.contextValue = 'revision';
    item.tooltip = node.revision.document.name + '\n' + node.revision.at;
    item.command = {
      command: 'spellbook.compareRevision',
      title: 'Compare with current',
      arguments: [node],
    };
    return item;
  }
}
