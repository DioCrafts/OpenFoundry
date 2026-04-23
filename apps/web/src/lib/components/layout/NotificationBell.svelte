<script lang="ts">
	import { getNotificationPreferences, listNotifications, markAllNotificationsRead, markNotificationRead, sendNotification, updateNotificationPreferences, type NotificationPreference, type NotificationSocketEvent, type UserNotification } from '$lib/api/notifications';
	import { auth } from '$stores/auth';
	import { notifications as toasts } from '$stores/notifications';
	import { notificationWebsocket } from '$stores/websocket';

	const isAuthenticated = auth.isAuthenticated;
	const token = auth.token;
	const connected = notificationWebsocket.connected;

	let open = $state(false);
	let activeTab = $state<'inbox' | 'preferences'>('inbox');
	let items = $state<UserNotification[]>([]);
	let unreadCount = $state(0);
	let loading = $state(false);
	let saving = $state(false);
	let sendingTest = $state(false);
	let error = $state('');
	let preferences = $state<NotificationPreference>({
		user_id: '',
		in_app_enabled: true,
		email_enabled: false,
		email_address: null,
		slack_webhook_url: null,
		teams_webhook_url: null,
		digest_frequency: 'instant',
		quiet_hours: {},
		updated_at: '',
	});

	async function loadInbox() {
		loading = true;
		try {
			const response = await listNotifications({ limit: 20 });
			items = response.data;
			unreadCount = response.unread_count;
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Failed to load notifications';
		} finally {
			loading = false;
		}
	}

	async function loadPreferences() {
		try {
			preferences = await getNotificationPreferences();
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Failed to load notification preferences';
		}
	}

	function upsertNotification(notification: UserNotification) {
		items = [notification, ...items.filter((item) => item.id !== notification.id)].slice(0, 20);
	}

	function applySocketEvent(event: NotificationSocketEvent) {
		unreadCount = event.unread_count;

		if (event.kind === 'snapshot') {
			items = event.data ?? [];
			return;
		}

		if (event.notification) {
			upsertNotification(event.notification);
			if (event.kind === 'notification.created') {
				toasts.info(event.notification.title);
			}
		}

		if (event.kind === 'notification.read_all') {
			items = items.map((item) => ({ ...item, status: 'read', read_at: item.read_at ?? new Date().toISOString() }));
		}
	}

	async function markRead(id: string) {
		const response = await markNotificationRead(id);
		unreadCount = response.unread_count;
		upsertNotification(response.notification);
	}

	async function markEverythingRead() {
		const response = await markAllNotificationsRead();
		unreadCount = response.unread_count;
		items = items.map((item) => ({ ...item, status: 'read', read_at: item.read_at ?? new Date().toISOString() }));
	}

	async function savePreferences() {
		saving = true;
		error = '';
		try {
			preferences = await updateNotificationPreferences({
				in_app_enabled: preferences.in_app_enabled,
				email_enabled: preferences.email_enabled,
				email_address: preferences.email_address,
				slack_webhook_url: preferences.slack_webhook_url,
				teams_webhook_url: preferences.teams_webhook_url,
				digest_frequency: preferences.digest_frequency,
				quiet_hours: preferences.quiet_hours,
			});
			toasts.success('Notification preferences updated');
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Failed to update preferences';
		} finally {
			saving = false;
		}
	}

	async function sendTestNotification() {
		sendingTest = true;
		error = '';
		try {
			const notification = await sendNotification({
				title: 'Workflow alert test',
				body: 'The notification service is connected and your in-app inbox is receiving events.',
				category: 'test',
				severity: 'info',
				channels: ['in_app'],
			});
			upsertNotification(notification);
			unreadCount += 1;
			toasts.success('Test notification sent');
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Failed to send test notification';
		} finally {
			sendingTest = false;
		}
	}

	$effect(() => {
		if ($isAuthenticated && $token) {
			void loadInbox();
			void loadPreferences();
			notificationWebsocket.connect($token, applySocketEvent);

			return () => notificationWebsocket.disconnect();
		}

		notificationWebsocket.disconnect();
	});
</script>

