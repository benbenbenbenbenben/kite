import * as fs from 'node:fs';
import * as path from 'node:path';
import * as vscode from 'vscode';
import {
  Executable,
  LanguageClient,
  LanguageClientOptions,
  ServerOptions
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;

type Association = {
  kiteSpec: string;
  kiteFile: string;
  kiteSpan: any;
  targetPath: string;
  sourceSpan?: any;
  status: 'pass' | 'fail' | 'warning';
  message?: string;
};

let allAssociationsByFile = new Map<string, Association[]>();
let cachedRawBindings: any[] = [];

function buildAssociationsFromBindings(bindings: any[]): Map<string, Association[]> {
  const result = new Map<string, Association[]>();
  for (const binding of bindings) {
    let status: 'pass' | 'fail' | 'warning' = 'pass';
    let message = `Kite Specification: ${binding.kite_spec}`;

    if (binding.kite_file) {
      const kiteUri = vscode.Uri.file(binding.kite_file);
      const diagnostics = vscode.languages.getDiagnostics(kiteUri);
      const kiteSpanLine = (binding.kite_span?.start_line ?? 1) - 1;

      for (const diag of diagnostics) {
        if (diag.source !== 'kite') { continue; }
        if (diag.range.start.line <= kiteSpanLine && diag.range.end.line >= kiteSpanLine) {
          if (diag.severity === vscode.DiagnosticSeverity.Error) {
            status = 'fail';
            message += `\n\n❌ ${diag.message}`;
            break;
          } else if (diag.severity === vscode.DiagnosticSeverity.Warning) {
            status = 'warning';
            message += `\n\n⚠️ ${diag.message}`;
          }
        }
      }
    }

    const assoc: Association = {
      kiteSpec: binding.kite_spec,
      kiteFile: binding.kite_file,
      kiteSpan: binding.kite_span,
      targetPath: binding.target_path,
      sourceSpan: binding.source_span,
      status,
      message
    };

    const fileUri = vscode.Uri.file(binding.target_path).toString();
    const existing = result.get(fileUri) ?? [];
    existing.push(assoc);
    result.set(fileUri, existing);
  }
  return result;
}

type SourceAugmentation = {
  line: number;
  label: string;
  detail: string;
  sourceLine?: number;
  kiteSpec?: string;
};

const kiteDiagnosticsByDocument = new Map<string, Map<string, SourceAugmentation[]>>();
const sourceAugmentations = new Map<string, SourceAugmentation[]>();
const sourceInlayHintEmitter = new vscode.EventEmitter<void>();
const sourceDiagnosticDecorationType = vscode.window.createTextEditorDecorationType({
  after: {
    color: new vscode.ThemeColor('editorCodeLens.foreground'),
    margin: '0 0 0 1rem'
  },
  overviewRulerColor: new vscode.ThemeColor('editorWarning.foreground'),
  overviewRulerLane: vscode.OverviewRulerLane.Right
});

const sourceInlayHintsProvider: vscode.InlayHintsProvider = {
  onDidChangeInlayHints: sourceInlayHintEmitter.event,
  provideInlayHints(document, range) {
    const entries = sourceAugmentations.get(document.uri.toString());
    if (!entries || entries.length === 0) {
      return [];
    }

    return entries.flatMap((entry) => {
      const line = clampLine(
        entry.sourceLine !== undefined ? entry.sourceLine : entry.line,
        document.lineCount
      );
      if (line < range.start.line || line > range.end.line) {
        return [];
      }

      const position = document.lineAt(line).range.end;
      const label = entry.kiteSpec ? `← ${entry.kiteSpec}` : entry.label;
      const hint = new vscode.InlayHint(position, label, vscode.InlayHintKind.Type);
      hint.paddingLeft = true;
      hint.tooltip = entry.detail;
      return [hint];
    });
  }
};

let passDecorType: vscode.TextEditorDecorationType;
let failDecorType: vscode.TextEditorDecorationType;
let warnDecorType: vscode.TextEditorDecorationType;

export function activate(context: vscode.ExtensionContext): void {
  const outputChannel = vscode.window.createOutputChannel('Kite Language Server');

  passDecorType = vscode.window.createTextEditorDecorationType({
    gutterIconPath: context.asAbsolutePath('media/kite-pass.svg'),
    gutterIconSize: 'contain'
  });
  failDecorType = vscode.window.createTextEditorDecorationType({
    gutterIconPath: context.asAbsolutePath('media/kite-fail.svg'),
    gutterIconSize: 'contain'
  });
  warnDecorType = vscode.window.createTextEditorDecorationType({
    gutterIconPath: context.asAbsolutePath('media/kite-warning.svg'),
    gutterIconSize: 'contain'
  });

  const configuredServerPath = vscode.workspace
    .getConfiguration('kite')
    .get<string>('server.path', 'kite');
  const serverPath = resolveServerPath(configuredServerPath, context.extensionMode);
  outputChannel.appendLine(`[kite] Extension mode: ${vscode.ExtensionMode[context.extensionMode]}`);
  outputChannel.appendLine(`[kite] Configured server path: "${configuredServerPath}"`);
  outputChannel.appendLine(`[kite] Resolved server path: "${serverPath}"`);
  outputChannel.appendLine(`[kite] Server command: ${serverPath} start-lsp`);

  const executable: Executable = {
    command: serverPath,
    args: ['start-lsp']
  };

  const serverOptions: ServerOptions = {
    run: executable,
    debug: executable
  };

  const kiteWatcher = vscode.workspace.createFileSystemWatcher('**/*.kite');
  const sourceWatcher = vscode.workspace.createFileSystemWatcher(
    '**/*.{rs,go,ts,tsx,js,jsx,py,java,cs}'
  );

  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: 'file', language: 'kite' },
      { scheme: 'file', language: 'rust' },
      { scheme: 'file', language: 'typescript' },
      { scheme: 'file', language: 'typescriptreact' },
      { scheme: 'file', language: 'go' },
      { scheme: 'file', language: 'python' },
      { scheme: 'file', language: 'csharp' },
      { scheme: 'file', language: 'prisma' }
    ],
    synchronize: {
      fileEvents: [kiteWatcher, sourceWatcher]
    },
    outputChannel
  };

  client = new LanguageClient(
    'kiteLanguageServer',
    'Kite Language Server',
    serverOptions,
    clientOptions
  );

  client.onNotification('kite/publishAssociations', (params: any) => {
    const bindings: any[] = params.bindings ?? [];
    const violations: any[] = params.violations ?? [];

    outputChannel.appendLine(`[kite] received publishAssociations: ${bindings.length} bindings, ${violations.length} violations`);
    for (const binding of bindings.slice(0, 5)) {
      outputChannel.appendLine(`[kite]   binding: spec=${binding.kite_spec} target=${binding.target_path} kite_file=${binding.kite_file}`);
    }

    cachedRawBindings = bindings;
    allAssociationsByFile = buildAssociationsFromBindings(bindings);

    outputChannel.appendLine(`[kite] associations map has ${allAssociationsByFile.size} unique target files`);
    for (const editor of vscode.window.visibleTextEditors) {
      applySourceDecorations(editor);
    }
  });

  function refreshBindingStatuses(): void {
    if (cachedRawBindings.length === 0) { return; }
    allAssociationsByFile = buildAssociationsFromBindings(cachedRawBindings);
    for (const editor of vscode.window.visibleTextEditors) {
      applySourceDecorations(editor);
    }
  }

  const inlayHintsRegistration = vscode.languages.registerInlayHintsProvider(
    { scheme: 'file' },
    sourceInlayHintsProvider
  );

  const diagnosticsListener = vscode.languages.onDidChangeDiagnostics((event) => {
    updateSourceAugmentationsForKiteUris(event.uris);

    // If any .kite file diagnostics changed, re-evaluate binding statuses
    const kiteChanged = event.uris.some(uri => uri.fsPath.endsWith('.kite'));
    if (kiteChanged && allAssociationsByFile.size > 0) {
      refreshBindingStatuses();
    }
  });

  const closeListener = vscode.workspace.onDidCloseTextDocument((document) => {
    if (isKiteUri(document.uri)) {
      clearSourceAugmentationsForKiteUri(document.uri);
    }
  });

  const visibleEditorsListener = vscode.window.onDidChangeVisibleTextEditors((editors) => {
    for (const editor of editors) {
      applySourceDecorations(editor);
    }
  });

  updateSourceAugmentationsForKiteUris(
    vscode.languages.getDiagnostics().map(([uri]) => uri)
  );
  for (const editor of vscode.window.visibleTextEditors) {
    applySourceDecorations(editor);
  }

  context.subscriptions.push(
    vscode.commands.registerCommand('kite.findRelatedSpecs', async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor) {
        return;
      }

      const uri = editor.document.uri.toString();
      const line = editor.selection.active.line;

      const associations = allAssociationsByFile.get(uri) ?? [];
      const related = associations.filter(
        (assoc) =>
          (assoc.sourceSpan?.start_line !== undefined && assoc.sourceSpan.start_line === line + 1) ||
          (assoc.sourceSpan === undefined && line === 0)
      );

      if (related.length === 0) {
        vscode.window.showInformationMessage('No related Kite specifications found for this line.');
        return;
      }

      const items = related.map((assoc) => ({
        label: assoc.kiteSpec,
        description: path.basename(assoc.kiteFile),
        assoc
      }));

      const selected = await vscode.window.showQuickPick(items, {
        placeHolder: 'Select a Kite specification to open'
      });

      if (selected) {
        const doc = await vscode.workspace.openTextDocument(vscode.Uri.file(selected.assoc.kiteFile));
        const editor = await vscode.window.showTextDocument(doc);
        const span = selected.assoc.kiteSpan;
        const range = new vscode.Range(
          span.start_line - 1,
          span.start_column - 1,
          span.end_line - 1,
          span.end_column - 1
        );
        editor.selection = new vscode.Selection(range.start, range.start);
        editor.revealRange(range);
      }
    })
  );

  client.start().then(
    () => outputChannel.appendLine('[kite] Language server started successfully'),
    (err) => {
      outputChannel.appendLine(`[kite] ERROR: Language server failed to start: ${err}`);
      vscode.window.showErrorMessage(`Kite LSP failed to start: ${err}`);
    }
  );
  context.subscriptions.push(
    outputChannel,
    kiteWatcher,
    sourceWatcher,
    inlayHintsRegistration,
    sourceInlayHintEmitter,
    sourceDiagnosticDecorationType,
    diagnosticsListener,
    closeListener,
    visibleEditorsListener
  );
  context.subscriptions.push({
    dispose: () => {
      void client?.stop();
    }
  });
}

