const state = {
  seed: 42,
  viewer: 0,
  view: null,
  hint: null,
  autoplayTimer: null,
  stepInFlight: false,
};

const elements = {
  seedInput: document.querySelector('#seedInput'),
  dealButton: document.querySelector('#dealButton'),
  hintButton: document.querySelector('#hintButton'),
  stepButton: document.querySelector('#stepButton'),
  autoplayToggle: document.querySelector('#autoplayToggle'),
  roundMeta: document.querySelector('#roundMeta'),
  statusText: document.querySelector('#statusText'),
  viewerText: document.querySelector('#viewerText'),
  currentPlayerText: document.querySelector('#currentPlayerText'),
  winnerText: document.querySelector('#winnerText'),
  bottomCards: document.querySelector('#bottomCards'),
  historyList: document.querySelector('#historyList'),
  viewerTabs: Array.from(document.querySelectorAll('.vtab')),
  players: [
    document.querySelector('#player0'),
    document.querySelector('#player1'),
    document.querySelector('#player2'),
  ],
  playZones: [
    document.querySelector('#play0'),
    document.querySelector('#play1'),
    document.querySelector('#play2'),
  ],
  winnerOverlay: document.querySelector('#winnerOverlay'),
  winnerIcon: document.querySelector('#winnerIcon'),
  winnerTitle: document.querySelector('#winnerTitle'),
  winnerDetail: document.querySelector('#winnerDetail'),
  newGameBtn: document.querySelector('#newGameBtn'),
  historyTrigger: document.querySelector('#historyTrigger'),
  historyDrawer: document.querySelector('#historyDrawer'),
  drawerClose: document.querySelector('#drawerClose'),
};

const relationshipLabels = {
  SelfPlayer: '自己',
  Ally: '队友',
  Opponent: '对手',
};

const roleLabels = {
  Landlord: '地主',
  Peasant: '农民',
};

const suitLabels = {
  C: '♣',
  D: '♦',
  H: '♥',
  S: '♠',
};

const rankLabels = {
  J: 'J',
  Q: 'Q',
  K: 'K',
  A: 'A',
};

elements.dealButton.addEventListener('click', () => startGame());
elements.hintButton.addEventListener('click', () => requestHint());
elements.stepButton.addEventListener('click', () => requestStep());
elements.autoplayToggle.addEventListener('change', () => setAutoplay(elements.autoplayToggle.checked));
elements.seedInput.addEventListener('change', () => {
  state.seed = normalizedSeed();
  elements.seedInput.value = String(state.seed);
});
elements.newGameBtn.addEventListener('click', () => {
  hideWinner();
  startGame();
});
elements.historyTrigger.addEventListener('click', toggleHistory);
elements.drawerClose.addEventListener('click', () => {
  elements.historyDrawer.hidden = true;
});

for (const tab of elements.viewerTabs) {
  tab.addEventListener('click', () => switchViewer(Number(tab.dataset.viewer)));
}

startGame();

async function startGame() {
  state.seed = normalizedSeed();
  state.hint = null;
  stopAutoplay();
  hideWinner();
  setBusy(true, '发牌中');
  try {
    state.view = await window.doudizhu.startGame({
      seed: state.seed,
      viewer: state.viewer,
    });
    render();
    elements.statusText.textContent = '已开局';
  } catch (error) {
    showError(error);
  } finally {
    setBusy(false);
  }
}

async function switchViewer(viewer) {
  state.viewer = viewer;
  state.hint = null;
  setActiveViewer();
  if (!state.view?.game_id) {
    await startGame();
    return;
  }

  setBusy(true, '切换中');
  try {
    state.view = await window.doudizhu.setViewer({
      gameId: state.view.game_id,
      viewer,
    });
    render();
    elements.statusText.textContent = '已切换视角';
  } catch (error) {
    showError(error);
  } finally {
    setBusy(false);
  }
}

async function requestHint() {
  if (!state.view?.game_id) {
    return;
  }

  setBusy(true, '提示中');
  try {
    state.hint = await window.doudizhu.getHint({
      gameId: state.view.game_id,
      viewer: state.viewer,
    });
    render();
    const count = state.hint.candidates?.length || 0;
    elements.statusText.textContent = count > 0 ? `提示 ${count} 手` : '暂无提示';
  } catch (error) {
    showError(error);
  } finally {
    setBusy(false);
  }
}

