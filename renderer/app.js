const state = {
  seed: 42,
  viewer: 0,
  deal: null,
};

const elements = {
  seedInput: document.querySelector('#seedInput'),
  dealButton: document.querySelector('#dealButton'),
  roundMeta: document.querySelector('#roundMeta'),
  statusText: document.querySelector('#statusText'),
  viewerText: document.querySelector('#viewerText'),
  bottomCards: document.querySelector('#bottomCards'),
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

elements.dealButton.addEventListener('click', () => requestDeal());
elements.seedInput.addEventListener('change', () => {
  state.seed = normalizedSeed();
  elements.seedInput.value = String(state.seed);
});

for (const tab of elements.viewerTabs) {
  tab.addEventListener('click', () => {
    state.viewer = Number(tab.dataset.viewer);
    setActiveViewer();
    requestDeal();
  });
}

requestDeal();

async function requestDeal() {
  state.seed = normalizedSeed();
  setBusy(true);
  try {
    state.deal = await window.doudizhu.deal({
      seed: state.seed,
      viewer: state.viewer,
    });
    render();
    elements.statusText.textContent = '已发牌';
  } catch (error) {
    elements.statusText.textContent = error.message;
  } finally {
    setBusy(false);
  }
}

function render() {
  const deal = state.deal;
  elements.roundMeta.textContent = `Seed ${deal.seed}`;
  const viewer = deal.players[deal.viewer];
  elements.viewerText.textContent = `玩家 ${deal.viewer} 视角 · ${roleLabel(viewer.role)}`;
  renderBottomCards(deal.bottom_cards);

  const orderedPlayers = tableOrder(deal.viewer);
  for (const [slot, playerId] of orderedPlayers.entries()) {
    const player = deal.players[playerId];
    renderPlayer(elements.players[slot], player, playerId === deal.viewer, deal.bottom_cards);
  }
}

function renderBottomCards(cards) {
  elements.bottomCards.replaceChildren(...cards.map((card) => cardElement(card)));
}

function renderPlayer(container, player, isSelf, bottomCards) {
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
        }),
      ),
    );
  } else {
    const visibleBacks = Math.min(player.hand_count, 7);
    for (let index = 0; index < visibleBacks; index += 1) {
      const card = document.createElement('div');
      card.className = 'card card-back';
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
  card.append(textNode('strong', parsed.rank), textNode('span', parsed.suit));
  card.setAttribute('aria-label', code);
  if (options.isBottomCard) {
    card.dataset.source = 'bottom';
    card.append(textNode('em', '底'));
  }
  return card;
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

function setBusy(isBusy) {
  elements.dealButton.disabled = isBusy;
  elements.dealButton.textContent = isBusy ? '发牌中' : '发牌';
}

function setActiveViewer() {
  for (const tab of elements.viewerTabs) {
    tab.classList.toggle('active', Number(tab.dataset.viewer) === state.viewer);
  }
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
  return roleLabels[role] || role;
}