export async function deactivate(): Promise<void> {
  if (client) {
    await client.stop();
    client = undefined;
  }

  kiteDiagnosticsByDocument.clear();
  sourceAugmentations.clear();
  sourceInlayHintEmitter.fire();
}

function resolveServerPath(serverPath: string, mode: vscode.ExtensionMode): string {
  if (serverPath !== 'kite' || mode !== vscode.ExtensionMode.Development) {
    return serverPath;
  }

  const workspaceFolder = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  if (!workspaceFolder) {
    return serverPath;
  }

  const binaryName = process.platform === 'win32' ? 'kite.exe' : 'kite';
  const localBinaryPath = path.resolve(workspaceFolder, '..', 'target', 'debug', binaryName);
  if (fs.existsSync(localBinaryPath)) {
    return localBinaryPath;
  }

  return serverPath;
}

function updateSourceAugmentationsForKiteUris(uris: readonly vscode.Uri[]): void {
  const affectedSourceUris = new Set<string>();

  for (const uri of uris) {
    if (!isKiteUri(uri)) {
      continue;
    }

    const kiteUri = uri.toString();
    const previous = kiteDiagnosticsByDocument.get(kiteUri);
    if (previous) {
      for (const sourceUri of previous.keys()) {
        affectedSourceUris.add(sourceUri);
      }
    }

    const next = collectAugmentationsForKiteUri(uri);
    if (next.size > 0) {
      kiteDiagnosticsByDocument.set(kiteUri, next);
      for (const sourceUri of next.keys()) {
        affectedSourceUris.add(sourceUri);
      }
    } else {
      kiteDiagnosticsByDocument.delete(kiteUri);
    }
  }

  refreshSourceAugmentations(affectedSourceUris);
}

