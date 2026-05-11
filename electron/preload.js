const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('doudizhu', {
  deal: ({ seed, viewer }) => ipcRenderer.invoke('deal', { seed, viewer }),
});
