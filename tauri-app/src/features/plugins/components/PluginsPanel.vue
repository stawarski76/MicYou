<script setup lang="ts">
import { useI18n } from 'vue-i18n';
const { t } = useI18n();
</script>

<template>
  <section class="p-6 space-y-3" role="status">
    <h2 class="text-lg font-semibold">{{ t('plugins.title') }}</h2>
    <p class="text-sm text-on-surface-variant">{{ t('plugins.securityDisabled') }}</p>
  </section>
</template>