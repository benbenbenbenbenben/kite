import * as assert from 'node:assert';
import * as fs from 'node:fs';
import * as path from 'node:path';
import * as vscode from 'vscode';

const FIXTURE_DIR = path.resolve(__dirname, '..', '..', '..', 'src', 'test', 'fixtures');

/**
 * Resolve the kite binary. Fails the test if it can't be found —
 * the binary must be built before running integration tests.
 */
function resolveKiteBinary(): string {
  const repoRoot = path.resolve(__dirname, '..', '..', '..', '..');
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
      'The language server may have failed to start.'
  );
}

suite('Kite VS Code Extension', function () {
  const outlineFixture = vscode.Uri.file(path.join(FIXTURE_DIR, 'outline.kite'));
  let kiteBinary: string;

  suiteSetup(async function () {
    kiteBinary = resolveKiteBinary();
    // Point the extension at the local binary
    await vscode.workspace
      .getConfiguration('kite')
      .update('server.path', kiteBinary, vscode.ConfigurationTarget.Global);
  });

  suite('Activation', () => {
    test('should activate when a .kite file is opened', async () => {
      await waitForActivation(outlineFixture);
      const doc = vscode.window.activeTextEditor?.document;
      assert.ok(doc, 'Expected an active text editor');
      assert.strictEqual(doc.languageId, 'kite');
    });
  });

  suite('Document Symbols (Outline)', function () {
    this.timeout(20_000);

    test('should provide document symbols for a .kite file', async function () {
      await waitForActivation(outlineFixture);
      const symbols = await waitForSymbols(outlineFixture);

      // Top level: one context
      assert.ok(symbols.length >= 1, `Expected at least 1 context, got ${symbols.length}`);
      const context = symbols[0];
      assert.strictEqual(context.name, 'OutlineTestContext');

      // Children: dictionary, boundary, aggregate
      const children = context.children;
      assert.ok(children.length >= 3, `Expected ≥3 children, got ${children.length}`);

      const names = children.map((c) => c.name);
      assert.ok(names.includes('dictionary'), `Expected 'dictionary' in ${JSON.stringify(names)}`);
      assert.ok(names.includes('boundary'), `Expected 'boundary' in ${JSON.stringify(names)}`);
      assert.ok(names.includes('Order'), `Expected 'Order' in ${JSON.stringify(names)}`);

      // Aggregate children
      const aggregate = children.find((c) => c.name === 'Order');
      assert.ok(aggregate, 'Expected aggregate Order');
      assert.ok(
        aggregate.children.length >= 3,
        `Expected ≥3 aggregate children, got ${aggregate.children.length}`
      );

      const aggregateNames = aggregate.children.map((c) => c.name);
      assert.ok(
        aggregateNames.includes('ship'),
        `Expected 'ship' in ${JSON.stringify(aggregateNames)}`
      );
      assert.ok(
        aggregateNames.includes('cancel'),
        `Expected 'cancel' in ${JSON.stringify(aggregateNames)}`
      );
      assert.ok(
        aggregateNames.includes('MustHaveItems'),
        `Expected 'MustHaveItems' in ${JSON.stringify(aggregateNames)}`
      );
    });
  });
});
