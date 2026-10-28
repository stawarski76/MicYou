import { expect, mock, test } from 'bun:test';
import { createRenderer, nextTick } from 'vue';
import { createI18n } from 'vue-i18n';
import en from '../../src/shared/locales/en.json';

let installed = false;
const commands: string[] = [];
const urls: string[] = [];
mock.module('../../src/platform', () => ({
  command: async (name: string) => {
    commands.push(name);
    if (name !== 'check_vbcable') throw new Error(`Unexpected command: ${name}`);
    return installed;
  },
  openUrl: async (url: string) => { urls.push(url); },
}));
mock.module('../../src/shared/lib/os', () => ({ isWindows: true }));
const { default: OnboardingWizard } = await import('../../src/features/onboarding/components/OnboardingWizard.vue');

// Exercise Vue's real component/event lifecycle without a browser or Tauri.
type Node = { text: string; tag?: string; props: Record<string, any>; children: Node[]; parent?: Node };
const node = (text = '', tag?: string): Node => ({ text, tag, props: {}, children: [] });
const renderer = createRenderer<Node, Node>({
  createElement: tag => node('', tag), createText: text => node(text), createComment: () => node(),
  setText: (n, text) => { n.text = text; }, setElementText: (n, text) => { n.text = text; n.children = []; },
  patchProp: (n, key, _old, value) => { n.props[key] = value; },
  insert: (n, parent, anchor) => {
    if (n.parent) n.parent.children.splice(n.parent.children.indexOf(n), 1);
    const i = anchor ? parent.children.indexOf(anchor) : -1;
    parent.children.splice(i < 0 ? parent.children.length : i, 0, n); n.parent = parent;
  },
  remove: n => { if (n.parent) n.parent.children.splice(n.parent.children.indexOf(n), 1); },
  parentNode: n => n.parent ?? null,
  nextSibling: n => n.parent?.children[n.parent.children.indexOf(n) + 1] ?? null,
});
const text = (n: Node): string => n.text + n.children.map(text).join('');
const all = (n: Node): Node[] => [n, ...n.children.flatMap(all)];

test('Windows onboarding reaches manual setup, opens vendor page and detects manual installation', async () => {
  installed = false; commands.length = 0; urls.length = 0;
  const root = node();
  const app = renderer.createApp(OnboardingWizard, { visible: true });
  app.use(createI18n({ legacy: false, locale: 'en', messages: { en } }));
  app.mount(root);
  const flush = async () => { await Promise.resolve(); await nextTick(); };
  const click = async (label: string) => {
    const button = all(root).find(n => n.tag === 'button' && text(n).trim() === label);
    expect(button).toBeDefined();
    await button!.props.onClick(); await flush();
  };
  try {
    await flush();
    await click(en.onboarding.next);
    expect(text(root)).toContain(en.vbcableDetect.manualSetupNotice);
    expect(text(root)).not.toContain(en.vbcableDetect.autoInstall);
    await click(en.vbcableDetect.manualDownload);
    expect(urls).toEqual(['https://vb-audio.com/Cable/']);
    installed = true;
    await click(en.vbcableDetect.checkAgain);
    expect(text(root)).toContain(en.onboarding.startUsing);
    expect(commands).toEqual(['check_vbcable', 'check_vbcable']);
  } finally { app.unmount(); }
});