function clearSourceAugmentationsForKiteUri(uri: vscode.Uri): void {
  const key = uri.toString();
  const existing = kiteDiagnosticsByDocument.get(key);
  if (!existing) {
    return;
  }

  const affectedSourceUris = new Set(existing.keys());
  kiteDiagnosticsByDocument.delete(key);
  refreshSourceAugmentations(affectedSourceUris);
}

function refreshSourceAugmentations(affectedSourceUris: Set<string>): void {
  if (affectedSourceUris.size === 0) {
    return;
  }

  for (const sourceUri of affectedSourceUris) {
    const merged: SourceAugmentation[] = [];
    const seen = new Set<string>();

    for (const diagnosticsBySource of kiteDiagnosticsByDocument.values()) {
      const entries = diagnosticsBySource.get(sourceUri);
      if (!entries) {
        continue;
      }

      for (const entry of entries) {
        const dedupeKey = `${entry.line}|${entry.label}|${entry.detail}`;
        if (seen.has(dedupeKey)) {
          continue;
        }
        seen.add(dedupeKey);
        merged.push(entry);
      }
    }

    if (merged.length === 0) {
      sourceAugmentations.delete(sourceUri);
      continue;
    }

    merged.sort((left, right) => left.line - right.line);
    sourceAugmentations.set(sourceUri, merged);
  }

  for (const editor of vscode.window.visibleTextEditors) {
    if (affectedSourceUris.has(editor.document.uri.toString())) {
      applySourceDecorations(editor);
    }
  }

  sourceInlayHintEmitter.fire();
}

