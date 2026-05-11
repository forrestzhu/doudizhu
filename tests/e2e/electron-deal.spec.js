const path = require('node:path');
const { test, expect } = require('@playwright/test');
const { _electron: electron } = require('playwright');

const projectRoot = path.resolve(__dirname, '../..');

test.describe('Electron deal table', () => {
  let app;
  let page;

  test.beforeEach(async () => {
    app = await electron.launch({
      args: [path.join(projectRoot, 'electron/main.js')],
      cwd: projectRoot,
    });
    page = await app.firstWindow();
    await page.locator('#statusText').getByText('已发牌').waitFor();
  });

  test.afterEach(async () => {
    await app.close();
  });

  test('renders the initial player view from the Rust deal endpoint', async () => {
    await expect(page.locator('#roundMeta')).toHaveText('Seed 42');
    await expect(page.locator('#viewerText')).toHaveText('玩家 0 视角 · 地主');

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

  test('switches viewer without exposing non-viewer hands', async () => {
    await page.getByRole('button', { name: '玩家 1' }).click();
    await expect(page.locator('#viewerText')).toHaveText('玩家 1 视角 · 农民');

    await expect(page.locator('#player0 .seat-header h2')).toHaveText('玩家 1');
    await expect(page.locator('#player0 .role-badge')).toHaveText('农民');
    await expect(page.locator('#player0 .seat-header')).toContainText('自己');
    await expect(page.locator('#player0 .seat-header')).toContainText('17 张');
    await expect(page.locator('#player0 .card[aria-label]')).toHaveCount(17);
    await expect(page.locator('#player0 .bottom-owned-card')).toHaveCount(0);

    await expect(page.locator('#player1 .seat-header h2')).toHaveText('玩家 2');
    await expect(page.locator('#player1 .role-badge')).toHaveText('农民');
    await expect(page.locator('#player1 .seat-header')).toContainText('队友');
    await expect(page.locator('#player1 .card[aria-label]')).toHaveCount(0);

    await expect(page.locator('#player2 .seat-header h2')).toHaveText('玩家 0');
    await expect(page.locator('#player2 .role-badge')).toHaveText('地主');
    await expect(page.locator('#player2 .seat-header')).toContainText('对手');
    await expect(page.locator('#player2 .card[aria-label]')).toHaveCount(0);
  });

  test('shows player 2 as a farmer and player 1 as ally from player 2 view', async () => {
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

  test('deals a different deterministic seed from the toolbar', async () => {
    await page.locator('#seedInput').fill('43');
    await page.getByRole('button', { name: '发牌' }).click();

    await expect(page.locator('#roundMeta')).toHaveText('Seed 43');
    await expect(page.locator('#bottomCards .card[aria-label="6H"]')).toHaveCount(1);
    await expect(page.locator('#bottomCards .card[aria-label="7D"]')).toHaveCount(1);
    await expect(page.locator('#bottomCards .card[aria-label="8C"]')).toHaveCount(1);
  });
});
