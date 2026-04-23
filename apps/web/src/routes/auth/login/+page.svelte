<script lang="ts">
  import { listPublicSsoProviders, type PublicSsoProvider } from '$api/auth';
  import { auth } from '$stores/auth';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';

  let email = $state('');
  let password = $state('');
  let error = $state('');
  let loading = $state(false);
  let providers = $state<PublicSsoProvider[]>([]);

  onMount(async () => {
    try {
      providers = await listPublicSsoProviders();
    } catch {
      providers = [];
    }
  });

  async function handleSubmit(e: Event) {
    e.preventDefault();
    error = '';
    loading = true;

    try {
      const result = await auth.login(email, password);
      goto(result.status === 'mfa_required' ? '/auth/mfa' : '/');
    } catch (err: any) {
      error = err.message ?? 'Login failed';
    } finally {
      loading = false;
    }
  }

  async function handleSsoLogin(slug: string) {
    error = '';

    try {
      await auth.startSsoLogin(slug);
    } catch (err: any) {
      error = err.message ?? 'SSO login failed';
    }
  }
</script>

<svelte:head>
  <title>Login — OpenFoundry</title>
</svelte:head>

<div class="w-full max-w-sm">
  <div class="text-center mb-8">
    <span class="text-4xl text-indigo-500">◆</span>
    <h1 class="text-2xl font-bold mt-2">Sign in to OpenFoundry</h1>
  </div>

  <form onsubmit={handleSubmit} class="space-y-4">
    {#if error}
      <div class="p-3 text-sm text-red-700 bg-red-50 dark:bg-red-950 dark:text-red-300 rounded-lg">
        {error}
      </div>
    {/if}

    <div>
      <label for="email" class="block text-sm font-medium mb-1">Email</label>
      <input
        id="email"
        type="email"
        bind:value={email}
        required
        class="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 rounded-lg
               bg-white dark:bg-gray-800 focus:outline-none focus:ring-2 focus:ring-indigo-500"
        placeholder="you@company.com"
      />
    </div>

    <div>
      <label for="password" class="block text-sm font-medium mb-1">Password</label>
      <input
        id="password"
        type="password"
        bind:value={password}
        required
        class="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 rounded-lg
               bg-white dark:bg-gray-800 focus:outline-none focus:ring-2 focus:ring-indigo-500"
        placeholder="••••••••"
      />
    </div>

    <button
      type="submit"
      disabled={loading}
      class="w-full py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-500
             disabled:opacity-50 transition-colors font-medium"
    >
      {loading ? 'Signing in...' : 'Sign In'}
    </button>

    {#if providers.length > 0}
      <div class="pt-3 space-y-2">
        <div class="text-xs uppercase tracking-[0.2em] text-gray-400">Single sign-on</div>
        {#each providers as provider}
          <button
            type="button"
            onclick={() => handleSsoLogin(provider.slug)}
            class="w-full py-2 border border-gray-300 dark:border-gray-700 rounded-lg hover:border-indigo-400 transition-colors font-medium"
          >
            Continue with {provider.name}
          </button>
        {/each}
      </div>
    {/if}
  </form>

  <p class="text-center text-sm text-gray-500 mt-6">
    Don't have an account?
    <a href="/auth/register" class="text-indigo-600 hover:text-indigo-500">Register</a>
  </p>
</div>
