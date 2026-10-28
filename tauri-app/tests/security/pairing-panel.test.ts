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
    expect(all(root).find(n => n.tag === 'img')).toBeUndefined();
    expect(text(root)).toContain(en.securePairing.secretNotice);
    expect(text(root)).toContain(en.securePairing.rotateNotice);
    await click(en.securePairing.reveal);
    expect(reveal).toHaveBeenCalledTimes(1); expect(rotate).not.toHaveBeenCalled();
    expect(all(root).find(n => n.tag === 'textarea')!.props.value).toBe(TEST_CODE);
    const qr = all(root).find(n => n.tag === 'img');
    expect(qr).toBeDefined();
    expect(qr!.props.src).toMatch(/^data:image\/png;base64,/);
    expect(qr!.props.alt).toBe(en.securePairing.qrAlt);
    await click(en.securePairing.hide);
    expect(all(root).find(n => n.tag === 'textarea')).toBeUndefined();
    expect(all(root).find(n => n.tag === 'img')).toBeUndefined();
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
    expect(all(root).find(n => n.tag === 'img')).toBeUndefined();
    expect(text(root)).toContain(en.securePairing.failed);
    expect(text(root)).not.toContain(TEST_CODE); expect(text(root)).not.toContain('private backend detail');
  } finally { app.unmount(); }
});

test('starting a renewal removes the previous pairing code and QR immediately', async () => {
  const root = node();
  let finish!: (code: string) => void;
  const renewal = new Promise<string>(resolve => { finish = resolve; });
  const app = renderer.createApp(PairingPanel, {
    revealCode: async () => TEST_CODE, rotateCode: () => renewal,
  });
  app.use(createI18n({ legacy: false, locale: 'en', messages: { en } })); app.mount(root);
  try {
    const button = (label: string) => all(root).find(n => n.tag === 'button' && text(n).trim() === label)!;
    await button(en.securePairing.reveal).props.onClick(); await nextTick();
    const pending = button(en.securePairing.rotate).props.onClick(); await nextTick();
    expect(all(root).find(n => n.tag === 'textarea')).toBeUndefined();
    expect(all(root).find(n => n.tag === 'img')).toBeUndefined();
    finish(`ymic1:${'22'.repeat(32)}:${'55'.repeat(32)}`); await pending; await nextTick();
    expect(all(root).find(n => n.tag === 'img')!.props.src).toMatch(/^data:image\/png;base64,/);
  } finally { app.unmount(); }
});

test('invalid backend pairing data is not rendered as a QR or exposed in errors', async () => {
  const root = node();
  const invalid = `https://untrusted.invalid/${TEST_CODE}`;
  const app = renderer.createApp(PairingPanel, { revealCode: async () => invalid, rotateCode: async () => invalid });
  app.use(createI18n({ legacy: false, locale: 'en', messages: { en } })); app.mount(root);
  try {
    await all(root).find(n => n.tag === 'button' && text(n).trim() === en.securePairing.reveal)!.props.onClick();
    await nextTick();
    expect(all(root).some(n => n.tag === 'img' || n.tag === 'textarea')).toBe(false);
    expect(text(root)).toContain(en.securePairing.failed);
    expect(text(root)).not.toContain(invalid);
  } finally { app.unmount(); }
});
