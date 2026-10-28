<script setup lang="ts">
import { ref } from 'vue';
import { useI18n } from 'vue-i18n';

const props = defineProps<{ revealCode: () => Promise<string>; rotateCode: () => Promise<string> }>();
const { t } = useI18n();
const code = ref('');
const busy = ref(false);
const failed = ref(false);
const copied = ref(false);
async function reveal(rotate = false) {
  busy.value = true; failed.value = false; copied.value = false;
  try { code.value = await (rotate ? props.rotateCode() : props.revealCode()); }
  catch { code.value = ''; failed.value = true; }
  finally { busy.value = false; }
}
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
      <textarea :value="code" readonly rows="3" :aria-label="t('securePairing.code')" class="w-full rounded-lg bg-surface-container p-2 font-mono text-xs break-all select-all" />
      <button type="button" @click="copy" class="rounded-xl bg-surface-variant px-4 py-2">
        {{ t(copied ? 'securePairing.copied' : 'securePairing.copy') }}
      </button>
      <button type="button" @click="code = ''" class="px-4 py-2">{{ t('securePairing.hide') }}</button>
    </div>
    <p class="text-xs text-on-surface-variant">{{ t('securePairing.rotateNotice') }}</p>
    <button type="button" :disabled="busy" @click="reveal(true)" class="rounded-xl bg-surface-variant px-4 py-2 disabled:opacity-50">
      {{ t('securePairing.rotate') }}
    </button>
    <p v-if="failed" role="alert" class="text-sm text-error">{{ t('securePairing.failed') }}</p>
  </section>
</template>
