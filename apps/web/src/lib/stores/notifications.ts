import { writable } from 'svelte/store';

export interface Toast {
  id: string;
  type: 'success' | 'error' | 'info' | 'warning';
  message: string;
}

function createNotificationStore() {
  const toasts = writable<Toast[]>([]);

  function add(type: Toast['type'], message: string, duration = 5000) {
    const id = crypto.randomUUID();
    toasts.update((t) => [...t, { id, type, message }]);
    if (duration > 0) {
      setTimeout(() => dismiss(id), duration);
    }
  }

  function dismiss(id: string) {
    toasts.update((t) => t.filter((n) => n.id !== id));
  }

  return {
    subscribe: toasts.subscribe,
    success: (msg: string) => add('success', msg),
    error: (msg: string) => add('error', msg),
    info: (msg: string) => add('info', msg),
    warning: (msg: string) => add('warning', msg),
    dismiss,
  };
}

export const notifications = createNotificationStore();
