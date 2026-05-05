<template>
  <el-popover
    placement="bottom-end"
    trigger="click"
    popper-class="theme-popover"
    :width="384"
  >
    <template #reference>
      <el-button class="theme-trigger" :icon="Brush" circle />
    </template>
    <div class="theme-selector-grid">
      <button
        v-for="themeItem in THEMES"
        :key="themeItem.id"
        type="button"
        class="theme-option"
        :class="{ 'is-active': themeItem.id === normalizedModelValue }"
        @click="selectTheme(themeItem.id)"
      >
        <span class="theme-option-color" :style="{ background: themeItem.accent, color: themeItem.accent }"></span>
        <span class="theme-option-name">{{ themeItem.name }}</span>
        <span class="theme-option-mode">{{ themeItem.isDark ? '深色' : '浅色' }}</span>
      </button>
    </div>
  </el-popover>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { Brush } from '@element-plus/icons-vue';
import { THEMES, normalizeThemeId, type ThemeId } from '@/theme';

const props = defineProps<{
  modelValue: string;
}>();

const emit = defineEmits<{
  'update:modelValue': [value: ThemeId];
  change: [value: ThemeId];
}>();

const normalizedModelValue = computed(() => normalizeThemeId(props.modelValue));

function selectTheme(themeId: ThemeId) {
  emit('update:modelValue', themeId);
  emit('change', themeId);
}
</script>
