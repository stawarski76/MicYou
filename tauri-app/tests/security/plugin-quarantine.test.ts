import { expect, test } from 'bun:test';
import { createSSRApp } from 'vue';
import { renderToString } from 'vue/server-renderer';
import { createI18n } from 'vue-i18n';
import PluginsPanel from '../../src/features/plugins/components/PluginsPanel.vue';
import en from '../../src/shared/locales/en.json';

test('plugin settings show quarantine instead of mounting market and import controls', async () => {
  const app = createSSRApp(PluginsPanel);
  app.use(createI18n({ legacy: false, locale: 'en', messages: { en } }));
  const html = await renderToString(app);
  expect(html).toContain('Third-party plugins are temporarily unavailable');
  expect(html).toContain('role="status"');
  expect(html).not.toContain('<button');
  expect(html).not.toContain('<iframe');
});
