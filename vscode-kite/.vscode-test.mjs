import { defineConfig } from '@vscode/test-cli';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  files: 'out/test/**/*.test.js',
  workspaceFolder: path.resolve(__dirname, 'src', 'test', 'fixtures'),
  mocha: {
    ui: 'tdd',
    timeout: 30_000,
  },
});
