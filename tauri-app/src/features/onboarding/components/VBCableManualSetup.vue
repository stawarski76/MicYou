<script setup lang="ts">
import { useI18n } from 'vue-i18n';

const { t } = useI18n();
defineEmits<{ download: []; recheck: [] }>();
</script>

<template>
  <div class="space-y-2">
    <p role="status" class="text-sm text-on-surface-variant">{{ t('vbcableDetect.manualSetupNotice') }}</p>
    <button type="button" @click="$emit('download')" class="w-full py-3 bg-primary rounded-xl text-sm font-bold text-on-primary">
      {{ t('vbcableDetect.manualDownload') }}
    </button>
    <button type="button" @click="$emit('recheck')" class="w-full py-3 bg-surface-variant rounded-xl text-sm font-bold">
      {{ t('vbcableDetect.checkAgain') }}
    </button>
  </div>
</template>
