import * as vscode from 'vscode';
import { complete, diagnose, referenceAt, Issue, Suggestion } from './language';
import { normalize, chars, bytes, Name } from './model';

const selector: vscode.DocumentSelector = { language: 'wowmacro' };
export class LanguageFeatures implements vscode.Disposable {
  readonly diagnostics = vscode.languages.createDiagnosticCollection('spellbook');
  readonly status = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 110);
  private disposables: vscode.Disposable[] = [];
  constructor(
    private names: () => Name[],
    private nameFor: (doc: vscode.TextDocument) => string,
  ) {
    this.status.name = 'Spellbook macro budget';
    this.status.command = 'workbench.actions.view.problems';
    this.disposables.push(
      vscode.languages.registerCompletionItemProvider(
        selector,
        {
          provideCompletionItems: (doc, position) => {
            return complete(doc.getText(), doc.offsetAt(position), this.names()).map((s, index) => {
              const kinds: Record<Suggestion['kind'], vscode.CompletionItemKind> = {
                command: vscode.CompletionItemKind.Keyword,
                condition: vscode.CompletionItemKind.Property,
                unit: vscode.CompletionItemKind.Variable,
                value: vscode.CompletionItemKind.EnumMember,
                spell: vscode.CompletionItemKind.Function,
                item: vscode.CompletionItemKind.Value,
                lua: vscode.CompletionItemKind.Function,
              };
              const item = new vscode.CompletionItem(s.label, kinds[s.kind]);
              item.insertText = s.insert;
              item.range = new vscode.Range(doc.positionAt(s.start), doc.positionAt(s.end));
              item.sortText = String(index).padStart(4, '0');
              item.detail = 'Spellbook · ' + s.kind;
              item.documentation = new vscode.MarkdownString().appendText(s.detail);
              return item;
            });
          },
        },
        '/',
        '#',
        '[',
        ',',
        ':',
        '@',
        ';',
        '!',
        '.',
      ),
      vscode.languages.registerHoverProvider(selector, {
        provideHover: (doc, position) => {
          const found = referenceAt(doc.getText(), doc.offsetAt(position), this.names());
          if (!found) return;
          const markdown = new vscode.MarkdownString().appendText(
            found.title + '\n\n' + found.description,
          );
          if (found.example) markdown.appendCodeblock(found.example, 'wowmacro');
          return new vscode.Hover(markdown);
        },
      }),
      vscode.languages.registerCodeActionsProvider(
        selector,
        {
          provideCodeActions: (doc, range) => {
            return this.issues(doc)
              .filter(
                (issue) =>
                  issue.fix &&
                  new vscode.Range(
                    doc.positionAt(issue.start),
                    doc.positionAt(issue.end),
                  ).intersection(range),
              )
              .map((issue) => {
                const fix = issue.fix!,
                  action = new vscode.CodeAction(fix.title, vscode.CodeActionKind.QuickFix);
                action.edit = new vscode.WorkspaceEdit();
                action.edit.replace(
                  doc.uri,
                  new vscode.Range(doc.positionAt(fix.start), doc.positionAt(fix.end)),
                  fix.text,
                );
                action.diagnostics = this.diagnostics
                  .get(doc.uri)
                  ?.filter((d) => d.code === issue.code);
                return action;
              });
          },
        },
        { providedCodeActionKinds: [vscode.CodeActionKind.QuickFix] },
      ),
      vscode.languages.registerDocumentSymbolProvider(selector, {
        provideDocumentSymbols: (doc) => {
          const symbols: vscode.DocumentSymbol[] = [];
          for (let i = 0; i < doc.lineCount; i++) {
            const line = doc.lineAt(i),
              m = /^\s*([/#]\S+)(.*)/.exec(line.text);
            if (m) {
              const selection = new vscode.Range(
                i,
                line.text.indexOf(m[1]),
                i,
                line.text.indexOf(m[1]) + m[1].length,
              );
              symbols.push(
                new vscode.DocumentSymbol(
                  m[1],
                  m[2].trim(),
                  vscode.SymbolKind.Function,
                  line.range,
                  selection,
                ),
              );
            }
          }
          return symbols;
        },
      }),
      vscode.workspace.onDidOpenTextDocument((doc) => this.update(doc)),
      vscode.workspace.onDidChangeTextDocument((event) => this.update(event.document)),
      vscode.workspace.onDidCloseTextDocument((doc) => this.diagnostics.delete(doc.uri)),
      vscode.window.onDidChangeActiveTextEditor(() => this.updateStatus()),
      vscode.workspace.onDidChangeConfiguration((event) => {
        if (event.affectsConfiguration('spellbook')) this.refresh();
      }),
    );
    this.refresh();
  }
  issues(doc: vscode.TextDocument): Issue[] {
    return diagnose(
      this.nameFor(doc),
      doc.getText(),
      vscode.workspace.getConfiguration('spellbook', doc.uri).get<number>('characterLimit', 255),
    );
  }
  refresh(): void {
    for (const doc of vscode.workspace.textDocuments) this.update(doc);
    this.updateStatus();
  }
  update(doc: vscode.TextDocument): void {
    if (doc.languageId !== 'wowmacro') return;
    const enabled = vscode.workspace
      .getConfiguration('spellbook', doc.uri)
      .get('diagnostics', true);
    this.diagnostics.set(
      doc.uri,
      enabled
        ? this.issues(doc).map((issue) => {
            const d = new vscode.Diagnostic(
              new vscode.Range(doc.positionAt(issue.start), doc.positionAt(issue.end)),
              issue.message,
              issue.severity === 'error'
                ? vscode.DiagnosticSeverity.Error
                : vscode.DiagnosticSeverity.Warning,
            );
            d.source = 'Spellbook';
            d.code = issue.code;
            return d;
          })
        : [],
    );
    this.updateStatus();
  }
  updateStatus(): void {
    const doc = vscode.window.activeTextEditor?.document;
    if (!doc || doc.languageId !== 'wowmacro') {
      this.status.hide();
      return;
    }
    const text = normalize(doc.getText()),
      count = chars(text),
      limit = vscode.workspace
        .getConfiguration('spellbook', doc.uri)
        .get<number>('characterLimit', 255);
    this.status.text = `$(book) ${count}/${limit} chars`;
    this.status.tooltip = `Spellbook · ${bytes(text)} UTF-8 bytes\n${Math.max(0, limit - count)} characters remaining\nClick to open structural diagnostics.`;
    this.status.backgroundColor =
      count > limit ? new vscode.ThemeColor('statusBarItem.errorBackground') : undefined;
    this.status.show();
  }
  dispose(): void {
    for (const d of this.disposables) d.dispose();
    this.diagnostics.dispose();
    this.status.dispose();
  }
}
