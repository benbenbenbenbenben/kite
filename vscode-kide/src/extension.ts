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
type SourceAugmentation = {
  line: number;
  label: string;
  detail: string;
};

const kideDiagnosticsByDocument = new Map<string, Map<string, SourceAugmentation[]>>();
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
      const line = clampLine(entry.line, document.lineCount);
      if (line < range.start.line || line > range.end.line) {
        return [];
      }

      const position = document.lineAt(line).range.end;
      const hint = new vscode.InlayHint(position, entry.label, vscode.InlayHintKind.Type);
      hint.paddingLeft = true;
      hint.tooltip = entry.detail;
      return [hint];
    });
  }
};

export function activate(context: vscode.ExtensionContext): void {
  const configuredServerPath = vscode.workspace
    .getConfiguration('kide')
    .get<string>('server.path', 'kide');
  const serverPath = resolveServerPath(configuredServerPath, context.extensionMode);

  const executable: Executable = {
    command: serverPath,
    args: ['start-lsp']
  };

  const serverOptions: ServerOptions = {
    run: executable,
    debug: executable
  };

  const kideWatcher = vscode.workspace.createFileSystemWatcher('**/*.kide');
  const sourceWatcher = vscode.workspace.createFileSystemWatcher(
    '**/*.{rs,go,ts,tsx,js,jsx,py,java,cs}'
  );

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: 'file', language: 'kide' }],
    synchronize: {
      fileEvents: [kideWatcher, sourceWatcher]
    }
  };

  client = new LanguageClient(
    'kideLanguageServer',
    'Kide Language Server',
    serverOptions,
    clientOptions
  );

  const inlayHintsRegistration = vscode.languages.registerInlayHintsProvider(
    { scheme: 'file' },
    sourceInlayHintsProvider
  );

  const diagnosticsListener = vscode.languages.onDidChangeDiagnostics((event) => {
    updateSourceAugmentationsForKideUris(event.uris);
  });

  const closeListener = vscode.workspace.onDidCloseTextDocument((document) => {
    if (isKideUri(document.uri)) {
      clearSourceAugmentationsForKideUri(document.uri);
    }
  });

  const visibleEditorsListener = vscode.window.onDidChangeVisibleTextEditors((editors) => {
    for (const editor of editors) {
      applySourceDecorations(editor);
    }
  });

  updateSourceAugmentationsForKideUris(
    vscode.languages.getDiagnostics().map(([uri]) => uri)
  );
  for (const editor of vscode.window.visibleTextEditors) {
    applySourceDecorations(editor);
  }

  void client.start();
  context.subscriptions.push(
    kideWatcher,
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

  kideDiagnosticsByDocument.clear();
  sourceAugmentations.clear();
  sourceInlayHintEmitter.fire();
}

function resolveServerPath(serverPath: string, mode: vscode.ExtensionMode): string {
  if (serverPath !== 'kide' || mode !== vscode.ExtensionMode.Development) {
    return serverPath;
  }

  const workspaceFolder = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  if (!workspaceFolder) {
    return serverPath;
  }

  const binaryName = process.platform === 'win32' ? 'kide.exe' : 'kide';
  const localBinaryPath = path.resolve(workspaceFolder, '..', 'target', 'debug', binaryName);
  if (fs.existsSync(localBinaryPath)) {
    return localBinaryPath;
  }

  return serverPath;
}

function updateSourceAugmentationsForKideUris(uris: readonly vscode.Uri[]): void {
  const affectedSourceUris = new Set<string>();

  for (const uri of uris) {
    if (!isKideUri(uri)) {
      continue;
    }

    const kideUri = uri.toString();
    const previous = kideDiagnosticsByDocument.get(kideUri);
    if (previous) {
      for (const sourceUri of previous.keys()) {
        affectedSourceUris.add(sourceUri);
      }
    }

    const next = collectAugmentationsForKideUri(uri);
    if (next.size > 0) {
      kideDiagnosticsByDocument.set(kideUri, next);
      for (const sourceUri of next.keys()) {
        affectedSourceUris.add(sourceUri);
      }
    } else {
      kideDiagnosticsByDocument.delete(kideUri);
    }
  }

  refreshSourceAugmentations(affectedSourceUris);
}

