<script setup lang="ts">
import { onBeforeUnmount, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { pairingQrDataUrl } from '../utils/pairingQr';

const props = defineProps<{ revealCode: () => Promise<string>; rotateCode: () => Promise<string> }>();
const { t } = useI18n();
const code = ref('');
const qr = ref('');
let request = 0;
const busy = ref(false);
const failed = ref(false);
const copied = ref(false);
async function reveal(rotate = false) {
  if (busy.value) return;
  const current = ++request;
  code.value = ''; qr.value = '';
  busy.value = true; failed.value = false; copied.value = false;
  try {
    const nextCode = await (rotate ? props.rotateCode() : props.revealCode());
    if (current !== request) return;
    const nextQr = await pairingQrDataUrl(nextCode);
    if (current !== request) return;
    code.value = nextCode; qr.value = nextQr;
  }
  catch { if (current === request) { code.value = ''; qr.value = ''; failed.value = true; } }
  finally { if (current === request) busy.value = false; }
}
function hide() { ++request; code.value = ''; qr.value = ''; copied.value = false; busy.value = false; }
onBeforeUnmount(hide);
async function copy() {
  try { await navigator.clipboard.writeText(code.value); copied.value = true; }
  catch { failed.value = true; }
}
</script>

<template>
  <section class="bg-surface-bright rounded-2xl p-4 space-y-3">
    <h4 class="font-bold text-on-surface">{{ t('securePairing.title') }}</h4>
    <p class="text-sm text-on-surface-variant">{{ t('securePairing.description') }}</p>
    <p class="text-xs text-on-surface-variant">{{ t('securePairing.secretNotice') }}</p>
    <button type="button" :disabled="busy" @click="reveal(false)" class="rounded-xl bg-primary px-4 py-2 text-on-primary disabled:opacity-50">
      {{ t('securePairing.reveal') }}
    </button>
    <div v-if="code" class="space-y-2">
      <p class="text-sm text-on-surface-variant">{{ t('securePairing.qrHint') }}</p>
      <img :src="qr" :alt="t('securePairing.qrAlt')" width="280" height="280" class="mx-auto max-w-full rounded-lg bg-white" draggable="false" />
      <textarea :value="code" readonly rows="3" :aria-label="t('securePairing.code')" class="w-full rounded-lg bg-surface-container p-2 font-mono text-xs break-all select-all" />
      <button type="button" @click="copy" class="rounded-xl bg-surface-variant px-4 py-2">
        {{ t(copied ? 'securePairing.copied' : 'securePairing.copy') }}
      </button>
      <button type="button" @click="hide" class="px-4 py-2">{{ t('securePairing.hide') }}</button>
    </div>
    <p class="text-xs text-on-surface-variant">{{ t('securePairing.rotateNotice') }}</p>
    <button type="button" :disabled="busy" @click="reveal(true)" class="rounded-xl bg-surface-variant px-4 py-2 disabled:opacity-50">
      {{ t('securePairing.rotate') }}
    </button>
    <p v-if="failed" role="alert" class="text-sm text-error">{{ t('securePairing.failed') }}</p>
  </section>
</template>
