const path = require('node:path');
const { test, expect } = require('@playwright/test');
const { _electron: electron } = require('playwright');

const projectRoot = path.resolve(__dirname, '../..');

async function playFirstSelfCard(page) {
  await expect(page.locator('#playButton')).toBeVisible();
  const firstCard = page.locator('#player0 .card[aria-label]').first();
  const card = await firstCard.getAttribute('aria-label');

  await firstCard.click({ position: { x: 6, y: 8 } });
  await page.getByRole('button', { name: '出牌' }).click();
  return card;
}

function seedFromMeta(meta) {
  return Number(meta.match(/Seed (\d+)/)?.[1]);
}

test.describe('Electron auto-play table', () => {
  let app;
  let page;

  test.beforeAll(async () => {
    app = await electron.launch({
      args: [path.join(projectRoot, 'electron/main.js')],
      cwd: projectRoot,
      env: { ...process.env, E2E_TEST: '1' },
    });
    page = await app.firstWindow();
    await page.locator('#statusText').getByText('已开局').waitFor();
  });

  test.afterAll(async () => {
    await app.close();
  });

  test.beforeEach(async () => {
    await page.getByRole('button', { name: '玩家 0' }).click();
    await expect(page.locator('#viewerText')).toContainText('玩家 0');
    const meta = await page.locator('#roundMeta').textContent();
    await page.getByRole('button', { name: '重新发牌' }).click();
    await page.waitForFunction(
      (old) => document.querySelector('#roundMeta')?.textContent !== old,
      meta,
    );
    await expect(page.locator('#statusText')).toHaveText('已开局');
  });

  test('starts the initial player view through startGame', async () => {
    await expect(page.locator('#roundMeta')).toHaveText(/Seed \d+ · game-/);
    await expect(page.locator('#viewerText')).toHaveText('玩家 0 视角 · 地主');
    await expect(page.locator('#currentPlayerText')).toHaveText('轮到你出牌');
    await expect(page.locator('#winnerOverlay')).toBeHidden();

    await expect(page.locator('#player0 .seat-header h2')).toHaveText('玩家 0');
    await expect(page.locator('#player0 .role-badge')).toHaveText('地主');
    await expect(page.locator('#player0 .seat-header')).toContainText('自己');
    await expect(page.locator('#player0 .seat-header')).toContainText('20 张');
    await expect(page.locator('#player0 .card[aria-label]')).toHaveCount(20);
    const handRows = await page.locator('#player0 .hand-cards .card[aria-label]').evaluateAll((cards) => [
      ...new Set(cards.map((card) => Math.round(card.getBoundingClientRect().top))),
    ]);
    expect(handRows).toHaveLength(1);
    await expect(page.locator('#player0 .bottom-owned-card')).toHaveCount(3);

    await expect(page.locator('#player1 .role-badge')).toHaveText('农民');
    await expect(page.locator('#player1 .card[aria-label]')).toHaveCount(0);
    await expect(page.locator('#player2 .role-badge')).toHaveText('农民');
    await expect(page.locator('#player2 .card[aria-label]')).toHaveCount(0);
    await expect(page.locator('#bottomCards .card[aria-label]')).toHaveCount(3);
  });

  test('switches viewer through setViewer without redealing', async () => {
    const gameMeta = await page.locator('#roundMeta').textContent();

    await page.getByRole('button', { name: '玩家 1' }).click();
    await expect(page.locator('#statusText')).toHaveText('已切换视角');
    await expect(page.locator('#roundMeta')).toHaveText(gameMeta);
    await expect(page.locator('#viewerText')).toHaveText('玩家 1 视角 · 农民');

    await expect(page.locator('#player0 .seat-header h2')).toHaveText('玩家 1');
    await expect(page.locator('#player0 .role-badge')).toHaveText('农民');
    await expect(page.locator('#player0 .seat-header')).toContainText('自己');
    await expect(page.locator('#player0 .seat-header')).toContainText('17 张');
    await expect(page.locator('#player0 .card[aria-label]')).toHaveCount(17);
    await expect(page.locator('#player0 .bottom-owned-card')).toHaveCount(0);

    await expect(page.locator('#player1 .seat-header h2')).toHaveText('玩家 2');
    await expect(page.locator('#player1 .seat-header')).toContainText('队友');
    await expect(page.locator('#player1 .card[aria-label]')).toHaveCount(0);

    await expect(page.locator('#player2 .seat-header h2')).toHaveText('玩家 0');
    await expect(page.locator('#player2 .seat-header')).toContainText('对手');
    await expect(page.locator('#player2 .card[aria-label]')).toHaveCount(0);
  });

  test('shows player 2 identity and ally from player 2 view', async () => {
    await page.getByRole('button', { name: '玩家 2' }).click();
    await expect(page.locator('#viewerText')).toHaveText('玩家 2 视角 · 农民');

    await expect(page.locator('#player0 .seat-header h2')).toHaveText('玩家 2');
    await expect(page.locator('#player0 .role-badge')).toHaveText('农民');
    await expect(page.locator('#player0 .seat-header')).toContainText('自己');
    await expect(page.locator('#player0 .card[aria-label]')).toHaveCount(17);

    await expect(page.locator('#player1 .seat-header h2')).toHaveText('玩家 0');
    await expect(page.locator('#player1 .role-badge')).toHaveText('地主');
    await expect(page.locator('#player1 .seat-header')).toContainText('对手');

    await expect(page.locator('#player2 .seat-header h2')).toHaveText('玩家 1');
    await expect(page.locator('#player2 .role-badge')).toHaveText('农民');
    await expect(page.locator('#player2 .seat-header')).toContainText('队友');
  });

  test('highlights hinted cards only in the current viewer hand', async () => {
    await page.getByRole('button', { name: '提示' }).click();
    await expect(page.locator('#statusText')).toHaveText('AI推荐');
    await page.waitForFunction(() => document.querySelectorAll('#player0 .selected-card[aria-label]').length > 0);
    await expect(page.locator('#player1 .selected-card')).toHaveCount(0);
    await expect(page.locator('#player2 .selected-card')).toHaveCount(0);
  });

  test('manual play appends public history and updates play zone', async () => {
    await expect(page.locator('#historyList .history-item')).toHaveCount(0);

    const manualCard = await playFirstSelfCard(page);

    await expect(page.locator('#historyList .history-item').last()).toContainText('玩家 0 出牌 Single');
    await expect(page.locator('#play0 .card[aria-label]')).toHaveCount(1);
    await expect(page.locator(`#play0 .card[aria-label="${manualCard}"]`)).toHaveCount(1);
  });

  test('preserves manual play when autoplay advances later turns', async () => {
    const manualCard = await playFirstSelfCard(page);

    await page.waitForFunction(() => document.querySelectorAll('#historyList .history-item').length >= 2, null, {
      timeout: 15_000,
    });
    await expect(page.locator('#play0 .card[aria-label]')).toHaveCount(1);
    await expect(page.locator(`#play0 .card[aria-label="${manualCard}"]`)).toHaveCount(1);
    await expect(page.locator('#historyList')).toContainText(manualCard);
  });

  test('autoplay grows public history after a manual play', async () => {
    await playFirstSelfCard(page);

    await page.waitForFunction(() => document.querySelectorAll('#historyList .history-item').length >= 2, null, {
      timeout: 15_000,
    });
    await expect(page.locator('#autoplayToggle')).not.toBeChecked();
  });

  test('autoplay stops at the next user decision after a manual play', async () => {
    await playFirstSelfCard(page);

    await expect(page.locator('#currentPlayerText')).toHaveText('轮到你出牌', {
      timeout: 45_000,
    });
    await expect(page.locator('#autoplayToggle')).not.toBeChecked();
    await expect(page.locator('#playButton')).toBeVisible();
  });

  test('redeals a different random seed from the toolbar', async () => {
    const initialMeta = await page.locator('#roundMeta').textContent();
    const initialSeed = seedFromMeta(initialMeta);

    await page.locator('#seedInput').fill('43');
    await page.getByRole('button', { name: '重新发牌' }).click();

    await page.waitForFunction((meta) => document.querySelector('#roundMeta').textContent !== meta, initialMeta);
    await expect(page.locator('#roundMeta')).toHaveText(/Seed \d+ · game-/);
    const nextSeed = seedFromMeta(await page.locator('#roundMeta').textContent());
    expect(nextSeed).not.toBe(initialSeed);
    expect(nextSeed).not.toBe(43);
    await expect(page.locator('#bottomCards .card[aria-label]')).toHaveCount(3);
  });

  test('does not leak hidden hands through non-self seat text or aria labels', async () => {
    await expect(page.locator('#player1 .card[aria-label], #player2 .card[aria-label]')).toHaveCount(0);
    await expect(page.locator('#player1 .card-back, #player2 .card-back')).toHaveCount(14);

    const leakedCodes = await page.locator('#player1 .cards, #player2 .cards').evaluateAll((nodes) => {
      const pattern = /\b(?:10|[3-9JQKA2])[CDHS]\b|[BR]J/g;
      return nodes.flatMap((node) => node.textContent.match(pattern) || []);
    });
    expect(leakedCodes).toEqual([]);
  });
});
