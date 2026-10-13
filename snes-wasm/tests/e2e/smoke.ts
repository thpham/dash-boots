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
  const audioRequests: string[] = [];
  page.on('request', (r) => {
    if (r.url().includes('/audio/')) audioRequests.push(r.url());
  });
  const problems: string[] = [];
  page.on('console', (m) => {
    if (m.type() === 'error' || m.type() === 'warning') problems.push(`${m.type()}: ${m.text()}`);
  });
  page.on('pageerror', (e) => problems.push(`pageerror: ${e.message}`));

  await page.goto(URL);
  await page.waitForSelector('#game:not([hidden])', { timeout: 10_000 });
  await sleep(500);
  check(
    audioRequests.length === 0,
    `audio must be synthesized, not fetched${audioRequests.length ? `: ${audioRequests.join(', ')}` : ''}`,
  );
  const canvas = page.locator('#game');
  const shot = (name: string) => canvas.screenshot({ path: `${OUT}/${name}.png` });

  const dims = await canvas.evaluate((c: HTMLCanvasElement) => [c.width, c.height, c.clientWidth, c.clientHeight]);
  check(dims[0] % 320 === 0 && dims[0] / 320 === dims[1] / 180, `canvas is an integer multiple of 320x180 (${dims[0]}x${dims[1]})`);
  check(dims[2] === dims[0] && dims[3] === dims[1], 'CSS size matches device pixels at dpr 1');

  const title = await shot('01-title');

  // Volume keys: G/H for the music, V/B for the SFX (plain letter keys — no
  // shift combos). The toast is the observable feedback (WebAudio gains are
  // not exposed to the DOM).
  await page.keyboard.press('KeyG');
  await page.waitForSelector('#toast.show', { timeout: 2_000 });
  check(/Music \d+%/.test(await page.locator('#toast').textContent() ?? ''), 'G/H adjusts the music volume (toast feedback)');
  await page.keyboard.press('KeyB');
  await page.waitForFunction(() => /Effects \d+%/.test(document.querySelector('#toast')?.textContent ?? ''), { timeout: 2_000 });
  check(true, 'V/B adjusts the SFX volume (no shift)');

  // Render-ahead: the boot level's track gets pre-rendered while the title
  // screen idles, so entering the level plays instantly (debug hook).
  await page.waitForFunction(
    () => {
      const h = (window as unknown as { __dashboots?: { audio?: { debugMusicCache(): number[] } } }).__dashboots;
      return !!h?.audio && h.audio.debugMusicCache().includes(0);
    },
    { timeout: 30_000 },
  );
  check(true, 'boot level track pre-rendered during the title screen (render-ahead)');

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

  // C toggles the CRT filter: still paused, so only the filter can change the frame.
  await page.keyboard.press('c');
  await sleep(200);
  const crtToggled = await shot('06-crt-toggled');
  check(!paused2.equals(crtToggled), 'C toggles the CRT filter in the past');
  await page.keyboard.press('c');
  await sleep(200);

  const meanLuma = (png: Buffer) =>
    page.evaluate(async (b64) => {
      const img = new Image();
      img.src = `data:image/png;base64,${b64}`;
      await img.decode();
      const c = document.createElement('canvas');
      c.width = img.width;
      c.height = img.height;
      const ctx = c.getContext('2d')!;
      ctx.drawImage(img, 0, 0);
      const d = ctx.getImageData(0, 0, c.width, c.height).data;
      let sum = 0;
      for (let i = 0; i < d.length; i += 4) sum += 0.2126 * d[i] + 0.7152 * d[i + 1] + 0.0722 * d[i + 2];
      return sum / (d.length / 4);
    }, png.toString('base64'));

  const glError = () =>
    page.evaluate(() => (document.querySelector('#game') as HTMLCanvasElement).getContext('webgl2')!.getError());

  // Rift scenario on the tiny test level.
  await page.goto(`${URL}?level=test_rift`);
  await page.waitForSelector('#game:not([hidden])', { timeout: 10_000 });
  await sleep(300);
  await page.keyboard.press('Enter');
  await sleep(200);
  const beforeRift = await shot('10-before-rift');
  await page.keyboard.down('ArrowRight');
  await sleep(700); // into the rift and mid-pivot
  const midShift = await shot('11-mid-shift');
  // Resize during the shift: targets must follow without errors (Review Focus).
  await page.setViewportSize({ width: 1280, height: 720 });
  await sleep(900);
  await page.keyboard.up('ArrowRight');
  const future = await shot('12-future');
  check(!beforeRift.equals(midShift) && !midShift.equals(future), 'the rift transforms the image over time');
  const lumaBefore = await meanLuma(beforeRift);
  const lumaFuture = await meanLuma(future);
  check(lumaFuture < lumaBefore * 0.5, `the future is a dark void (luma ${lumaBefore.toFixed(0)} -> ${lumaFuture.toFixed(0)})`);
  check((await glError()) === 0, 'no WebGL error after the shift and a resize');
  const t0 = await shot('13-future-a');
  await sleep(300);
  const t1 = await shot('14-future-b');
  check(!t0.equals(t1), 'game loop still running in the future (motes/scanlines animate)');

  // The way back: keep walking right through the second rift (column 30, well
  // before the goal at 56) until the world flips back to the retro look, then
  // let the 1.8 s pivot finish and confirm the past is back.
  await page.keyboard.down('ArrowRight');
  let lumaBack = 0;
  for (let i = 0; i < 20; i++) {
    await sleep(400);
    const png = await canvas.screenshot({ path: `${OUT}/15-walking-back.png` });
    lumaBack = await meanLuma(png);
    if (lumaBack > lumaFuture * 2) break;
  }
  await page.keyboard.up('ArrowRight');
  await sleep(2000); // ToPast takes 1.8 s
  const backToPast = await shot('15-back-to-past');
  lumaBack = await meanLuma(backToPast);
  check(lumaBack > lumaFuture * 2, `the past is back (luma ${lumaFuture.toFixed(0)} -> ${lumaBack.toFixed(0)})`);
  check((await glError()) === 0, 'no WebGL error after the way back');

  // ENDLESS: a shared seed reproduces the run's first level exactly.
  const levelHashFor = async (url: string) => {
    await page.goto(url);
    await page.waitForSelector('#game:not([hidden])', { timeout: 10_000 });
    await sleep(300);
    await page.keyboard.press('Enter'); // ENDLESS is preselected by ?seed=
    await page.waitForFunction(() => !!(window as any).__dashboots?.levelHash, null, { timeout: 10_000 });
    await sleep(500);
    return page.evaluate(() => (window as any).__dashboots as { levelHash: string; levelSource: string });
  };
  const runA = await levelHashFor(`${URL}?seed=4242&start=2`);
  const endlessShot = await shot('20-endless');
  check(!title.equals(endlessShot), `ENDLESS starts a generated level (${runA.levelSource})`);
  check(/^model:\d in \d+ ms$/.test(runA.levelSource), `the embedded model wrote it (${runA.levelSource})`);
  const runB = await levelHashFor(`${URL}?seed=4242&start=2`);
  check(runA.levelHash === runB.levelHash, `?seed= reproduces the level (${runA.levelHash})`);

  // Junk in the URL and in storage must not break the boot (Review Focus 2–3).
  await page.evaluate(() => localStorage.setItem('dash-boots-run', '{"seed":"x"'));
  await page.goto(`${URL}?seed=abc&start=9`);
  await page.waitForSelector('#game:not([hidden])', { timeout: 10_000 });
  await sleep(300);
  check(true, 'boot survives a corrupt saved run and a junk ?seed=');
  await page.evaluate(() => localStorage.removeItem('dash-boots-run'));

  check(problems.length === 0, `no console errors/warnings${problems.length ? `:\n${problems.join('\n')}` : ''}`);
  await browser.close();
  console.log(`screenshots in ${OUT}/`);
} finally {
  server.kill();
}