function collectAugmentationsForKiteUri(uri: vscode.Uri): Map<string, SourceAugmentation[]> {
  const diagnostics = vscode.languages
    .getDiagnostics(uri)
    .filter((diagnostic) => diagnostic.source === 'kite');
  const bySourceUri = new Map<string, SourceAugmentation[]>();

  for (const diagnostic of diagnostics) {
    const boundPath = boundFilePathFromDiagnostic(diagnostic);
    if (!boundPath) {
      continue;
    }

    const sourceUri = resolveBoundFileUri(boundPath, uri);
    if (!sourceUri || sourceUri.toString() === uri.toString()) {
      continue;
    }

    const sourceKey = sourceUri.toString();
    const existing = bySourceUri.get(sourceKey) ?? [];
    const diag = diagnostic as any;
    const sourceSpan = diag.data?.sourceSpan;
    const kiteSpec = diag.data?.kiteSpec;
    existing.push({
      line: diagnostic.range.start.line,
      label: labelForDiagnostic(diagnostic),
      detail: detailForDiagnostic(diagnostic),
      sourceLine: sourceSpan?.start_line !== undefined ? sourceSpan.start_line - 1 : undefined,
      kiteSpec
    });
    bySourceUri.set(sourceKey, existing);
  }

  return bySourceUri;
}

function applySourceDecorations(editor: vscode.TextEditor): void {
  const uri = editor.document.uri.toString();
  const augmentations = sourceAugmentations.get(uri) ?? [];
  const associations = allAssociationsByFile.get(uri) ?? [];

  // Diagnostic-based (text decorations)
  const diagDecorations = augmentations.map((entry) => {
    const line = clampLine(
      entry.sourceLine !== undefined ? entry.sourceLine : entry.line,
      editor.document.lineCount
    );
    const position = editor.document.lineAt(line).range.end;
    const label = entry.kiteSpec ? `← ${entry.kiteSpec}` : entry.label;
    return {
      range: new vscode.Range(position, position),
      hoverMessage: entry.detail,
      renderOptions: {
        after: {
          contentText: label
        }
      }
    } satisfies vscode.DecorationOptions;
  });
  editor.setDecorations(sourceDiagnosticDecorationType, diagDecorations);

  // Association-based (gutter icons)
  if (passDecorType && failDecorType && warnDecorType) {
    const passDecors: vscode.DecorationOptions[] = [];
    const failDecors: vscode.DecorationOptions[] = [];
    const warnDecors: vscode.DecorationOptions[] = [];

    for (const assoc of associations) {
      // Skip aggregate-level bindings with no resolved source span
      if (!assoc.sourceSpan?.start_line) {
        continue;
      }
      const line = clampLine(
        assoc.sourceSpan.start_line - 1,
        editor.document.lineCount
      );
      const position = editor.document.lineAt(line).range.start;
      const decor = {
        range: new vscode.Range(position, position),
        hoverMessage: assoc.message
      };

      if (assoc.status === 'pass') {
        passDecors.push(decor);
      } else if (assoc.status === 'fail') {
        failDecors.push(decor);
      } else if (assoc.status === 'warning') {
        warnDecors.push(decor);
      }
    }

    editor.setDecorations(passDecorType, passDecors);
    editor.setDecorations(failDecorType, failDecors);
    editor.setDecorations(warnDecorType, warnDecors);
  }
}