async function requestStep() {
  if (!state.view?.game_id) {
    return;
  }
  if (state.stepInFlight) {
    return;
  }

  state.stepInFlight = true;
  state.hint = null;
  setBusy(true, '执行中');
  try {
    state.view = await window.doudizhu.autoStep({
      gameId: state.view.game_id,
      viewer: state.viewer,
    });
    render();
    elements.statusText.textContent = '已单步';
    if (state.view.winner !== null && state.view.winner !== undefined) {
      stopAutoplay();
      showWinner(state.view.winner);
    }
  } catch (error) {
    stopAutoplay();
    showError(error);
  } finally {
    state.stepInFlight = false;
    setBusy(false);
  }
}

function setAutoplay(enabled) {
  if (!enabled) {
    stopAutoplay();
    return;
  }

  if (state.autoplayTimer) {
    return;
  }

  elements.autoplayToggle.checked = true;
  state.autoplayTimer = window.setInterval(() => {
    requestStep();
  }, 200);
  requestStep();
}

function stopAutoplay() {
  if (state.autoplayTimer) {
    window.clearInterval(state.autoplayTimer);
    state.autoplayTimer = null;
  }
  elements.autoplayToggle.checked = false;
}

function toggleHistory() {
  const drawer = elements.historyDrawer;
  drawer.hidden = !drawer.hidden;
}

function render() {
  const view = state.view;
  if (!view) {
    return;
  }

  elements.roundMeta.textContent = `Seed ${view.seed} · ${view.game_id}`;
  const viewer = view.players.find((player) => player.id === view.viewer);
  elements.viewerText.textContent = `玩家 ${view.viewer} 视角 · ${roleLabel(viewer?.role)}`;
  elements.currentPlayerText.textContent = view.winner == null ? `轮到 玩家 ${view.current_player}` : '';
  elements.winnerText.textContent = '';
  renderBottomCards(view.bottom_cards);
  renderHistory(view.history);
  renderPlayZones(view.history, view.viewer);
  setActiveViewer();

  const orderedPlayers = tableOrder(view.viewer);
  const hintedCards = new Set(state.hint?.recommended || []);
  for (const [slot, playerId] of orderedPlayers.entries()) {
    const player = view.players.find((entry) => entry.id === playerId);
    renderPlayer(elements.players[slot], player, playerId === view.viewer, view.bottom_cards, hintedCards);
  }
}

function renderPlayZones(history, viewer) {
  const lastPlays = lastPlayPerPlayer(history);
  const ordered = tableOrder(viewer);

  for (let slot = 0; slot < 3; slot++) {
    const playerId = ordered[slot];
    const zone = elements.playZones[slot];
    zone.replaceChildren();
    zone.className = 'play-zone';

    const play = lastPlays[playerId];
    if (!play) {
      continue;
    }

    if (play.action === 'pass') {
      zone.classList.add('has-play');
      zone.textContent = '不出';
    } else {
      zone.classList.add('has-play');
      zone.append(...play.cards.map((c) => cardElement(c)));
    }
  }
}

function lastPlayPerPlayer(history) {
  const result = {};
  for (let i = history.length - 1; i >= 0; i--) {
    const entry = history[i];
    if (!(entry.player in result)) {
      result[entry.player] = entry;
    }
  }
  return result;
}

function renderBottomCards(cards) {
  elements.bottomCards.replaceChildren(...cards.map((card) => cardElement(card)));
}

function renderHistory(history) {
  elements.historyList.replaceChildren();
  if (!history.length) {
    const item = document.createElement('li');
    item.className = 'history-empty';
    item.textContent = '暂无公开历史';
    elements.historyList.append(item);
    return;
  }

  for (const entry of history.slice().reverse()) {
    const item = document.createElement('li');
    item.className = 'history-item';
    const summary = document.createElement('span');
    summary.textContent = `#${entry.turn} 玩家 ${entry.player} ${actionLabel(entry.action)} ${kindLabel(entry.kind)}`;
    item.append(summary, cardCodesElement(entry.cards));
    elements.historyList.append(item);
  }
}

