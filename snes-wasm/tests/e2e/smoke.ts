// Headless browser smoke test: boots the production build, plays a few seconds
// and checks there are no errors. Run with `just e2e` (needs a Chromium; the
// nix devShell provides one on Linux, or set CHROMIUM_PATH).

import { execFileSync, spawn } from 'node:child_process';
import { mkdirSync } from 'node:fs';
import { chromium } from 'playwright-core';

const PORT = 4173;
const URL = `http://localhost:${PORT}/`;
const OUT = 'test-results';

function chromiumPath(): string {
  if (process.env.CHROMIUM_PATH) return process.env.CHROMIUM_PATH;
  for (const bin of ['chromium', 'chromium-browser', 'google-chrome']) {
    try {
      return execFileSync('which', [bin], { encoding: 'utf8' }).trim();
    } catch {
      // try next
    }
  }
  throw new Error('No Chromium found: set CHROMIUM_PATH or run inside `nix develop`.');
}

function check(cond: unknown, msg: string): asserts cond {
  if (!cond) throw new Error(`E2E FAILED: ${msg}`);
  console.log(`ok - ${msg}`);
}

const server = spawn('npx', ['vite', 'preview', '--port', String(PORT), '--strictPort'], { stdio: 'ignore' });
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

try {
  mkdirSync(OUT, { recursive: true });
  for (let i = 0; i < 50; i++) {
    try {
      if ((await fetch(URL)).ok) break;
    } catch {
      await sleep(100);
    }
  }

  const browser = await chromium.launch({
    executablePath: chromiumPath(),
    args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'],
  });
  const page = await browser.newPage({ viewport: { width: 960, height: 540 } });
  const problems: string[] = [];
  page.on('console', (m) => {
    if (m.type() === 'error' || m.type() === 'warning') problems.push(`${m.type()}: ${m.text()}`);
  });
  page.on('pageerror', (e) => problems.push(`pageerror: ${e.message}`));

  await page.goto(URL);
  await page.waitForSelector('#game:not([hidden])', { timeout: 10_000 });
  await sleep(500);
  const canvas = page.locator('#game');
  const shot = (name: string) => canvas.screenshot({ path: `${OUT}/${name}.png` });

  const dims = await canvas.evaluate((c: HTMLCanvasElement) => [c.width, c.height, c.clientWidth, c.clientHeight]);
  check(dims[0] === 320 && dims[1] === 180, 'canvas renders at native 320x180');
  check(dims[2] % 320 === 0 && dims[2] / 320 === dims[3] / 180, `integer CSS scaling (${dims[2]}x${dims[3]})`);

  const title = await shot('01-title');
  await page.keyboard.press('Enter');
  await sleep(300);
  const start = await shot('02-playing');
  check(!title.equals(start), 'Enter leaves the title screen');

  await page.keyboard.down('ArrowRight');
  await sleep(700);
  await page.keyboard.down('Space');
  await sleep(250);
  await page.keyboard.up('Space');
  await sleep(600);
  await page.keyboard.up('ArrowRight');
  const moved = await shot('03-moved');
  check(!moved.equals(start), 'input moves the game');

  await page.keyboard.press('Enter');
  await sleep(200);
  const paused1 = await shot('04-paused');
  await sleep(400);
  const paused2 = await shot('05-paused-later');
  check(paused1.equals(paused2), 'pause freezes the frame');

  check(problems.length === 0, `no console errors/warnings${problems.length ? `:\n${problems.join('\n')}` : ''}`);
  await browser.close();
  console.log(`screenshots in ${OUT}/`);
} finally {
  server.kill();
}
