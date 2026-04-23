<script lang="ts">
  import '../app.css';
  import { auth } from '$stores/auth';
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import Sidebar from '$components/layout/Sidebar.svelte';
  import TopBar from '$components/layout/TopBar.svelte';
  import CopilotPanel from '$components/ai/CopilotPanel.svelte';

  let { children } = $props();

  onMount(() => {
    auth.restore();
  });

  // Auth pages don't get the app shell
  const isAuthPage = $derived($page.url.pathname.startsWith('/auth'));
</script>

{#if isAuthPage}
  <div class="min-h-full flex items-center justify-center bg-gray-50 dark:bg-gray-950">
    {@render children()}
  </div>
{:else}
  <div class="h-full flex">
    <Sidebar />
    <div class="flex-1 flex flex-col min-w-0">
      <TopBar />
      <main class="flex-1 overflow-auto p-6">
        {@render children()}
      </main>
    </div>
  </div>
  <CopilotPanel />
{/if}