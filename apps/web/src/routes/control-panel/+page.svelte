<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { get } from 'svelte/store';

	import { ApiError } from '$api/client';
	import {
		getControlPanel,
		getUpgradeReadiness,
		previewIdentityProviderMapping,
		updateControlPanel,
		type AppBrandingSettings,
		type ControlPanelSettings,
		type IdentityProviderMapping,
		type IdentityProviderMappingPreviewResponse,
		type UpgradeAssistantSettings,
		type UpgradeReadinessResponse,
	} from '$lib/api/control-panel';
	import { listSsoProviders, type SsoProviderRecord } from '$lib/api/auth';
	import { auth } from '$stores/auth';
	import { notifications } from '$stores/notifications';

	type ControlPanelDraft = {
		platform_name: string;
		support_email: string;
		docs_url: string;
		status_page_url: string;
		announcement_banner: string;
		maintenance_mode: boolean;
		release_channel: string;
		default_region: string;
		deployment_mode: string;
		allow_self_signup: boolean;
		allowed_email_domains_text: string;
		restricted_operations_text: string;
		branding_display_name: string;
		branding_primary_color: string;
		branding_accent_color: string;
		branding_logo_url: string;
		branding_favicon_url: string;
		branding_show_powered_by: boolean;
		identity_provider_mappings_json: string;
		resource_management_policies_json: string;
		upgrade_assistant_json: string;
	};

	const currentUser = auth.user;
	const isAuthenticated = auth.isAuthenticated;

	let loading = $state(true);
	let saving = $state(false);
	let uiError = $state('');
	let notice = $state('');
	let settings = $state<ControlPanelSettings | null>(null);
	let draft = $state<ControlPanelDraft>(createEmptyDraft());
	let ssoProviders = $state<SsoProviderRecord[]>([]);
	let upgradeReadiness = $state<UpgradeReadinessResponse | null>(null);
	let previewLoading = $state(false);
	let idpPreviewProviderSlug = $state('');
	let idpPreviewEmail = $state('');
	let idpPreviewRawClaimsJson = $state(JSON.stringify(defaultPreviewClaims(), null, 2));
	let idpPreviewResult = $state<IdentityProviderMappingPreviewResponse | null>(null);
	let idpPreviewError = $state('');

	onMount(() => {
		void loadPage();
	});

	function createEmptyDraft(): ControlPanelDraft {
		return {
			platform_name: 'OpenFoundry',
			support_email: 'support@openfoundry.dev',
			docs_url: 'https://docs.openfoundry.dev',
			status_page_url: 'https://status.openfoundry.dev',
			announcement_banner: '',
			maintenance_mode: false,
			release_channel: 'stable',
			default_region: 'eu-west-1',
			deployment_mode: 'self-hosted',
			allow_self_signup: false,
			allowed_email_domains_text: '',
			restricted_operations_text: '',
			branding_display_name: 'OpenFoundry',
			branding_primary_color: '#0f766e',
			branding_accent_color: '#d97706',
			branding_logo_url: '',
			branding_favicon_url: '',
			branding_show_powered_by: true,
			identity_provider_mappings_json: '[]',
			resource_management_policies_json: '[]',
			upgrade_assistant_json: JSON.stringify(defaultUpgradeAssistant(), null, 2),
		};
	}

	function defaultUpgradeAssistant(): UpgradeAssistantSettings {
		return {
			current_version: '2026.04.0',
			target_version: '2026.05.0',
			maintenance_window: 'Sun 02:00-04:00 UTC',
			rollback_channel: 'stable',
			preflight_checks: [],
			rollout_stages: [],
			rollback_steps: [],
		};
	}

	function defaultPreviewClaims() {
		return {
			organization_id: '00000000-0000-7000-8000-000000000001',
			workspace: 'shared-enterprise',
			classification_clearance: 'internal',
			department: 'finance',
			roles: ['viewer', 'editor'],
		};
	}

	function toDraft(value: ControlPanelSettings): ControlPanelDraft {
		return {
			platform_name: value.platform_name,
			support_email: value.support_email,
			docs_url: value.docs_url,
			status_page_url: value.status_page_url,
			announcement_banner: value.announcement_banner,
			maintenance_mode: value.maintenance_mode,
			release_channel: value.release_channel,
			default_region: value.default_region,
			deployment_mode: value.deployment_mode,
			allow_self_signup: value.allow_self_signup,
			allowed_email_domains_text: value.allowed_email_domains.join(', '),
			restricted_operations_text: value.restricted_operations.join(', '),
			branding_display_name: value.default_app_branding.display_name,
			branding_primary_color: value.default_app_branding.primary_color,
			branding_accent_color: value.default_app_branding.accent_color,
			branding_logo_url: value.default_app_branding.logo_url ?? '',
			branding_favicon_url: value.default_app_branding.favicon_url ?? '',
			branding_show_powered_by: value.default_app_branding.show_powered_by,
			identity_provider_mappings_json: JSON.stringify(value.identity_provider_mappings, null, 2),
			resource_management_policies_json: JSON.stringify(value.resource_management_policies, null, 2),
			upgrade_assistant_json: JSON.stringify(value.upgrade_assistant, null, 2),
		};
	}

	function canReadControlPanel() {
		const user = get(currentUser);
		if (!user) return false;
		return user.roles.includes('admin')
			|| user.permissions.includes('*:*')
			|| user.permissions.includes('control_panel:read')
			|| user.permissions.includes('control_panel:*')
			|| user.permissions.includes('control_panel:write');
	}

	function canWriteControlPanel() {
		const user = get(currentUser);
		if (!user) return false;
		return user.roles.includes('admin')
			|| user.permissions.includes('*:*')
			|| user.permissions.includes('control_panel:write')
			|| user.permissions.includes('control_panel:*');
	}

	function parseCsv(value: string) {
		return value
			.split(',')
			.map((entry) => entry.trim())
			.filter(Boolean);
	}

	function toNullableString(value: string) {
		const trimmed = value.trim();
		return trimmed ? trimmed : null;
	}

	function buildBranding(): AppBrandingSettings {
		return {
			display_name: draft.branding_display_name.trim() || draft.platform_name.trim() || 'OpenFoundry',
			primary_color: draft.branding_primary_color,
			accent_color: draft.branding_accent_color,
			logo_url: toNullableString(draft.branding_logo_url),
			favicon_url: toNullableString(draft.branding_favicon_url),
			show_powered_by: draft.branding_show_powered_by,
		};
	}

	function parseJsonDraft<T>(value: string, label: string) {
		try {
			return JSON.parse(value) as T;
		} catch (error) {
			const detail = error instanceof Error ? error.message : 'Invalid JSON';
			throw new Error(`${label} contains invalid JSON: ${detail}`);
		}
	}

	function setPreviewDefaults(nextSettings: ControlPanelSettings, providers: SsoProviderRecord[]) {
		const primaryMapping = nextSettings.identity_provider_mappings[0];
		const defaultProviderSlug = primaryMapping?.provider_slug ?? providers[0]?.slug ?? '';
		const defaultDomain = primaryMapping?.allowed_email_domains[0] ?? 'openfoundry.dev';

		if (!idpPreviewProviderSlug || !providers.some((provider) => provider.slug === idpPreviewProviderSlug)) {
			idpPreviewProviderSlug = defaultProviderSlug;
		}
		if (!idpPreviewEmail) {
			idpPreviewEmail = `operator@${defaultDomain}`;
		}
		if (!idpPreviewRawClaimsJson.trim()) {
			idpPreviewRawClaimsJson = JSON.stringify(defaultPreviewClaims(), null, 2);
		}
	}

	function mappingSummary() {
		try {
			const mappings = parseJsonDraft<IdentityProviderMapping[]>(
				draft.identity_provider_mappings_json,
				'Identity provider mappings',
			);
			const ruleCount = mappings.reduce((total, entry) => total + entry.organization_rules.length, 0);
			return `${mappings.length} mapping(s), ${ruleCount} advanced org rule(s)`;
		} catch {
			return 'Invalid mapping JSON';
		}
	}

	function policySummary() {
		try {
			const policies = parseJsonDraft<{ name: string }[]>(
				draft.resource_management_policies_json,
				'Resource management policies',
			);
			return `${policies.length} resource policy/policies configured`;
		} catch {
			return 'Invalid policy JSON';
		}
	}

	async function runIdentityMappingPreview() {
		if (!idpPreviewProviderSlug.trim()) {
			idpPreviewError = 'Selecciona un provider para ejecutar el preview.';
			return;
		}

		previewLoading = true;
		idpPreviewError = '';
		idpPreviewResult = null;

		try {
			const rawClaims = parseJsonDraft<Record<string, unknown>>(idpPreviewRawClaimsJson, 'Identity preview claims');
			idpPreviewResult = await previewIdentityProviderMapping({
				provider_slug: idpPreviewProviderSlug.trim(),
				email: idpPreviewEmail.trim(),
				raw_claims: rawClaims,
			});
		} catch (error: unknown) {
			idpPreviewError = error instanceof Error ? error.message : 'Unable to run the identity mapping preview';
		} finally {
			previewLoading = false;
		}
	}

	async function loadPage() {
		loading = true;
		uiError = '';
		notice = '';

		try {
			await auth.restore();
			if (!get(isAuthenticated)) {
				goto('/auth/login');
				return;
			}
			if (!canReadControlPanel()) {
				uiError = 'Your session cannot read the platform control panel.';
				return;
			}

			const [nextSettings, providers, readiness] = await Promise.all([
				getControlPanel(),
				listSsoProviders().catch(() => []),
				getUpgradeReadiness(),
			]);
			settings = nextSettings;
			ssoProviders = providers;
			upgradeReadiness = readiness;
			draft = toDraft(nextSettings);
			setPreviewDefaults(nextSettings, providers);
		} catch (error: unknown) {
			if (error instanceof ApiError && error.status === 403) {
				uiError = 'Your session cannot read the platform control panel.';
			} else {
				uiError = error instanceof Error ? error.message : 'Unable to load the control panel';
			}
		} finally {
			loading = false;
		}
	}

	async function saveControlPanel() {
		if (!canWriteControlPanel()) {
			uiError = 'Your session cannot modify the platform control panel.';
			return;
		}

		saving = true;
		uiError = '';
		notice = '';

		try {
			const nextSettings = await updateControlPanel({
				platform_name: draft.platform_name.trim(),
				support_email: draft.support_email.trim(),
				docs_url: draft.docs_url.trim(),
				status_page_url: draft.status_page_url.trim(),
				announcement_banner: draft.announcement_banner.trim(),
				maintenance_mode: draft.maintenance_mode,
				release_channel: draft.release_channel.trim(),
				default_region: draft.default_region.trim(),
				deployment_mode: draft.deployment_mode.trim(),
				allow_self_signup: draft.allow_self_signup,
				allowed_email_domains: parseCsv(draft.allowed_email_domains_text),
				default_app_branding: buildBranding(),
				restricted_operations: parseCsv(draft.restricted_operations_text),
				identity_provider_mappings: parseJsonDraft(draft.identity_provider_mappings_json, 'Identity provider mappings'),
				resource_management_policies: parseJsonDraft(draft.resource_management_policies_json, 'Resource management policies'),
				upgrade_assistant: parseJsonDraft(draft.upgrade_assistant_json, 'Upgrade assistant'),
			});
			settings = nextSettings;
			upgradeReadiness = await getUpgradeReadiness();
			draft = toDraft(nextSettings);
			setPreviewDefaults(nextSettings, ssoProviders);
			notice = 'Platform control panel updated.';
			notifications.success(notice);
		} catch (error: unknown) {
			uiError = error instanceof Error ? error.message : 'Unable to save the control panel';
			notifications.error(uiError);
		} finally {
			saving = false;
		}
	}
