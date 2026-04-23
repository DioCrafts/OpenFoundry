<script lang="ts">
  import { auth } from '$stores/auth';
  import { goto } from '$app/navigation';
  import NotificationBell from '$components/layout/NotificationBell.svelte';

  const isAuthenticated = auth.isAuthenticated;
  const user = auth.user;

  function handleLogout() {
    auth.logout();
    goto('/auth/login');
  }
</script>

<header class="h-14 bg-white dark:bg-gray-900 border-b border-gray-200 dark:border-gray-800 flex items-center justify-between px-6">
  <div class="flex items-center gap-4">
    <div class="relative">
      <input
        type="text"
        placeholder="Search datasets, queries, pipelines..."
        class="w-80 pl-9 pr-4 py-1.5 text-sm rounded-lg border border-gray-300 dark:border-gray-700
               bg-gray-50 dark:bg-gray-800 focus:outline-none focus:ring-2 focus:ring-indigo-500"
      />
      <span class="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400 text-sm">🔍</span>
    </div>
  </div>

  <div class="flex items-center gap-3">
    {#if $isAuthenticated}
      <NotificationBell />
      <span class="text-sm text-gray-600 dark:text-gray-400">
        {$user?.name ?? ''}
      </span>
      <button
        onclick={handleLogout}
        class="text-sm text-gray-500 hover:text-gray-700 dark:hover:text-gray-300"
      >
        Logout
      </button>
    {:else}
      <a href="/auth/login" class="text-sm text-indigo-600 hover:text-indigo-500">Login</a>
    {/if}
  </div>
</header>
