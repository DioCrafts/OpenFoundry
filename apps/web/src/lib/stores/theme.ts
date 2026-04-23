import { writable } from 'svelte/store';

type Theme = 'light' | 'dark' | 'system';

function createThemeStore() {
  const theme = writable<Theme>('system');

  function set(value: Theme) {
    theme.set(value);
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem('of_theme', value);
    }
    applyTheme(value);
  }

  function restore() {
    if (typeof localStorage === 'undefined') return;
    const saved = localStorage.getItem('of_theme') as Theme | null;
    if (saved) {
      theme.set(saved);
      applyTheme(saved);
    }
  }

  function applyTheme(value: Theme) {
    if (typeof document === 'undefined') return;
    const isDark =
      value === 'dark' ||
      (value === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches);
    document.documentElement.classList.toggle('dark', isDark);
  }

  return { subscribe: theme.subscribe, set, restore };
}

export const theme = createThemeStore();
