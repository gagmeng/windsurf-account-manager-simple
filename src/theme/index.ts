export const DEFAULT_THEME_ID = 'aurora';

export const THEMES = [
  { id: 'original-light', name: '原始浅色', accent: '#409eff', isDark: false },
  { id: 'original-dark', name: '原始深色', accent: '#409eff', isDark: true },
  { id: 'aurora', name: '极光蓝紫', accent: '#7c3aed', isDark: false },
  { id: 'obsidian', name: '曜石黑金', accent: '#f59e0b', isDark: true },
  { id: 'sapphire', name: '深海蓝', accent: '#2563eb', isDark: true },
  { id: 'emerald', name: '翡翠绿', accent: '#10b981', isDark: false },
  { id: 'sunset', name: '日落橙', accent: '#f97316', isDark: false },
  { id: 'grape', name: '葡萄紫', accent: '#9333ea', isDark: true },
  { id: 'rose', name: '玫瑰粉', accent: '#e11d48', isDark: false },
  { id: 'cyberpunk', name: '赛博霓虹', accent: '#06b6d4', isDark: true },
  { id: 'ocean', name: '海洋青', accent: '#0891b2', isDark: false },
  { id: 'graphite', name: '石墨灰', accent: '#64748b', isDark: true },
  { id: 'gold', name: '香槟金', accent: '#d97706', isDark: false },
  { id: 'mint', name: '薄荷绿', accent: '#14b8a6', isDark: false },
  { id: 'lavender', name: '薰衣草', accent: '#8b5cf6', isDark: false },
  { id: 'glacier', name: '冰川白', accent: '#0ea5e9', isDark: false },
] as const;

export type ThemeId = typeof THEMES[number]['id'];

const themeIdSet = new Set<string>(THEMES.map(theme => theme.id));

export function normalizeThemeId(themeId: string | null | undefined): ThemeId {
  if (themeId === 'light') return 'original-light';
  if (themeId === 'dark') return 'original-dark';
  if (themeId && themeIdSet.has(themeId)) return themeId as ThemeId;
  return DEFAULT_THEME_ID;
}

export function getTheme(themeId: string | null | undefined) {
  const normalizedThemeId = normalizeThemeId(themeId);
  return THEMES.find(theme => theme.id === normalizedThemeId) ?? THEMES[0];
}

export function applyTheme(themeId: string | null | undefined): ThemeId {
  const theme = getTheme(themeId);
  const root = document.documentElement;

  root.dataset.theme = theme.id;
  root.classList.toggle('dark', theme.isDark);
  root.classList.toggle('el-theme-dark', theme.isDark);
  root.style.removeProperty('--el-color-white');

  localStorage.setItem('theme', theme.id);
  return theme.id;
}
