<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { auth } from '$stores/auth';

	let error = $state('');

	onMount(async () => {
		const params = new URLSearchParams(window.location.search);
		const code = params.get('code');
		const state = params.get('state');

		if (!code || !state) {
			error = 'Missing authorization code or state.';
			return;
		}

		try {
			const result = await auth.handleSsoCallback(code, state);
			goto(result.status === 'mfa_required' ? '/auth/mfa' : '/');
		} catch (err: any) {
			error = err.message ?? 'SSO callback failed';
		}
	});
</script>

<svelte:head>
	<title>Signing In — OpenFoundry</title>
</svelte:head>

<div class="w-full max-w-md rounded-3xl border border-gray-200 bg-white p-8 text-center shadow-sm dark:border-gray-800 dark:bg-gray-900">
	<div class="text-xs uppercase tracking-[0.25em] text-gray-400">Single sign-on</div>
	<h1 class="mt-2 text-2xl font-bold">Completing your sign-in</h1>
	{#if error}
		<p class="mt-4 rounded-xl bg-red-50 px-4 py-3 text-sm text-red-700 dark:bg-red-950 dark:text-red-300">{error}</p>
		<a href="/auth/login" class="mt-6 inline-block text-sm text-indigo-600 hover:text-indigo-500">Back to login</a>
	{:else}
		<p class="mt-4 text-sm text-gray-500">We are validating your identity provider response.</p>
	{/if}
</div>
