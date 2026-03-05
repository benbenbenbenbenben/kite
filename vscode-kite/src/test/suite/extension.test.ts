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

  suite('Commands', function () {
    test('should register kite.findRelatedSpecs command', async () => {
      const commands = await vscode.commands.getCommands(true);
      assert.ok(commands.includes('kite.findRelatedSpecs'), 'Command kite.findRelatedSpecs not registered');
    });
  });

  suite('Intent Diagnostic Propagation', function () {
    this.timeout(60_000);

    const workspaceRoot = () =>
      vscode.workspace.workspaceFolders?.[0]?.uri.fsPath ?? '';

    const demosKitePath = () =>
      path.join(workspaceRoot(), 'shipping-co', 'domain', 'demos.kite');

    const intentDemoTsPath = () =>
      path.join(workspaceRoot(), 'shipping-co', 'src', 'lib', 'demos', 'intent_demo.ts');

    /**
     * Poll for diagnostics matching a predicate on a given URI.
     * Returns [allDiagnostics, matched] when found, or throws on timeout.
     */
    async function waitForDiagnostics(
      uri: vscode.Uri,
      predicate: (d: vscode.Diagnostic) => boolean,
      description: string,
      maxAttempts = 30,
      intervalMs = 1000
    ): Promise<vscode.Diagnostic[]> {
      for (let i = 0; i < maxAttempts; i++) {
        const diags = vscode.languages.getDiagnostics(uri);
        const matched = diags.filter(predicate);
        if (matched.length > 0) {
          return matched;
        }
        await new Promise((r) => setTimeout(r, intervalMs));
      }
      // Dump what we DO have for debugging
      const all = vscode.languages.getDiagnostics(uri);
      const summary = all.map(d =>
        `[${d.source}] sev=${d.severity} code=${typeof d.code === 'object' ? (d.code as any).value : d.code} msg=${d.message.slice(0, 80)}`
      ).join('\n  ');
      throw new Error(
        `Timed out waiting for diagnostics: ${description}\n` +
        `URI: ${uri.toString()}\n` +
        `Current diagnostics (${all.length}):\n  ${summary || '(none)'}`
      );
    }

    test('demos.kite should produce COMMAND_INTENT_MISMATCH on the .kite file', async function () {
      const kiteUri = vscode.Uri.file(demosKitePath());
      await waitForActivation(kiteUri);

      // Wait for kite diagnostics to include intent mismatch
      const matched = await waitForDiagnostics(
        kiteUri,
        d => d.source === 'kite' && diagnosticCodeValue(d) === 'COMMAND_INTENT_MISMATCH',
        'COMMAND_INTENT_MISMATCH on demos.kite'
      );

      assert.ok(matched.length > 0, 'Expected COMMAND_INTENT_MISMATCH diagnostic on demos.kite');
      assert.ok(
        matched[0].message.includes('updateProfile'),
        `Expected message to mention 'updateProfile', got: ${matched[0].message}`
      );
    });

    test('intent_demo.ts should receive source diagnostics from .kite intent mismatch', async function () {
      // Ensure the extension is active by opening the .kite file first
      const kiteUri = vscode.Uri.file(demosKitePath());
      await waitForActivation(kiteUri);

      // Wait for the .kite diagnostic to exist first
      await waitForDiagnostics(
        kiteUri,
        d => d.source === 'kite' && diagnosticCodeValue(d) === 'COMMAND_INTENT_MISMATCH',
        'COMMAND_INTENT_MISMATCH on demos.kite (prerequisite)'
      );

      // Now check intent_demo.ts for source diagnostics
      const tsUri = vscode.Uri.file(intentDemoTsPath());
      try {
        const matched = await waitForDiagnostics(
          tsUri,
          d => d.source === 'kite',
          'any kite diagnostic on intent_demo.ts',
          15,
          1000
        );
        // If we get here, diagnostics propagated — success
        assert.ok(matched.length > 0, 'Expected at least one kite diagnostic on intent_demo.ts');
      } catch {
        // This is the bug — diagnostics did NOT propagate to the source file
        const kiteDiags = vscode.languages.getDiagnostics(kiteUri);
        const tsDiags = vscode.languages.getDiagnostics(tsUri);
        assert.fail(
          `Intent diagnostic did NOT propagate to intent_demo.ts.\n` +
          `demos.kite has ${kiteDiags.length} diagnostics (${kiteDiags.filter(d => d.source === 'kite').length} from kite)\n` +
          `intent_demo.ts has ${tsDiags.length} diagnostics (${tsDiags.filter(d => d.source === 'kite').length} from kite)\n` +
          `This confirms the bug: .kite intent diagnostics don't propagate to source files.`
        );
      }
    });

    test('editing .kite file intent should update source file diagnostics', async function () {
      const kiteUri = vscode.Uri.file(demosKitePath());
      await waitForActivation(kiteUri);

      // Wait for initial diagnostics
      await waitForDiagnostics(
        kiteUri,
        d => d.source === 'kite',
        'initial kite diagnostics on demos.kite'
      );

      // Open the .kite file and insert a comment to trigger did_change
      const doc = await vscode.workspace.openTextDocument(kiteUri);
      const editor = await vscode.window.showTextDocument(doc);

      // Add a comment at the top — this should trigger a full workspace diagnostic refresh
      await editor.edit(editBuilder => {
        editBuilder.insert(new vscode.Position(0, 0), '// intent test edit\n');
      });

      // Wait a bit for the debounced refresh (300ms + processing)
      await new Promise(r => setTimeout(r, 2000));

      // Now check intent_demo.ts for diagnostics
      const tsUri = vscode.Uri.file(intentDemoTsPath());
      const tsDiags = vscode.languages.getDiagnostics(tsUri);
      const kiteDiags = tsDiags.filter(d => d.source === 'kite');

      // Log what we have for debugging
      console.log(`[intent-test] After edit: intent_demo.ts has ${tsDiags.length} total diagnostics, ${kiteDiags.length} from kite`);
      for (const d of tsDiags) {
        console.log(`[intent-test]   [${d.source}] sev=${d.severity} code=${typeof d.code === 'object' ? (d.code as any).value : d.code} msg=${d.message.slice(0, 100)}`);
      }

      // Revert the edit
      await editor.edit(editBuilder => {
        editBuilder.delete(new vscode.Range(0, 0, 1, 0));
      });
      await doc.save();

      // The assertion — after editing a .kite file, the source file should have kite diagnostics
      assert.ok(
        kiteDiags.length > 0,
        `Expected kite diagnostics on intent_demo.ts after editing demos.kite, but found ${kiteDiags.length}. ` +
        `Total diagnostics: ${tsDiags.length}.`
      );
    });
  });
});

function diagnosticCodeValue(d: vscode.Diagnostic): string | undefined {
  if (typeof d.code === 'string') { return d.code; }
  if (typeof d.code === 'number') { return d.code.toString(); }
  if (d.code && typeof d.code === 'object' && 'value' in d.code) {
    return (d.code as any).value?.toString();
  }
  return undefined;
}
