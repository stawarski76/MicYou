<script setup lang="ts">
defineProps<{ content: string }>();
</script>

<template>
  <!-- Remote plugin descriptions are untrusted data, never application HTML. -->
  <pre class="whitespace-pre-wrap break-words font-sans text-sm leading-relaxed">{{ content }}</pre>
</template>