function isKiteUri(uri: vscode.Uri): boolean {
  return uri.scheme === 'file' && uri.fsPath.toLowerCase().endsWith('.kite');
}

function boundFilePathFromDiagnostic(diagnostic: vscode.Diagnostic): string | undefined {
  const inMessage = quotedValueAfterPrefix(diagnostic.message, "bound file '");
  if (inMessage) {
    return inMessage;
  }

  for (const related of diagnostic.relatedInformation ?? []) {
    const inRelated = quotedValueAfterPrefix(related.message, "bound file '");
    if (inRelated) {
      return inRelated;
    }
  }

  return undefined;
}

function resolveBoundFileUri(boundPath: string, kiteUri: vscode.Uri): vscode.Uri | undefined {
  if (kiteUri.scheme !== 'file') {
    return undefined;
  }

  if (boundPath.startsWith('file://')) {
    return vscode.Uri.parse(boundPath);
  }

  const absolutePath = path.isAbsolute(boundPath)
    ? boundPath
    : path.resolve(path.dirname(kiteUri.fsPath), boundPath);

  return vscode.Uri.file(absolutePath);
}

function labelForDiagnostic(diagnostic: vscode.Diagnostic): string {
  const code = diagnosticCode(diagnostic);
  switch (code) {
    case 'DICTIONARY_TERM_FORBIDDEN':
      return 'kite: forbidden term';
    case 'CONTEXT_BOUNDARY_FORBIDDEN':
    case 'CONTEXT_BOUNDARY_DUPLICATE_FORBID':
    case 'CONTEXT_BOUNDARY_SELF_FORBID':
      return 'kite: boundary violation';
    case 'BINDING_FILE_NOT_FOUND':
      return 'kite: missing bound file';
    case 'BINDING_SYMBOL_NOT_FOUND':
      return 'kite: unresolved symbol';
    case 'BINDING_SYMBOL_UNVERIFIED_DEPENDENCY':
      return 'kite: blocked symbol check';
    case 'BINDING_HASH_MISMATCH':
      return 'kite: hash mismatch';
    case 'BINDING_HASH_INVALID_FORMAT':
      return 'kite: invalid hash format';
    case 'COMMAND_BINDING_ARITY_MISMATCH':
      return 'kite: binding arity mismatch';
    case 'COMMAND_BINDING_INTENT_SUSPICIOUS':
      return 'kite: suspicious binding';
    default:
      return 'kite: binding diagnostic';
  }
}

function detailForDiagnostic(diagnostic: vscode.Diagnostic): string {
  const details = [diagnostic.message];
  const code = diagnosticCode(diagnostic);
  if (code) {
    details.push(`code: ${code}`);
  }

  for (const related of diagnostic.relatedInformation ?? []) {
    details.push(`related: ${related.message}`);
  }

  return details.join('\n');
}

function diagnosticCode(diagnostic: vscode.Diagnostic): string | undefined {
  if (typeof diagnostic.code === 'string') {
    return diagnostic.code;
  }
  if (typeof diagnostic.code === 'number') {
    return diagnostic.code.toString();
  }
  if (
    diagnostic.code &&
    typeof diagnostic.code === 'object' &&
    typeof diagnostic.code.value !== 'undefined'
  ) {
    return diagnostic.code.value.toString();
  }

  return undefined;
}

function quotedValueAfterPrefix(value: string, prefix: string): string | undefined {
  const start = value.indexOf(prefix);
  if (start < 0) {
    return undefined;
  }

  const remaining = value.slice(start + prefix.length);
  const end = remaining.indexOf('\'');
  if (end < 0) {
    return undefined;
  }

  return remaining.slice(0, end);
}

function clampLine(line: number, lineCount: number): number {
  if (lineCount <= 1) {
    return 0;
  }
  return Math.min(Math.max(line, 0), lineCount - 1);
}