</script>

<svelte:head>
	<title>OpenFoundry — Control Panel</title>
</svelte:head>

<div class="mx-auto max-w-6xl space-y-6">
	<section class="overflow-hidden rounded-[2rem] border border-slate-200 bg-gradient-to-br from-emerald-950 via-slate-950 to-amber-950 px-6 py-8 text-white shadow-xl shadow-emerald-950/20">
		<div class="flex flex-col gap-6 lg:flex-row lg:items-end lg:justify-between">
			<div class="max-w-3xl">
				<p class="text-xs font-semibold uppercase tracking-[0.28em] text-emerald-200">Platform Control</p>
				<h1 class="mt-3 text-3xl font-semibold tracking-tight">{draft.platform_name || 'OpenFoundry'} Control Panel</h1>
				<p class="mt-3 max-w-2xl text-sm leading-6 text-slate-200">
					Gestiona branding, comunicaciones operativas, canal de release, restricciones de plataforma y postura de self-signup desde una superficie única.
				</p>
			</div>
			<div class="grid gap-3 sm:grid-cols-3">
				<div class="rounded-2xl border border-white/10 bg-white/10 px-4 py-3 backdrop-blur">
					<div class="text-[11px] uppercase tracking-[0.24em] text-emerald-200">Release</div>
					<div class="mt-2 text-lg font-semibold">{draft.release_channel || 'stable'}</div>
				</div>
				<div class="rounded-2xl border border-white/10 bg-white/10 px-4 py-3 backdrop-blur">
					<div class="text-[11px] uppercase tracking-[0.24em] text-emerald-200">Deployment</div>
					<div class="mt-2 text-lg font-semibold">{draft.deployment_mode || 'self-hosted'}</div>
				</div>
				<div class="rounded-2xl border border-white/10 bg-white/10 px-4 py-3 backdrop-blur">
					<div class="text-[11px] uppercase tracking-[0.24em] text-emerald-200">Status</div>
					<div class="mt-2 text-lg font-semibold">{draft.maintenance_mode ? 'Maintenance' : 'Operational'}</div>
				</div>
			</div>
		</div>
	</section>

	{#if loading}
		<section class="rounded-3xl border border-slate-200 bg-white px-6 py-8 text-sm text-slate-500 shadow-sm">
			Loading platform control plane...
		</section>
	{:else if uiError && !settings && !canReadControlPanel()}
		<section class="rounded-3xl border border-amber-200 bg-amber-50 px-6 py-8 text-sm text-amber-800 shadow-sm">
			{uiError}
		</section>
	{:else}
		{#if uiError}
			<div class="rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700">{uiError}</div>
		{/if}
		{#if notice}
			<div class="rounded-2xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-700">{notice}</div>
		{/if}

		<div class="grid gap-6 xl:grid-cols-[1.2fr_0.8fr]">
			<section class="rounded-3xl border border-slate-200 bg-white p-6 shadow-sm">
				<div class="flex items-center justify-between gap-4">
					<div>
						<p class="text-xs font-semibold uppercase tracking-[0.24em] text-slate-500">Operations</p>
						<h2 class="mt-2 text-xl font-semibold text-slate-900">Platform administration</h2>
					</div>
					<button
						class="rounded-full bg-slate-900 px-4 py-2 text-sm font-semibold text-white transition hover:bg-slate-700 disabled:cursor-not-allowed disabled:bg-slate-300"
						onclick={saveControlPanel}
						disabled={saving || !canWriteControlPanel()}
					>
						{saving ? 'Saving...' : 'Save changes'}
					</button>
				</div>

				<div class="mt-6 grid gap-4 md:grid-cols-2">
					<label class="block text-sm">
						<span class="mb-2 block font-medium text-slate-700">Platform name</span>
						<input class="w-full rounded-2xl border border-slate-200 px-4 py-3 outline-none transition focus:border-emerald-500" bind:value={draft.platform_name} />
					</label>
					<label class="block text-sm">
						<span class="mb-2 block font-medium text-slate-700">Support email</span>
						<input class="w-full rounded-2xl border border-slate-200 px-4 py-3 outline-none transition focus:border-emerald-500" bind:value={draft.support_email} />
					</label>
					<label class="block text-sm">
						<span class="mb-2 block font-medium text-slate-700">Docs URL</span>
						<input class="w-full rounded-2xl border border-slate-200 px-4 py-3 outline-none transition focus:border-emerald-500" bind:value={draft.docs_url} />
					</label>
					<label class="block text-sm">
						<span class="mb-2 block font-medium text-slate-700">Status page URL</span>
						<input class="w-full rounded-2xl border border-slate-200 px-4 py-3 outline-none transition focus:border-emerald-500" bind:value={draft.status_page_url} />
					</label>
					<label class="block text-sm">
						<span class="mb-2 block font-medium text-slate-700">Release channel</span>
						<input class="w-full rounded-2xl border border-slate-200 px-4 py-3 outline-none transition focus:border-emerald-500" bind:value={draft.release_channel} />
					</label>
					<label class="block text-sm">
						<span class="mb-2 block font-medium text-slate-700">Default region</span>
						<input class="w-full rounded-2xl border border-slate-200 px-4 py-3 outline-none transition focus:border-emerald-500" bind:value={draft.default_region} />
					</label>
					<label class="block text-sm">
						<span class="mb-2 block font-medium text-slate-700">Deployment mode</span>
						<input class="w-full rounded-2xl border border-slate-200 px-4 py-3 outline-none transition focus:border-emerald-500" bind:value={draft.deployment_mode} />
					</label>
					<label class="flex items-center gap-3 rounded-2xl border border-slate-200 px-4 py-3 text-sm text-slate-700">
						<input type="checkbox" bind:checked={draft.maintenance_mode} />
						Maintenance mode
					</label>
					<label class="flex items-center gap-3 rounded-2xl border border-slate-200 px-4 py-3 text-sm text-slate-700 md:col-span-2">
						<input type="checkbox" bind:checked={draft.allow_self_signup} />
						Allow self-signup for new enrollments
					</label>
					<label class="block text-sm md:col-span-2">
						<span class="mb-2 block font-medium text-slate-700">Announcement banner</span>
						<textarea class="min-h-24 w-full rounded-2xl border border-slate-200 px-4 py-3 outline-none transition focus:border-emerald-500" bind:value={draft.announcement_banner}></textarea>
					</label>
					<label class="block text-sm md:col-span-2">
						<span class="mb-2 block font-medium text-slate-700">Allowed email domains</span>
						<input class="w-full rounded-2xl border border-slate-200 px-4 py-3 outline-none transition focus:border-emerald-500" bind:value={draft.allowed_email_domains_text} />
						<p class="mt-2 text-xs text-slate-500">Comma-separated domains used to constrain self-signup and external invites.</p>
					</label>
					<label class="block text-sm md:col-span-2">
						<span class="mb-2 block font-medium text-slate-700">Restricted operations</span>
						<input class="w-full rounded-2xl border border-slate-200 px-4 py-3 outline-none transition focus:border-emerald-500" bind:value={draft.restricted_operations_text} />
						<p class="mt-2 text-xs text-slate-500">Comma-separated operations to slow down during incidents, freezes or upgrade windows.</p>
					</label>
					<label class="block text-sm md:col-span-2">
						<span class="mb-2 block font-medium text-slate-700">Identity provider mappings JSON</span>
						<textarea class="min-h-48 w-full rounded-2xl border border-slate-200 px-4 py-3 font-mono text-xs outline-none transition focus:border-emerald-500" bind:value={draft.identity_provider_mappings_json}></textarea>
						<p class="mt-2 text-xs text-slate-500">Asigna org, workspace, clearance, default roles y dominios permitidos por `provider_slug`. Soporta `organization_rules` con `email_domain` o `claim_equals`. Providers detectados: {ssoProviders.length}. {mappingSummary()}.</p>
					</label>
					<label class="block text-sm md:col-span-2">
						<span class="mb-2 block font-medium text-slate-700">Resource management policies JSON</span>
						<textarea class="min-h-48 w-full rounded-2xl border border-slate-200 px-4 py-3 font-mono text-xs outline-none transition focus:border-emerald-500" bind:value={draft.resource_management_policies_json}></textarea>
						<p class="mt-2 text-xs text-slate-500">Estas políticas alimentan `tenant_tier` y `tenant_quotas`, que luego el gateway usa para clamp de queries, workers, body size y rate limiting. {policySummary()}.</p>
					</label>
					<label class="block text-sm md:col-span-2">
						<span class="mb-2 block font-medium text-slate-700">Upgrade assistant JSON</span>
						<textarea class="min-h-48 w-full rounded-2xl border border-slate-200 px-4 py-3 font-mono text-xs outline-none transition focus:border-emerald-500" bind:value={draft.upgrade_assistant_json}></textarea>
						<p class="mt-2 text-xs text-slate-500">Define versión actual/objetivo, maintenance window, preflight checks con estados `pending/ready/warning/blocked`, stages de rollout y rollback steps. El panel de readiness los valida contra el estado vivo de auth.</p>
					</label>
				</div>
			</section>

			<div class="space-y-6">
				<section class="overflow-hidden rounded-3xl border border-slate-200 bg-white shadow-sm">
					<div class="border-b border-slate-200 px-6 py-4">
						<p class="text-xs font-semibold uppercase tracking-[0.24em] text-slate-500">Branding</p>
						<h2 class="mt-2 text-xl font-semibold text-slate-900">Workspace identity</h2>
					</div>
					<div class="space-y-4 px-6 py-5">
						<label class="block text-sm">
							<span class="mb-2 block font-medium text-slate-700">Display name</span>
							<input class="w-full rounded-2xl border border-slate-200 px-4 py-3 outline-none transition focus:border-emerald-500" bind:value={draft.branding_display_name} />
						</label>
						<div class="grid gap-4 md:grid-cols-2">
							<label class="block text-sm">
								<span class="mb-2 block font-medium text-slate-700">Primary color</span>
								<input type="color" class="h-12 w-full rounded-2xl border border-slate-200 bg-white px-2 py-2" bind:value={draft.branding_primary_color} />
							</label>
							<label class="block text-sm">
								<span class="mb-2 block font-medium text-slate-700">Accent color</span>
								<input type="color" class="h-12 w-full rounded-2xl border border-slate-200 bg-white px-2 py-2" bind:value={draft.branding_accent_color} />
							</label>
						</div>
						<label class="block text-sm">
							<span class="mb-2 block font-medium text-slate-700">Logo URL</span>
							<input class="w-full rounded-2xl border border-slate-200 px-4 py-3 outline-none transition focus:border-emerald-500" bind:value={draft.branding_logo_url} />
						</label>
						<label class="block text-sm">
							<span class="mb-2 block font-medium text-slate-700">Favicon URL</span>
							<input class="w-full rounded-2xl border border-slate-200 px-4 py-3 outline-none transition focus:border-emerald-500" bind:value={draft.branding_favicon_url} />
						</label>
						<label class="flex items-center gap-3 rounded-2xl border border-slate-200 px-4 py-3 text-sm text-slate-700">
							<input type="checkbox" bind:checked={draft.branding_show_powered_by} />
							Show powered-by footer in published apps
						</label>
					</div>
				</section>

				<section class="overflow-hidden rounded-3xl border border-slate-200 shadow-sm">
					<div
						class="px-6 py-5 text-white"
						style={`background: linear-gradient(140deg, ${draft.branding_primary_color}, ${draft.branding_accent_color});`}
					>
						<p class="text-xs font-semibold uppercase tracking-[0.24em] text-white/75">Preview</p>
						<h3 class="mt-3 text-2xl font-semibold">{draft.branding_display_name || draft.platform_name || 'OpenFoundry'}</h3>
						<p class="mt-2 max-w-sm text-sm text-white/85">
							{draft.announcement_banner || 'Platform announcements, rollout windows and support links will surface here for operators and builders.'}
						</p>
					</div>
					<div class="bg-white px-6 py-5 text-sm text-slate-600">
						<div class="flex items-center justify-between gap-4">
							<div>
								<div class="font-medium text-slate-900">{draft.support_email || 'support@openfoundry.dev'}</div>
								<div class="mt-1">{draft.default_region || 'global'} • {draft.release_channel || 'stable'}</div>
							</div>
							<div class={`rounded-full px-3 py-1 text-xs font-semibold uppercase tracking-[0.2em] ${draft.maintenance_mode ? 'bg-amber-100 text-amber-700' : 'bg-emerald-100 text-emerald-700'}`}>
								{draft.maintenance_mode ? 'Maintenance' : 'Healthy'}
							</div>
						</div>
						{#if draft.branding_show_powered_by}
							<div class="mt-4 rounded-2xl border border-dashed border-slate-200 px-4 py-3 text-xs uppercase tracking-[0.24em] text-slate-400">
								Powered by {draft.platform_name || 'OpenFoundry'}
							</div>
						{/if}
					</div>
				</section>

				<section class="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
					<div class="flex items-start justify-between gap-4">
						<div>
							<p class="text-xs font-semibold uppercase tracking-[0.24em] text-slate-500">Upgrade Assistant</p>
							<h2 class="mt-2 text-xl font-semibold text-slate-900">Readiness signal</h2>
						</div>
						{#if upgradeReadiness}
							<span class={`rounded-full px-3 py-1 text-xs font-semibold uppercase tracking-[0.2em] ${upgradeReadiness.readiness === 'ready' ? 'bg-emerald-100 text-emerald-700' : upgradeReadiness.readiness === 'attention' ? 'bg-amber-100 text-amber-700' : 'bg-rose-100 text-rose-700'}`}>
								{upgradeReadiness.readiness}
							</span>
						{/if}
					</div>
					{#if upgradeReadiness}
						<div class="mt-4 rounded-2xl border border-slate-200 bg-slate-50 px-4 py-4 text-sm text-slate-600">
							<div><span class="font-medium text-slate-900">Current:</span> {upgradeReadiness.current_version}</div>
							<div class="mt-2"><span class="font-medium text-slate-900">Target:</span> {upgradeReadiness.target_version}</div>
							<div class="mt-2"><span class="font-medium text-slate-900">Channel:</span> {upgradeReadiness.release_channel}</div>
						</div>
						<div class="mt-4 grid gap-3 sm:grid-cols-3">
							<div class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-4">
								<div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-slate-500">Preflight</div>
								<div class="mt-2 text-lg font-semibold text-slate-900">
									{upgradeReadiness.preflight_ready_count}/{upgradeReadiness.preflight_total_count}
								</div>
								<div class="mt-1 text-xs text-slate-500">checks ready</div>
							</div>
							<div class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-4">
								<div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-slate-500">Rollout</div>
								<div class="mt-2 text-lg font-semibold text-slate-900">
									{upgradeReadiness.completed_stage_count}/{upgradeReadiness.total_stage_count}
								</div>
								<div class="mt-1 text-xs text-slate-500">stages completed</div>
							</div>
							<div class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-4">
								<div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-slate-500">Coverage</div>
								<div class="mt-2 text-lg font-semibold text-slate-900">
									{upgradeReadiness.completed_rollout_percentage}%
								</div>
								<div class="mt-1 text-xs text-slate-500">rollout already completed</div>
							</div>
						</div>
						{#if upgradeReadiness.next_stage}
							<div class="mt-4 rounded-2xl border border-emerald-200 bg-emerald-50 px-4 py-4 text-sm text-emerald-900">
								<div class="text-xs font-semibold uppercase tracking-[0.18em] text-emerald-700">Next stage</div>
								<div class="mt-2 font-semibold">{upgradeReadiness.next_stage.label}</div>
								<div class="mt-1 text-emerald-800">
									{upgradeReadiness.next_stage.rollout_percentage}% rollout target • status {upgradeReadiness.next_stage.status}
								</div>
							</div>
						{/if}
						{#if upgradeReadiness.blockers.length > 0}
							<div class="mt-4 rounded-2xl border border-rose-200 bg-rose-50 px-4 py-4 text-sm text-rose-800">
								<div class="text-xs font-semibold uppercase tracking-[0.18em] text-rose-700">Blockers</div>
								<div class="mt-3 space-y-2">
									{#each upgradeReadiness.blockers as blocker}
										<div>{blocker}</div>
									{/each}
								</div>
							</div>
						{/if}
						{#if upgradeReadiness.recommended_actions.length > 0}
							<div class="mt-4 rounded-2xl border border-amber-200 bg-amber-50 px-4 py-4 text-sm text-amber-900">
								<div class="text-xs font-semibold uppercase tracking-[0.18em] text-amber-700">Recommended actions</div>
								<div class="mt-3 space-y-2">
									{#each upgradeReadiness.recommended_actions as action}
										<div>{action}</div>
									{/each}
								</div>
							</div>
						{/if}
						<div class="mt-4 space-y-3">
							{#each upgradeReadiness.checks as check}
								<div class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-4">
									<div class="flex items-start justify-between gap-3">
										<div>
											<div class="font-medium text-slate-900">{check.label}</div>
											<div class="mt-1 text-xs text-slate-500">{check.id}</div>
										</div>
										<span class={`rounded-full px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] ${check.status === 'ready' ? 'bg-emerald-100 text-emerald-700' : check.status === 'warning' ? 'bg-amber-100 text-amber-700' : 'bg-rose-100 text-rose-700'}`}>
											{check.status}
										</span>
									</div>
									<p class="mt-3 text-sm text-slate-600">{check.detail}</p>
								</div>
							{/each}
						</div>
					{:else}
						<div class="mt-4 text-sm text-slate-500">Upgrade readiness unavailable.</div>
					{/if}
				</section>

				<section class="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
					<div class="flex items-start justify-between gap-4">
						<div>
							<p class="text-xs font-semibold uppercase tracking-[0.24em] text-slate-500">Identity Mapping</p>
							<h2 class="mt-2 text-xl font-semibold text-slate-900">IdP assignment preview</h2>
							<p class="mt-2 text-sm text-slate-500">
								Simula la resolución real de org, workspace, clearance, roles y tenant tier antes de tocar producción.
							</p>
						</div>
						<button
							class="rounded-full bg-slate-900 px-4 py-2 text-sm font-semibold text-white transition hover:bg-slate-700 disabled:cursor-not-allowed disabled:bg-slate-300"
							onclick={runIdentityMappingPreview}
							disabled={previewLoading || !canReadControlPanel()}
						>
							{previewLoading ? 'Previewing...' : 'Run preview'}
						</button>
					</div>
					<div class="mt-5 grid gap-4">
						<label class="block text-sm">
							<span class="mb-2 block font-medium text-slate-700">Provider</span>
							<select class="w-full rounded-2xl border border-slate-200 px-4 py-3 outline-none transition focus:border-emerald-500" bind:value={idpPreviewProviderSlug}>
								<option value="">Select provider</option>
								{#each ssoProviders as provider}
									<option value={provider.slug}>{provider.name} • {provider.slug}</option>
								{/each}
							</select>
						</label>
						<label class="block text-sm">
							<span class="mb-2 block font-medium text-slate-700">User email</span>
							<input class="w-full rounded-2xl border border-slate-200 px-4 py-3 outline-none transition focus:border-emerald-500" bind:value={idpPreviewEmail} />
						</label>
						<label class="block text-sm">
							<span class="mb-2 block font-medium text-slate-700">Raw claims JSON</span>
							<textarea class="min-h-40 w-full rounded-2xl border border-slate-200 px-4 py-3 font-mono text-xs outline-none transition focus:border-emerald-500" bind:value={idpPreviewRawClaimsJson}></textarea>
						</label>
					</div>
					{#if idpPreviewError}
						<div class="mt-4 rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700">
							{idpPreviewError}
						</div>
					{/if}
					{#if idpPreviewResult}
						<div class="mt-4 rounded-2xl border border-slate-200 bg-slate-50 px-4 py-4">
							<div class="flex items-start justify-between gap-3">
								<div>
									<div class="text-xs font-semibold uppercase tracking-[0.18em] text-slate-500">Resolved identity</div>
									<div class="mt-2 text-lg font-semibold text-slate-900">{idpPreviewResult.email}</div>
									<div class="mt-1 text-xs text-slate-500">{idpPreviewResult.provider_slug}</div>
								</div>
								<span class={`rounded-full px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] ${idpPreviewResult.mapping_found ? 'bg-emerald-100 text-emerald-700' : 'bg-amber-100 text-amber-700'}`}>
									{idpPreviewResult.mapping_found ? 'mapped' : 'provider-only'}
								</span>
							</div>
							<div class="mt-4 grid gap-3 sm:grid-cols-2">
								<div>
									<div class="text-xs uppercase tracking-[0.18em] text-slate-500">Matched rule</div>
									<div class="mt-1 font-medium text-slate-900">{idpPreviewResult.matched_rule_name ?? 'None'}</div>
								</div>
								<div>
									<div class="text-xs uppercase tracking-[0.18em] text-slate-500">Organization</div>
									<div class="mt-1 font-medium text-slate-900">{idpPreviewResult.organization_id ?? 'Unassigned'}</div>
								</div>
								<div>
									<div class="text-xs uppercase tracking-[0.18em] text-slate-500">Workspace</div>
									<div class="mt-1 font-medium text-slate-900">{idpPreviewResult.workspace ?? 'None'}</div>
								</div>
								<div>
									<div class="text-xs uppercase tracking-[0.18em] text-slate-500">Clearance</div>
									<div class="mt-1 font-medium text-slate-900">{idpPreviewResult.classification_clearance ?? 'None'}</div>
								</div>
								<div>
									<div class="text-xs uppercase tracking-[0.18em] text-slate-500">Tenant tier</div>
									<div class="mt-1 font-medium text-slate-900">{idpPreviewResult.tenant_tier ?? 'None'}</div>
								</div>
								<div>
									<div class="text-xs uppercase tracking-[0.18em] text-slate-500">Resource policy</div>
									<div class="mt-1 font-medium text-slate-900">{idpPreviewResult.resource_policy_name ?? 'None'}</div>
								</div>
							</div>
							<div class="mt-4">
								<div class="text-xs uppercase tracking-[0.18em] text-slate-500">Roles</div>
								<div class="mt-2 flex flex-wrap gap-2">
									{#if idpPreviewResult.role_names.length === 0}
										<span class="rounded-full bg-slate-200 px-3 py-1 text-xs font-medium text-slate-700">No roles</span>
									{:else}
										{#each idpPreviewResult.role_names as role}
											<span class="rounded-full bg-slate-900 px-3 py-1 text-xs font-medium text-white">{role}</span>
										{/each}
									{/if}
								</div>
							</div>
							{#if idpPreviewResult.quota}
								<div class="mt-4 grid gap-3 sm:grid-cols-2">
									<div class="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm text-slate-600">
										<div><span class="font-medium text-slate-900">Query limit:</span> {idpPreviewResult.quota.max_query_limit}</div>
										<div class="mt-2"><span class="font-medium text-slate-900">RPM:</span> {idpPreviewResult.quota.requests_per_minute}</div>
										<div class="mt-2"><span class="font-medium text-slate-900">Storage:</span> {idpPreviewResult.quota.max_storage_gb} GB</div>
									</div>
									<div class="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm text-slate-600">
										<div><span class="font-medium text-slate-900">Pipeline workers:</span> {idpPreviewResult.quota.max_pipeline_workers}</div>
										<div class="mt-2"><span class="font-medium text-slate-900">Shared spaces:</span> {idpPreviewResult.quota.max_shared_spaces}</div>
										<div class="mt-2"><span class="font-medium text-slate-900">Guest sessions:</span> {idpPreviewResult.quota.max_guest_sessions}</div>
									</div>
								</div>
							{/if}
							{#if idpPreviewResult.notes.length > 0}
								<div class="mt-4 rounded-2xl border border-dashed border-slate-300 bg-white px-4 py-4 text-sm text-slate-600">
									<div class="text-xs font-semibold uppercase tracking-[0.18em] text-slate-500">Notes</div>
									<div class="mt-3 space-y-2">
										{#each idpPreviewResult.notes as note}
											<div>{note}</div>
										{/each}
									</div>
								</div>
							{/if}
						</div>
					{/if}
				</section>

				<section class="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
					<p class="text-xs font-semibold uppercase tracking-[0.24em] text-slate-500">Identity</p>
					<h2 class="mt-2 text-xl font-semibold text-slate-900">Enabled SSO providers</h2>
					<div class="mt-4 space-y-3">
						{#if ssoProviders.length === 0}
							<div class="rounded-2xl border border-dashed border-slate-200 px-4 py-4 text-sm text-slate-500">
								No SSO providers configured yet.
							</div>
						{:else}
							{#each ssoProviders as provider}
								<div class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-4">
									<div class="flex items-start justify-between gap-3">
										<div>
											<div class="font-medium text-slate-900">{provider.name}</div>
											<div class="mt-1 text-xs text-slate-500">{provider.slug} • {provider.provider_type}</div>
										</div>
										<span class={`rounded-full px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] ${provider.enabled ? 'bg-emerald-100 text-emerald-700' : 'bg-slate-200 text-slate-700'}`}>
											{provider.enabled ? 'enabled' : 'disabled'}
										</span>
									</div>
								</div>
							{/each}
						{/if}
					</div>
				</section>

				<section class="rounded-3xl border border-slate-200 bg-slate-50 p-5 shadow-sm">
					<p class="text-xs font-semibold uppercase tracking-[0.24em] text-slate-500">Audit Trail</p>
					<div class="mt-3 text-sm text-slate-600">
						<div><span class="font-medium text-slate-900">Last update:</span> {settings?.updated_at ?? 'unknown'}</div>
						<div class="mt-2"><span class="font-medium text-slate-900">Updated by:</span> {settings?.updated_by ?? 'system'}</div>
					</div>
				</section>
			</div>
		</div>
	{/if}
</div>
