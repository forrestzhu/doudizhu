const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('doudizhu', {
  startGame: ({ seed, viewer }) => ipcRenderer.invoke('start-game', { seed, viewer }),
  setViewer: ({ gameId, viewer }) => ipcRenderer.invoke('set-viewer', { gameId, viewer }),
  getHint: ({ gameId, viewer }) => ipcRenderer.invoke('get-hint', { gameId, viewer }),
  autoStep: ({ gameId, viewer }) => ipcRenderer.invoke('auto-step', { gameId, viewer }),
});
