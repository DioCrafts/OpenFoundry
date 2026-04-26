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
  <div class="of-shell flex min-h-full items-center justify-center">
    {@render children()}
  </div>
{:else}
  <div class="of-shell flex h-full">
    <Sidebar />
    <div class="flex min-w-0 flex-1 flex-col">
      <TopBar />
      <main class="of-scrollbar flex-1 overflow-auto">
        <div class="of-page">
          {@render children()}
        </div>
      </main>
    </div>
  </div>
  <CopilotPanel />
{/if}