function clearSourceAugmentationsForKideUri(uri: vscode.Uri): void {
  const key = uri.toString();
  const existing = kideDiagnosticsByDocument.get(key);
  if (!existing) {
    return;
  }

  const affectedSourceUris = new Set(existing.keys());
  kideDiagnosticsByDocument.delete(key);
  refreshSourceAugmentations(affectedSourceUris);
}

function refreshSourceAugmentations(affectedSourceUris: Set<string>): void {
  if (affectedSourceUris.size === 0) {
    return;
  }

  for (const sourceUri of affectedSourceUris) {
    const merged: SourceAugmentation[] = [];
    const seen = new Set<string>();

    for (const diagnosticsBySource of kideDiagnosticsByDocument.values()) {
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

function collectAugmentationsForKideUri(uri: vscode.Uri): Map<string, SourceAugmentation[]> {
  const diagnostics = vscode.languages
    .getDiagnostics(uri)
    .filter((diagnostic) => diagnostic.source === 'kide');
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
    existing.push({
      line: diagnostic.range.start.line,
      label: labelForDiagnostic(diagnostic),
      detail: detailForDiagnostic(diagnostic)
    });
    bySourceUri.set(sourceKey, existing);
  }

  return bySourceUri;
}

function applySourceDecorations(editor: vscode.TextEditor): void {
  const entries = sourceAugmentations.get(editor.document.uri.toString()) ?? [];
  if (entries.length === 0) {
    editor.setDecorations(sourceDiagnosticDecorationType, []);
    return;
  }

  const decorations = entries.map((entry) => {
    const line = clampLine(entry.line, editor.document.lineCount);
    const position = editor.document.lineAt(line).range.end;
    return {
      range: new vscode.Range(position, position),
      hoverMessage: entry.detail,
      renderOptions: {
        after: {
          contentText: entry.label
        }
      }
    } satisfies vscode.DecorationOptions;
  });

  editor.setDecorations(sourceDiagnosticDecorationType, decorations);
}

function isKideUri(uri: vscode.Uri): boolean {
  return uri.scheme === 'file' && uri.fsPath.toLowerCase().endsWith('.kide');
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

function resolveBoundFileUri(boundPath: string, kideUri: vscode.Uri): vscode.Uri | undefined {
  if (kideUri.scheme !== 'file') {
    return undefined;
  }

  if (boundPath.startsWith('file://')) {
    return vscode.Uri.parse(boundPath);
  }

  const absolutePath = path.isAbsolute(boundPath)
    ? boundPath
    : path.resolve(path.dirname(kideUri.fsPath), boundPath);

  return vscode.Uri.file(absolutePath);
}

function labelForDiagnostic(diagnostic: vscode.Diagnostic): string {
  const code = diagnosticCode(diagnostic);
  switch (code) {
    case 'DICTIONARY_TERM_FORBIDDEN':
      return 'kide: forbidden term';
    case 'CONTEXT_BOUNDARY_FORBIDDEN':
    case 'CONTEXT_BOUNDARY_DUPLICATE_FORBID':
    case 'CONTEXT_BOUNDARY_SELF_FORBID':
      return 'kide: boundary violation';
    case 'BINDING_FILE_NOT_FOUND':
      return 'kide: missing bound file';
    case 'BINDING_SYMBOL_NOT_FOUND':
      return 'kide: unresolved symbol';
    case 'BINDING_SYMBOL_UNVERIFIED_DEPENDENCY':
      return 'kide: blocked symbol check';
    case 'BINDING_HASH_MISMATCH':
      return 'kide: hash mismatch';
    case 'BINDING_HASH_INVALID_FORMAT':
      return 'kide: invalid hash format';
    case 'COMMAND_BINDING_ARITY_MISMATCH':
      return 'kide: binding arity mismatch';
    case 'COMMAND_BINDING_INTENT_SUSPICIOUS':
      return 'kide: suspicious binding';
    default:
      return 'kide: binding diagnostic';
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
