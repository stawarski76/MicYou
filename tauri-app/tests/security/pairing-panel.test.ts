import { expect, mock, test } from 'bun:test';
import { createRenderer, nextTick } from 'vue';
import { createI18n } from 'vue-i18n';
import PairingPanel from '../../src/features/connection/components/PairingPanel.vue';
import en from '../../src/shared/locales/en.json';

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
const TEST_CODE = `ymic1:${'11'.repeat(32)}:${'42'.repeat(32)}`;

test('pairing credential stays hidden until requested and can be hidden again', async () => {
  const root = node(); const reveal = mock(async () => TEST_CODE); const rotate = mock(async () => TEST_CODE);
  const app = renderer.createApp(PairingPanel, { revealCode: reveal, rotateCode: rotate });
  app.use(createI18n({ legacy: false, locale: 'en', messages: { en } })); app.mount(root);
  const click = async (label: string) => {
    const button = all(root).find(n => n.tag === 'button' && text(n).trim() === label);
    expect(button).toBeDefined(); await button!.props.onClick(); await nextTick();
  };
  try {
    expect(reveal).not.toHaveBeenCalled(); expect(rotate).not.toHaveBeenCalled();
    expect(all(root).find(n => n.tag === 'textarea')).toBeUndefined();
    expect(text(root)).toContain(en.securePairing.secretNotice);
    expect(text(root)).toContain(en.securePairing.rotateNotice);
    await click(en.securePairing.reveal);
    expect(reveal).toHaveBeenCalledTimes(1); expect(rotate).not.toHaveBeenCalled();
    expect(all(root).find(n => n.tag === 'textarea')!.props.value).toBe(TEST_CODE);
    await click(en.securePairing.hide);
    expect(all(root).find(n => n.tag === 'textarea')).toBeUndefined();
    await click(en.securePairing.rotate);
    expect(rotate).toHaveBeenCalledTimes(1);
  } finally { app.unmount(); }
});

test('pairing failures do not expose a previous credential or raw backend errors', async () => {
  const root = node();
  const app = renderer.createApp(PairingPanel, {
    revealCode: async () => TEST_CODE,
    rotateCode: async () => { throw new Error(`private backend detail: ${TEST_CODE}`); },
  });
  app.use(createI18n({ legacy: false, locale: 'en', messages: { en } })); app.mount(root);
  try {
    const click = async (label: string) => { await all(root).find(n => n.tag === 'button' && text(n).trim() === label)!.props.onClick(); await nextTick(); };
    await click(en.securePairing.reveal); await click(en.securePairing.rotate);
    expect(all(root).find(n => n.tag === 'textarea')).toBeUndefined();
    expect(text(root)).toContain(en.securePairing.failed);
    expect(text(root)).not.toContain(TEST_CODE); expect(text(root)).not.toContain('private backend detail');
  } finally { app.unmount(); }
});
