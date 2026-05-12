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
  previousPlay: document.querySelector('#previousPlay'),
  bottomCards: document.querySelector('#bottomCards'),
  historyList: document.querySelector('#historyList'),
  viewerTabs: Array.from(document.querySelectorAll('.viewer-tab')),
  players: [
    document.querySelector('#player0'),
    document.querySelector('#player1'),
    document.querySelector('#player2'),
  ],
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

for (const tab of elements.viewerTabs) {
  tab.addEventListener('click', () => switchViewer(Number(tab.dataset.viewer)));
}

startGame();

async function startGame() {
  state.seed = normalizedSeed();
  state.hint = null;
  stopAutoplay();
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

function render() {
  const view = state.view;
  if (!view) {
    return;
  }

  elements.roundMeta.textContent = `Seed ${view.seed} · ${view.game_id}`;
  const viewer = view.players.find((player) => player.id === view.viewer);
  elements.viewerText.textContent = `玩家 ${view.viewer} 视角 · ${roleLabel(viewer?.role)}`;
  elements.currentPlayerText.textContent = `当前玩家 ${view.current_player}`;
  elements.winnerText.textContent = view.winner === null || view.winner === undefined ? '胜者 未定' : `胜者 玩家 ${view.winner}`;
  renderPreviousPlay(view.previous_play);
  renderBottomCards(view.bottom_cards);
  renderHistory(view.history);
  setActiveViewer();

  const orderedPlayers = tableOrder(view.viewer);
  const hintedCards = new Set(state.hint?.recommended || []);
  for (const [slot, playerId] of orderedPlayers.entries()) {
    const player = view.players.find((entry) => entry.id === playerId);
    renderPlayer(elements.players[slot], player, playerId === view.viewer, view.bottom_cards, hintedCards);
  }
}

function renderPreviousPlay(previousPlay) {
  elements.previousPlay.replaceChildren();
  if (!previousPlay) {
    elements.previousPlay.textContent = '上一手：无';
    return;
  }

  elements.previousPlay.append(
    document.createTextNode(`上一手：玩家 ${previousPlay.player} ${actionLabel(previousPlay.action)} ${kindLabel(previousPlay.kind)} `),
    cardCodesElement(previousPlay.cards),
  );
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
    summary.textContent = `#${entry.turn} 玩家 ${entry.player} ${actionLabel(entry.action)} ${kindLabel(entry.kind)} · 剩 ${entry.hand_count_after}`;
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