function renderPlayer(container, player, isSelf, bottomCards, hintedCards) {
  if (!player) {
    return;
  }

  const relationship = relationshipLabels[player.relationship] || player.relationship;
  const role = roleLabel(player.role);
  container.classList.toggle('is-self', isSelf);
  container.classList.toggle('is-landlord', player.role === 'Landlord');
  container.replaceChildren();

  const header = document.createElement('header');
  header.className = 'seat-header';
  header.append(
    textNode('h2', `玩家 ${player.id}`),
    badge(role, `role-badge ${player.role === 'Landlord' ? 'landlord' : 'peasant'}`),
    badge(relationship, 'relationship-badge'),
    textNode('strong', `${player.hand_count} 张`),
  );

  const cards = document.createElement('div');
  cards.className = isSelf ? 'cards hand-cards' : 'cards hidden-cards';
  if (isSelf) {
    const bottomSet = new Set(player.role === 'Landlord' ? bottomCards : []);
    cards.replaceChildren(
      ...player.visible_hand.map((card) =>
        cardElement(card, {
          isBottomCard: bottomSet.has(card),
          isHinted: hintedCards.has(card),
        }),
      ),
    );
  } else {
    const visibleBacks = Math.min(player.hand_count, 7);
    for (let index = 0; index < visibleBacks; index += 1) {
      const card = document.createElement('div');
      card.className = 'card card-back';
      card.setAttribute('aria-hidden', 'true');
      cards.append(card);
    }
  }

  container.append(header, cards);
}

function cardElement(code, options = {}) {
  const card = document.createElement('div');
  const parsed = parseCard(code);
  card.className = `card ${parsed.red ? 'red' : 'black'}`;
  if (options.isBottomCard) {
    card.classList.add('bottom-owned-card');
  }
  if (options.isHinted) {
    card.classList.add('hinted-card');
  }
  card.append(textNode('strong', parsed.rank), textNode('span', parsed.suit));
  card.setAttribute('aria-label', code);
  if (options.isBottomCard) {
    card.dataset.source = 'bottom';
    card.append(textNode('em', '底'));
  }
  return card;
}

function cardCodesElement(cards = []) {
  const wrapper = document.createElement('span');
  wrapper.className = 'history-cards';
  if (!cards.length) {
    wrapper.textContent = '过';
    return wrapper;
  }
  for (const card of cards) {
    const code = document.createElement('span');
    code.className = 'history-card-code';
    code.textContent = card;
    wrapper.append(code);
  }
  return wrapper;
}

function parseCard(code) {
  if (code === 'BJ') {
    return { rank: '小王', suit: '', red: false };
  }
  if (code === 'RJ') {
    return { rank: '大王', suit: '', red: true };
  }

  const suit = code.slice(-1);
  const rank = code.slice(0, -1);
  return {
    rank: rankLabels[rank] || rank,
    suit: suitLabels[suit] || suit,
    red: suit === 'D' || suit === 'H',
  };
}

function tableOrder(viewer) {
  return [viewer, (viewer + 1) % 3, (viewer + 2) % 3];
}

function setBusy(isBusy, label = '发牌中') {
  for (const button of [elements.dealButton, elements.hintButton, elements.stepButton, ...elements.viewerTabs]) {
    button.disabled = isBusy;
  }
  elements.dealButton.textContent = isBusy ? label : '重新发牌';
}

function setActiveViewer() {
  for (const tab of elements.viewerTabs) {
    tab.classList.toggle('active', Number(tab.dataset.viewer) === state.viewer);
  }
}

function showWinner(winnerId) {
  const view = state.view;
  if (!view) return;

  const winnerPlayer = view.players.find((p) => p.id === winnerId);
  const isLandlordWin = winnerPlayer?.role === 'Landlord';

  elements.winnerIcon.className = `overlay-icon ${isLandlordWin ? 'landlord-win' : 'farmer-win'}`;
  elements.winnerIcon.textContent = isLandlordWin ? '地' : '农';
  elements.winnerTitle.textContent = `玩家 ${winnerId} 获胜`;
  elements.winnerDetail.textContent = isLandlordWin ? '地主胜利' : '农民胜利';
  elements.winnerOverlay.hidden = false;
}

function hideWinner() {
  elements.winnerOverlay.hidden = true;
}

function showError(error) {
  elements.statusText.textContent = error.message;
}

function normalizedSeed() {
  const value = Number(elements.seedInput.value);
  return Number.isInteger(value) && value >= 0 ? value : 42;
}

function textNode(tagName, content) {
  const element = document.createElement(tagName);
  element.textContent = content;
  return element;
}

function badge(content, className) {
  const element = document.createElement('span');
  element.className = className;
  element.textContent = content;
  return element;
}

function roleLabel(role) {
  return roleLabels[role] || role || '未知';
}

function actionLabel(action) {
  return action === 'pass' ? '不出' : '出牌';
}

function kindLabel(kind) {
  return kind || '';
}
