const { app, BrowserWindow, ipcMain } = require('electron');
const { execFile } = require('node:child_process');
const path = require('node:path');

const projectRoot = path.resolve(__dirname, '..');
const arenaBinary = path.join(projectRoot, 'target', 'debug', 'arena');
const sessions = new Map();
let nextGameId = 1;
let arenaBuildPromise = null;

function createWindow() {
  const isTest = process.env.E2E_TEST === '1';

  const window = new BrowserWindow({
    width: 1280,
    height: 820,
    minWidth: 980,
    minHeight: 680,
    backgroundColor: '#f5f1e8',
    title: '斗地主',
    show: !isTest,
    focusable: !isTest,
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
  ipcMain.handle('start-game', async (_event, request) => startGame(request));
  ipcMain.handle('set-viewer', async (_event, request) => setViewer(request));
  ipcMain.handle('get-hint', async (_event, request) => getHint(request));
  ipcMain.handle('auto-step', async (_event, request) => autoStep(request));
  ipcMain.handle('manual-step', async (_event, request) => manualStep(request));
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

async function startGame(request = {}) {
  const seed = integerOrDefault(request.seed, 42);
  const viewer = normalizedViewer(request.viewer);
  const gameId = `game-${nextGameId}`;
  nextGameId += 1;

  const session = {
    gameId,
    seed,
    actions: [],
    reports: new Map(),
  };
  sessions.set(gameId, session);

  return gameViewFromArena(session, viewer);
}

async function setViewer(request = {}) {
  const gameId = String(request.gameId || '');
  const session = requireSession(gameId);
  const viewer = normalizedViewer(request.viewer);
  return gameViewFromArena(session, viewer);
}

async function getHint(request = {}) {
  const gameId = String(request.gameId || '');
  const session = requireSession(gameId);
  const viewer = normalizedViewer(request.viewer);
  const view = await gameViewFromArena(session, viewer);
  if (view.current_player !== viewer || view.winner !== null) {
    return {
      recommended: [],
      candidates: [],
    };
  }

  const report = await sessionReport(session, viewer);
  return normalizeHintReport(report.hint);
}

async function autoStep(request = {}) {
  const gameId = String(request.gameId || '');
  const session = requireSession(gameId);
  const viewer = normalizedViewer(request.viewer);
  const current = await gameViewFromArena(session, viewer);
  if (current.winner === null || current.winner === undefined) {
    const candidateActions = [...session.actions, { kind: 'auto' }];
    const report = await sessionReport(session, viewer, candidateActions);
    session.actions = candidateActions;
    session.reports.clear();
    session.reports.set(String(viewer), report);
    return normalizeGameView(report.view, session.gameId);
  }

  return current;
}

async function manualStep(request = {}) {
  const gameId = String(request.gameId || '');
  const session = requireSession(gameId);
  const viewer = normalizedViewer(request.viewer);
  const cards = Array.isArray(request.cards) ? request.cards : [];
  const candidateActions = [...session.actions, { kind: 'manual', cards }];
  const report = await sessionReport(session, viewer, candidateActions);

  session.actions = candidateActions;
  session.reports.clear();
  session.reports.set(String(viewer), report);
  const view = normalizeGameView(report.view, session.gameId);
  return { view, hint: normalizeHintReport(report.hint) };
}

async function gameViewFromArena(session, viewer) {
  const report = await sessionReport(session, viewer);
  return normalizeGameView(report.view, session.gameId);
}

function sessionReport(session, viewer, actions = session.actions) {
  const cacheKey = String(viewer);
  if (actions === session.actions && session.reports.has(cacheKey)) {
    return session.reports.get(cacheKey);
  }

  return runArena([
    '--session',
    '--seed',
    String(session.seed),
    '--viewer',
    String(viewer),
    '--actions',
    JSON.stringify(actions),
    '--landlord-policy',
    'strategic',
    '--strategy-file',
    path.join(projectRoot, 'strategies/roles_v1.json'),
    '--format',
    'json',
  ]).then((report) => {
    if (actions === session.actions) {
      session.reports.set(cacheKey, report);
    }
    return report;
  });
}

function normalizeGameView(view, gameId) {
  return {
    game_id: gameId,
    seed: view.seed,
    viewer: view.viewer,
    current_player: view.current_player,
    winner: view.winner,
    bottom_cards: view.bottom_cards || [],
    previous_play: normalizePreviousPlay(view),
    players: (view.players || []).map((player) => ({
      ...player,
      visible_hand: player.id === view.viewer ? view.visible_hand || [] : [],
    })),
    history: (view.history || []).map(normalizeTurn),
  };
}

function normalizeHintReport(hint) {
  return {
    recommended: hint.recommended || [],
    candidates: (hint.legal_hints || []).map((cards) => ({
      cards,
      kind: inferKind(cards),
    })),
  };
}

function normalizeTurn(entry) {
  const action = entry.decision === 'Pass' ? 'pass' : 'play';
  return {
    turn: entry.turn,
    player: entry.player,
    action,
    cards: entry.cards || [],
    kind: entry.accepted_hand?.kind || (action === 'pass' ? 'Pass' : inferKind(entry.cards || [])),
    hand_count_after: '',
  };
}

function normalizePreviousPlay(view) {
  const hand = view.previous_play;
  if (!hand) {
    return null;
  }
  return {
    player: view.previous_player,
    action: 'play',
    cards: hand.cards || [],
    kind: hand.kind,
  };
}

function inferKind(cards) {
  if (cards.length === 0) {
    return 'Pass';
  }
  if (cards.length === 1) {
    return 'Single';
  }
  return `Cards${cards.length}`;
}

function requireSession(gameId) {
  const session = sessions.get(gameId);
  if (!session) {
    throw new Error(`unknown gameId: ${gameId}`);
  }
  return session;
}

function normalizedViewer(value) {
  const parsed = integerOrDefault(value, 0);
  return parsed >= 0 && parsed <= 2 ? parsed : 0;
}

function integerOrDefault(value, fallback) {
  const parsed = Number(value);
  return Number.isInteger(parsed) && parsed >= 0 ? parsed : fallback;
}

function runArena(args) {
  return ensureArenaBuilt().then(() => execArena(args));
}

function ensureArenaBuilt() {
  if (!arenaBuildPromise) {
    arenaBuildPromise = new Promise((resolve, reject) => {
      execFile(
        'cargo',
        ['build', '--quiet', '--bin', 'arena'],
        {
          cwd: projectRoot,
          timeout: 60_000,
          maxBuffer: 1024 * 1024,
        },
        (error, _stdout, stderr) => {
          if (error) {
            arenaBuildPromise = null;
            reject(new Error(stderr.trim() || error.message));
            return;
          }
          resolve();
        },
      );
    });
  }
  return arenaBuildPromise;
}

function execArena(args) {
  return new Promise((resolve, reject) => {
    execFile(
      arenaBinary,
      args,
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
          reject(new Error(`invalid arena JSON: ${parseError.message}`));
        }
      },
    );
  });
}
