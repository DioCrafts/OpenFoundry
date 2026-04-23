<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { auth } from '$stores/auth';

	const pendingChallenge = auth.pendingChallenge;

	let code = $state('');
	let error = $state('');
	let loading = $state(false);
	let codeInput: HTMLInputElement | null = null;

	onMount(() => {
		if (!$pendingChallenge) {
			goto('/auth/login');
			return;
		}

		codeInput?.focus();
	});

	async function handleSubmit(event: Event) {
		event.preventDefault();
		error = '';
		loading = true;

		try {
			await auth.completeMfa(code);
			goto('/');
		} catch (err: any) {
			error = err.message ?? 'MFA verification failed';
		} finally {
			loading = false;
		}
	}
</script>

<svelte:head>
	<title>MFA Challenge — OpenFoundry</title>
</svelte:head>

<div class="w-full max-w-sm rounded-3xl border border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-900 p-8 shadow-sm">
	<div class="mb-6">
		<div class="text-xs uppercase tracking-[0.25em] text-gray-400">Security checkpoint</div>
		<h1 class="mt-2 text-2xl font-bold">Enter your verification code</h1>
		<p class="mt-2 text-sm text-gray-500">
			Use your authenticator app or one of your recovery codes to finish signing in.
		</p>
	</div>

	<form onsubmit={handleSubmit} class="space-y-4">
		{#if error}
			<div class="rounded-xl bg-red-50 px-4 py-3 text-sm text-red-700 dark:bg-red-950 dark:text-red-300">
				{error}
			</div>
		{/if}

		<div>
			<label for="mfa-code" class="mb-1 block text-sm font-medium">Verification code</label>
			<input
				id="mfa-code"
				type="text"
				bind:this={codeInput}
				bind:value={code}
				required
				class="w-full rounded-xl border border-gray-300 bg-white px-3 py-2 tracking-[0.35em] uppercase dark:border-gray-700 dark:bg-gray-800"
				placeholder="123456"
			/>
		</div>

		<button
			type="submit"
			disabled={loading}
			class="w-full rounded-xl bg-indigo-600 py-2 font-medium text-white transition-colors hover:bg-indigo-500 disabled:opacity-50"
		>
			{loading ? 'Verifying...' : 'Verify'}
		</button>
	</form>
</div>
