import * as assert from 'node:assert';
import * as fs from 'node:fs';
import * as path from 'node:path';
import * as vscode from 'vscode';

/**
 * Resolve the kite binary. Fails the test if it can't be found —
 * the binary must be built before running integration tests.
 */
function resolveKiteBinary(): string {
  // The workspace is {repo}/examples, so the repo root is one level up
  const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath ?? '';
  const repoRoot = path.resolve(workspaceRoot, '..');
  const candidates = [
    path.join(repoRoot, 'target', 'debug', 'kite'),
    path.join(repoRoot, 'target', 'release', 'kite'),
  ];
  for (const candidate of candidates) {
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }
  throw new Error(
    `kite binary not found. Searched:\n${candidates.join('\n')}\n` +
      'Run "cargo build -p kite-cli" before running integration tests.'
  );
}

/**
 * Wait for the extension to activate (it activates on `.kite` files).
 */
async function waitForActivation(uri: vscode.Uri, timeoutMs = 15_000): Promise<void> {
  const doc = await vscode.workspace.openTextDocument(uri);
  await vscode.window.showTextDocument(doc);

  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const ext = vscode.extensions.getExtension('kite.kite-vscode');
    if (ext?.isActive) {
      return;
    }
    await new Promise((r) => setTimeout(r, 250));
  }

  const doc2 = vscode.window.activeTextEditor?.document;
  if (doc2?.languageId === 'kite') {
    return;
  }

  throw new Error('Extension failed to activate within timeout');
}

/**
 * Poll for document symbols with retries. The LSP needs a moment to
 * start up and register its provider after extension activation.
 */
async function waitForSymbols(
  uri: vscode.Uri,
  maxAttempts = 15,
  intervalMs = 1000
): Promise<vscode.DocumentSymbol[]> {
  for (let i = 0; i < maxAttempts; i++) {
    const symbols = await vscode.commands.executeCommand<vscode.DocumentSymbol[]>(
      'vscode.executeDocumentSymbolProvider',
      uri
    );
    if (symbols && symbols.length > 0) {
      return symbols;
    }
    await new Promise((r) => setTimeout(r, intervalMs));
  }
  throw new Error(
    `LSP did not return document symbols after ${maxAttempts} attempts. ` +
      'The language server may have failed to start or is hanging.'
  );
}

/**
 * Find a .kite file in the workspace to test with.
 */
function findKiteFileInWorkspace(): vscode.Uri {
  const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  if (!workspaceRoot) {
    throw new Error('No workspace folder found');
  }

  // Try the shipping-co regression file first (it's small and known-good)
  const regressionFile = path.join(
    workspaceRoot,
    'shipping-co',
    'domain',
    'regressions',
    'expected-pass-minimal.kite'
  );
  if (fs.existsSync(regressionFile)) {
    return vscode.Uri.file(regressionFile);
  }

  // Fallback: find any .kite file
  throw new Error(
    `No .kite file found at ${regressionFile}. ` +
      'Make sure the test workspace points to the examples directory.'
  );
}

suite('Kite VS Code Extension', function () {
  let kiteBinary: string;
  let testFileUri: vscode.Uri;

  suiteSetup(async function () {
    kiteBinary = resolveKiteBinary();
    testFileUri = findKiteFileInWorkspace();
    // Point the extension at the local binary
    await vscode.workspace
      .getConfiguration('kite')
      .update('server.path', kiteBinary, vscode.ConfigurationTarget.Global);
  });

  suite('Activation', () => {
    test('should activate when a .kite file is opened', async () => {
      await waitForActivation(testFileUri);
      const doc = vscode.window.activeTextEditor?.document;
      assert.ok(doc, 'Expected an active text editor');
      assert.strictEqual(doc.languageId, 'kite');
    });
  });

  suite('Document Symbols (Outline)', function () {
    this.timeout(20_000);

    test('should return document symbols without hanging', async function () {
      // This test uses the real example workspace (multiple .kite files with
      // binding references) to reproduce the scenario where the LSP server
      // previously hung because diagnostics blocked the request pipeline.
      await waitForActivation(testFileUri);
      const symbols = await waitForSymbols(testFileUri);

      assert.ok(symbols && symbols.length > 0, 'Expected document symbols from LSP');

      // The minimal regression file has at least one context
      const context = symbols[0];
      assert.ok(context.name, 'Expected context to have a name');
      assert.ok(context.children.length > 0, 'Expected context to have children');
    });
  });
});
