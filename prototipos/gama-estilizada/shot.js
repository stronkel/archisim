const { chromium } = require('playwright');
(async () => {
  const shots = JSON.parse(process.argv[2] || '[]');
  const browser = await chromium.launch({ args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'] });
  const page = await browser.newPage({ viewport: { width: 1400, height: 820 } });
  page.on('console', (m) => console.log('console:', m.text()));
  page.on('pageerror', (e) => console.log('pageerror:', e.message));
  await page.goto('file://' + __dirname + '/out/index.html?still');
  await page.waitForTimeout(4000);
  for (const s of shots) {
    await page.evaluate((s) => {
      if (s.view) window.__set.view(s.view);
      if (s.light) window.__set.light(s.light);
      if (s.style !== undefined) window.__set.style(s.style);
      if (s.cam) window.__set.cam(s.cam[0], s.cam[1]);
    }, s);
    await page.evaluate(() => window.__set.render());
    await page.screenshot({ path: 'out/' + s.name + '.png' });
    console.log('shot', s.name);
  }
  await browser.close();
})();
