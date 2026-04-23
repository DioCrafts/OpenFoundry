import { browser } from '$app/environment';
import { get, writable } from 'svelte/store';
import {
  createDashboard,
  createStarterDashboards,
  duplicateDashboardDefinition,
  type DashboardDefinition,
} from '$lib/utils/dashboards';

const STORAGE_KEY = 'of_dashboards';

function createDashboardsStore() {
  const dashboards = writable<DashboardDefinition[]>([]);
  let restored = false;

  function persist(next: DashboardDefinition[]) {
    dashboards.set(next);

    if (!browser) {
      return next;
    }

    localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
    return next;
  }

  function restore() {
    if (restored) {
      return;
    }

    restored = true;

    if (!browser) {
      return;
    }

    const raw = localStorage.getItem(STORAGE_KEY);

    if (!raw) {
      persist(createStarterDashboards());
      return;
    }

    try {
      const parsed = JSON.parse(raw) as DashboardDefinition[];
      if (!Array.isArray(parsed) || parsed.length === 0) {
        persist(createStarterDashboards());
        return;
      }

      dashboards.set(parsed);
    } catch {
      persist(createStarterDashboards());
    }
  }

  function create(name?: string) {
    const dashboard = createDashboard(name);
    persist([...get(dashboards), dashboard]);
    return dashboard;
  }

  function save(dashboard: DashboardDefinition) {
    const nextDashboard = {
      ...dashboard,
      updatedAt: new Date().toISOString(),
    };

    const existing = get(dashboards);
    const hasDashboard = existing.some((entry) => entry.id === dashboard.id);

    if (hasDashboard) {
      persist(existing.map((entry) => entry.id === dashboard.id ? nextDashboard : entry));
    } else {
      persist([...existing, nextDashboard]);
    }

    return nextDashboard;
  }

  function remove(id: string) {
    persist(get(dashboards).filter((entry) => entry.id !== id));
  }

  function duplicate(id: string) {
    const source = get(dashboards).find((entry) => entry.id === id);
    if (!source) {
      return null;
    }

    const copy = duplicateDashboardDefinition(source);
    persist([...get(dashboards), copy]);
    return copy;
  }

  function getById(id: string) {
    return get(dashboards).find((entry) => entry.id === id) ?? null;
  }

  return {
    subscribe: dashboards.subscribe,
    restore,
    create,
    save,
    remove,
    duplicate,
    getById,
  };
}

export const dashboards = createDashboardsStore();