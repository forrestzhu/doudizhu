const path = require('node:path');
const { test, expect } = require('@playwright/test');
const { _electron: electron } = require('playwright');

const projectRoot = path.resolve(__dirname, '../..');

test.describe('Electron auto-play table', () => {
  let app;
  let page;

  test.beforeEach(async () => {
    app = await electron.launch({
      args: [path.join(projectRoot, 'electron/main.js')],
      cwd: projectRoot,
    });
    page = await app.firstWindow();
    await page.locator('#statusText').getByText('已开局').waitFor();
  });

  test.afterEach(async () => {
    await app.close();
  });

  test('starts the initial player view through startGame', async () => {
    await expect(page.locator('#roundMeta')).toContainText('Seed 42');
    await expect(page.locator('#roundMeta')).toContainText('game-');
    await expect(page.locator('#viewerText')).toHaveText('玩家 0 视角 · 地主');
    await expect(page.locator('#currentPlayerText')).toHaveText('当前玩家 0');
    await expect(page.locator('#winnerText')).toHaveText('胜者 未定');
    await expect(page.locator('#previousPlay')).toHaveText('上一手：无');

    await expect(page.locator('#player0 .seat-header h2')).toHaveText('玩家 0');
    await expect(page.locator('#player0 .role-badge')).toHaveText('地主');
    await expect(page.locator('#player0 .seat-header')).toContainText('自己');
    await expect(page.locator('#player0 .seat-header')).toContainText('20 张');
    await expect(page.locator('#player0 .card[aria-label]')).toHaveCount(20);
    await expect(page.locator('#player0 .bottom-owned-card')).toHaveCount(3);
    await expect(page.locator('#player0 .bottom-owned-card[aria-label="7D"]')).toHaveCount(1);
    await expect(page.locator('#player0 .bottom-owned-card[aria-label="QS"]')).toHaveCount(1);
    await expect(page.locator('#player0 .bottom-owned-card[aria-label="AS"]')).toHaveCount(1);

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
    await expect(page.locator('#statusText')).toContainText('提示');
    await expect(page.locator('#player0 .hinted-card[aria-label]')).toHaveCount(1);
    await expect(page.locator('#player0 .hinted-card[aria-label="3D"]')).toHaveCount(1);
    await expect(page.locator('#player1 .hinted-card')).toHaveCount(0);
    await expect(page.locator('#player2 .hinted-card')).toHaveCount(0);
  });

  test('single step appends public history and previous play', async () => {
    await expect(page.locator('#historyList .history-item')).toHaveCount(0);

    await page.getByRole('button', { name: '单步' }).click();
    await expect(page.locator('#statusText')).toHaveText('已单步');
    await expect(page.locator('#historyList .history-item')).toHaveCount(1);
    await expect(page.locator('#historyList')).toContainText('#1 玩家 0 出牌 Single');
    await expect(page.locator('#previousPlay')).toContainText('上一手：玩家 0 出牌 Single');
    await expect(page.locator('#currentPlayerText')).toHaveText('当前玩家 1');
  });

  test('autoplay grows public history', async () => {
    await page.locator('#autoplayToggle').check();
    await page.waitForFunction(() => document.querySelectorAll('#historyList .history-item').length >= 2, null, {
      timeout: 10_000,
    });
    await page.locator('#autoplayToggle').uncheck();
  });

  test('redeals a different deterministic seed from the toolbar', async () => {
    await page.locator('#seedInput').fill('43');
    await page.getByRole('button', { name: '重新发牌' }).click();

    await expect(page.locator('#roundMeta')).toContainText('Seed 43');
    await expect(page.locator('#bottomCards .card[aria-label="6H"]')).toHaveCount(1);
    await expect(page.locator('#bottomCards .card[aria-label="7D"]')).toHaveCount(1);
    await expect(page.locator('#bottomCards .card[aria-label="8C"]')).toHaveCount(1);
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
