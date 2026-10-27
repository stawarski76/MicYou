import { expect, test } from 'bun:test';
import { createSSRApp } from 'vue';
import { renderToString } from 'vue/server-renderer';
import { createI18n } from 'vue-i18n';
import VBCableManualSetup from '../../src/features/onboarding/components/VBCableManualSetup.vue';
import en from '../../src/shared/locales/en.json';

test('manual driver setup explains the download and exposes a recheck action', async () => {
  const app = createSSRApp(VBCableManualSetup);
  app.use(createI18n({ legacy: false, locale: 'en', messages: { en } }));
  const html = await renderToString(app);
  expect(html).toContain('Automatic driver installation is temporarily unavailable');
  expect(html).toContain('role="status"');
  expect(html).toContain(en.vbcableDetect.manualDownload);
  expect(html).toContain('Check again');
  expect(html.match(/<button/g)).toHaveLength(2);
  expect(html).not.toContain(en.vbcableDetect.autoInstall);
});
