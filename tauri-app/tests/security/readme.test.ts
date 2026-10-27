import { describe, expect, test } from 'bun:test';
import { createSSRApp } from 'vue';
import { renderToString } from 'vue/server-renderer';
import PluginReadme from '../../src/features/plugins/components/PluginReadme.vue';

describe('untrusted plugin README', () => {
  test('shows the Markdown attribute-injection payload literally', async () => {
    const payload = '!["onerror="alert(1)](x)';
    const html = await renderToString(createSSRApp(PluginReadme, { content: payload }));
    expect(html).not.toContain('<img');
    expect(html).toContain('![&quot;onerror=&quot;alert(1)](x)');
  });

  test('does not create elements or active links from raw HTML', async () => {
    const payload = '<img src=x onerror=alert(1)>\n<a href="javascript:alert(1)">open</a>';
    const html = await renderToString(createSSRApp(PluginReadme, { content: payload }));
    expect(html).not.toContain('<img');
    expect(html).not.toContain('<a ');
    expect(html).toContain('&lt;img src=x onerror=alert(1)&gt;');
    expect(html).toContain('&lt;a href=&quot;javascript:alert(1)&quot;&gt;open&lt;/a&gt;');
  });

  test('preserves plain text and newlines without interpreting Markdown', async () => {
    const html = await renderToString(createSSRApp(PluginReadme, { content: '# Setup\nA & B\n**bold**' }));
    expect(html).toContain('# Setup\nA &amp; B\n**bold**');
    expect(html).not.toContain('<h1');
    expect(html).not.toContain('<strong');
  });
});