{#if $isAuthenticated}
	<div class="relative">
		<button
			type="button"
			aria-label="Open notification center"
			onclick={() => {
				open = !open;
				if (open) {
					void loadInbox();
				}
			}}
			class="relative rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm shadow-sm transition-colors hover:bg-slate-50 dark:border-gray-700 dark:bg-gray-900 dark:hover:bg-gray-800"
		>
			<span>🔔</span>
			{#if unreadCount > 0}
				<span class="absolute -right-1 -top-1 rounded-full bg-rose-500 px-1.5 py-0.5 text-[10px] font-semibold text-white">
					{unreadCount > 99 ? '99+' : unreadCount}
				</span>
			{/if}
		</button>

		{#if open}
			<div class="absolute right-0 top-14 z-20 w-[24rem] rounded-2xl border border-slate-200 bg-white p-4 shadow-2xl dark:border-gray-700 dark:bg-gray-900">
				<div class="flex items-center justify-between">
					<div>
						<div class="text-xs uppercase tracking-[0.22em] text-gray-400">Notification Center</div>
						<div class="mt-1 text-sm text-gray-500">In-app alerts, delivery preferences, and live workflow approvals.</div>
					</div>
					<div class="flex items-center gap-2 text-xs text-gray-500">
						<span class={`h-2.5 w-2.5 rounded-full ${$connected ? 'bg-emerald-500' : 'bg-amber-500'}`}></span>
						{$connected ? 'Live' : 'Offline'}
					</div>
				</div>

				<div class="mt-4 flex gap-2 rounded-xl bg-slate-100 p-1 dark:bg-gray-800">
					<button
						type="button"
						onclick={() => activeTab = 'inbox'}
						class={`flex-1 rounded-lg px-3 py-2 text-sm font-medium ${activeTab === 'inbox' ? 'bg-white shadow-sm dark:bg-gray-900' : 'text-gray-500'}`}
					>Inbox</button>
					<button
						type="button"
						onclick={() => activeTab = 'preferences'}
						class={`flex-1 rounded-lg px-3 py-2 text-sm font-medium ${activeTab === 'preferences' ? 'bg-white shadow-sm dark:bg-gray-900' : 'text-gray-500'}`}
					>Preferences</button>
				</div>

				{#if error}
					<div class="mt-4 rounded-xl border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-700 dark:border-rose-900/40 dark:bg-rose-950/40 dark:text-rose-300">{error}</div>
				{/if}

				{#if activeTab === 'inbox'}
					<div class="mt-4 space-y-3">
						<div class="flex items-center justify-between text-sm text-gray-500">
							<span>{unreadCount} unread</span>
							<div class="flex gap-2">
								<button type="button" onclick={sendTestNotification} disabled={sendingTest} class="rounded-lg border border-slate-200 px-3 py-1.5 hover:bg-slate-50 disabled:opacity-50 dark:border-gray-700 dark:hover:bg-gray-800">
									{sendingTest ? 'Sending...' : 'Send test'}
								</button>
								<button type="button" onclick={markEverythingRead} class="rounded-lg border border-slate-200 px-3 py-1.5 hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">
									Mark all read
								</button>
							</div>
						</div>

						{#if loading}
							<div class="py-8 text-center text-sm text-gray-500">Loading notifications...</div>
						{:else if items.length === 0}
							<div class="rounded-xl border border-dashed border-slate-300 px-4 py-8 text-center text-sm text-gray-500 dark:border-gray-700">
								No notifications yet.
							</div>
						{:else}
							<div class="max-h-[22rem] space-y-3 overflow-auto pr-1">
								{#each items as item (item.id)}
									<div class={`rounded-xl border p-3 ${item.status === 'unread' ? 'border-blue-200 bg-blue-50/70 dark:border-blue-900/40 dark:bg-blue-950/30' : 'border-slate-200 dark:border-gray-700'}`}>
										<div class="flex items-start justify-between gap-3">
											<div>
												<div class="flex flex-wrap items-center gap-2">
													<div class="font-medium">{item.title}</div>
													<span class="rounded-full bg-slate-100 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-[0.16em] text-slate-600 dark:bg-gray-800 dark:text-gray-300">{item.category}</span>
												</div>
												<div class="mt-1 text-sm text-gray-600 dark:text-gray-300">{item.body}</div>
												<div class="mt-2 flex flex-wrap gap-2 text-xs text-gray-500">
													<span>{new Date(item.created_at).toLocaleString()}</span>
													<span>{Array.isArray(item.channels) ? item.channels.join(', ') : ''}</span>
												</div>
											</div>
											{#if item.status === 'unread'}
												<button type="button" onclick={() => markRead(item.id)} class="rounded-lg border border-slate-200 px-2.5 py-1 text-xs hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">
													Read
												</button>
											{/if}
										</div>
									</div>
								{/each}
							</div>
						{/if}
					</div>
				{:else}
					<div class="mt-4 space-y-4 text-sm">
						<label class="flex items-center justify-between rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700">
							<span>Enable in-app notifications</span>
							<input type="checkbox" bind:checked={preferences.in_app_enabled} />
						</label>

						<label class="flex items-center justify-between rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700">
							<span>Enable email delivery</span>
							<input type="checkbox" bind:checked={preferences.email_enabled} />
						</label>

						<input bind:value={preferences.email_address} placeholder="Email address" class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800" />
						<input bind:value={preferences.slack_webhook_url} placeholder="Slack webhook URL" class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800" />
						<input bind:value={preferences.teams_webhook_url} placeholder="MS Teams webhook URL" class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800" />

						<select bind:value={preferences.digest_frequency} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800">
							<option value="instant">Instant</option>
							<option value="hourly">Hourly digest</option>
							<option value="daily">Daily digest</option>
						</select>

						<button type="button" onclick={savePreferences} disabled={saving} class="w-full rounded-xl bg-slate-900 px-4 py-2 text-white disabled:opacity-50 dark:bg-white dark:text-slate-900">
							{saving ? 'Saving...' : 'Save preferences'}
						</button>
					</div>
				{/if}
			</div>
		{/if}
	</div>
{/if}
