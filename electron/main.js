const { app, BrowserWindow, ipcMain } = require('electron');
const { execFile } = require('node:child_process');
const path = require('node:path');

const projectRoot = path.resolve(__dirname, '..');

function createWindow() {
  const window = new BrowserWindow({
    width: 1280,
    height: 820,
    minWidth: 980,
    minHeight: 680,
    backgroundColor: '#f5f1e8',
    title: '斗地主',
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: false,
    },
  });

  window.loadFile(path.join(projectRoot, 'renderer/index.html'));
}

app.whenReady().then(() => {
  ipcMain.handle('deal', async (_event, request) => deal(request));
  createWindow();

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow();
    }
  });
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

function deal(request = {}) {
  const seed = integerOrDefault(request.seed, 42);
  const viewer = integerOrDefault(request.viewer, 0);
  return runHarness([
    '--deal',
    '--seed',
    String(seed),
    '--viewer',
    String(viewer),
    '--format',
    'json',
  ]);
}

function integerOrDefault(value, fallback) {
  const parsed = Number(value);
  return Number.isInteger(parsed) && parsed >= 0 ? parsed : fallback;
}

function runHarness(args) {
  return new Promise((resolve, reject) => {
    execFile(
      'cargo',
      ['run', '--quiet', '--bin', 'harness', '--', ...args],
      {
        cwd: projectRoot,
        timeout: 30_000,
        maxBuffer: 1024 * 1024,
      },
      (error, stdout, stderr) => {
        if (error) {
          reject(new Error(stderr.trim() || error.message));
          return;
        }

        try {
          resolve(JSON.parse(stdout));
        } catch (parseError) {
          reject(new Error(`invalid harness JSON: ${parseError.message}`));
        }
      },
    );
  });
}
