<script lang="ts">
  import { goto } from '$app/navigation';
  import { ApiError } from '$api/client';
  import {
    addUserToGroup,
    assignUserRole,
    createApiKey,
    createGroup,
    createPermission,
    createPolicy,
    createRole,
    createSsoProvider,
    deactivateUser,
    deletePolicy,
    deleteSsoProvider,
    disableMfa,
    enrollMfa,
    evaluatePolicy,
    getMfaStatus,
    listApiKeys,
    listGroups,
    listPermissions,
    listPolicies,
    listRoles,
    listSsoProviders,
    listUsers,
    removeUserFromGroup,
    removeUserRole,
    revokeApiKey,
    updateUser,
    verifyMfaSetup,
    type ApiKeyRecord,
    type ApiKeyWithSecret,
    type GroupRecord,
    type MfaEnrollmentResponse,
    type MfaStatusResponse,
    type PermissionRecord,
    type PolicyEvaluationResult,
    type PolicyRecord,
    type RoleRecord,
    type SsoProviderRecord,
    type UserProfile,
  } from '$api/auth';
  import { auth } from '$stores/auth';
  import { get } from 'svelte/store';
  import { onMount } from 'svelte';

  const currentUser = auth.user;
  const isAuthenticated = auth.isAuthenticated;

  let loading = $state(true);
  let error = $state('');
  let notice = $state('');
  let saving = $state<string | null>(null);

  let users = $state<UserProfile[]>([]);
  let permissions = $state<PermissionRecord[]>([]);
  let roles = $state<RoleRecord[]>([]);
  let groups = $state<GroupRecord[]>([]);
  let policies = $state<PolicyRecord[]>([]);
  let apiKeys = $state<ApiKeyRecord[]>([]);
  let mfaStatus = $state<MfaStatusResponse | null>(null);
  let mfaEnrollment = $state<MfaEnrollmentResponse | null>(null);
  let ssoProviders = $state<SsoProviderRecord[]>([]);
  let newApiKey = $state<ApiKeyWithSecret | null>(null);
  let policyEvaluation = $state<PolicyEvaluationResult | null>(null);

  let selectedRoleByUser = $state<Record<string, string>>({});
  let selectedGroupByUser = $state<Record<string, string>>({});
  let rolePermissionIds = $state<string[]>([]);
  let groupRoleIds = $state<string[]>([]);

  let permissionForm = $state({ resource: '', action: '', description: '' });
  let roleForm = $state({ name: '', description: '' });
  let groupForm = $state({ name: '', description: '' });
  let policyForm = $state({
    name: '',
    description: '',
    effect: 'allow',
    resource: 'datasets',
    action: 'read',
    conditions: '{\n  "subject": {},\n  "resource": {}\n}',
    row_filter: '',
    enabled: true,
  });
  let policyEvaluationForm = $state({
    resource: 'datasets',
    action: 'read',
    resource_attributes: '{\n  "organization_id": null\n}',
  });
  let apiKeyForm = $state({ name: '', scopes: '', expires_at: '' });
  let mfaVerifyCode = $state('');
  let mfaDisableCode = $state('');
  let ssoForm = $state({
    slug: '',
    name: '',
    provider_type: 'oidc',
    enabled: true,
    client_id: '',
    client_secret: '',
    issuer_url: '',
    authorization_url: '',
    token_url: '',
    userinfo_url: '',
    scopes: 'openid,profile,email',
    saml_metadata_url: '',
    saml_entity_id: '',
    saml_sso_url: '',
    saml_certificate: '',
    attribute_mapping: '{\n  "subject": "sub",\n  "email": "email",\n  "name": "name"\n}',
  });

  function hasPermission(permission: string) {
    return get(currentUser)?.permissions.includes(permission) ?? false;
  }

  function canReadUsers() {
    return hasPermission('users:read') || hasPermission('users:write');
  }

  function canManageUsers() {
    return hasPermission('users:write');
  }

  function canReadRoles() {
    return hasPermission('roles:read') || hasPermission('roles:write');
  }

  function canManageRoles() {
    return hasPermission('roles:write');
  }

  function canReadGroups() {
    return hasPermission('groups:read') || hasPermission('groups:write');
  }

  function canManageGroups() {
    return hasPermission('groups:write');
  }

  function canReadPermissions() {
    return hasPermission('permissions:read') || hasPermission('permissions:write');
  }

  function canManagePermissions() {
    return hasPermission('permissions:write');
  }

  function canReadPolicies() {
    return hasPermission('policies:read') || hasPermission('policies:write');
  }

  function canManagePolicies() {
    return hasPermission('policies:write');
  }

  function canReadSso() {
    return hasPermission('sso:read') || hasPermission('sso:write');
  }

  function canManageSso() {
    return hasPermission('sso:write');
  }

  function toOptionalString(value: string) {
    const trimmed = value.trim();
    return trimmed ? trimmed : null;
  }

  function toScopes(value: string) {
    return value
      .split(',')
      .map((entry) => entry.trim())
      .filter(Boolean);
  }

  function parseJson(value: string) {
    return value.trim() ? JSON.parse(value) : {};
  }

  function toIsoDateTime(value: string) {
    return value ? new Date(value).toISOString() : null;
  }

  function toggleSelection(entries: string[], value: string) {
    return entries.includes(value)
      ? entries.filter((entry) => entry !== value)
      : [...entries, value];
  }

  function roleIdByName(roleName: string) {
    return roles.find((role) => role.name === roleName)?.id;
  }

  function groupIdByName(groupName: string) {
    return groups.find((group) => group.name === groupName)?.id;
  }

  function isForbidden(error: unknown) {
    return error instanceof ApiError && (error.status === 403 || error.status === 404);
  }

  async function loadOptional<T>(loader: () => Promise<T>, assign: (value: T) => void) {
    try {
      assign(await loader());
    } catch (err) {
      if (!isForbidden(err)) {
        throw err;
      }
    }
  }

  async function refreshUsers() {
    if (!canReadUsers()) return;
    users = await listUsers();
  }

  async function refreshPermissions() {
    if (!canReadPermissions()) return;
    permissions = await listPermissions();
  }

  async function refreshRoles() {
    if (!canReadRoles()) return;
    roles = await listRoles();
  }

  async function refreshGroups() {
    if (!canReadGroups()) return;
    groups = await listGroups();
  }

  async function refreshPolicies() {
    if (!canReadPolicies()) return;
    policies = await listPolicies();
  }

  async function refreshMfa() {
    await loadOptional(getMfaStatus, (value) => {
      mfaStatus = value;
    });
  }

  async function refreshApiKeys() {
    await loadOptional(listApiKeys, (value) => {
      apiKeys = value;
    });
  }

  async function refreshSsoProviders() {
    if (!canReadSso()) return;
    ssoProviders = await listSsoProviders();
  }

  async function loadSettings() {
    loading = true;
    error = '';

    try {
      await auth.restore();
      if (!get(isAuthenticated)) {
        goto('/auth/login');
        return;
      }

      await Promise.all([
        refreshUsers(),
        refreshPermissions(),
        refreshRoles(),
        refreshGroups(),
        refreshPolicies(),
        refreshMfa(),
        refreshApiKeys(),
        refreshSsoProviders(),
      ]);
    } catch (err: any) {
      error = err.message ?? 'Failed to load enterprise settings';
    } finally {
      loading = false;
    }
  }

  async function withSaving(key: string, work: () => Promise<void>) {
    saving = key;
    error = '';
    notice = '';

    try {
      await work();
    } catch (err: any) {
      error = err.message ?? 'Request failed';
    } finally {
      saving = null;
    }
  }

  async function handleCreatePermission() {
    await withSaving('permission', async () => {
      await createPermission({
        resource: permissionForm.resource,
        action: permissionForm.action,
        description: toOptionalString(permissionForm.description),
      });
      permissionForm = { resource: '', action: '', description: '' };
      await refreshPermissions();
      await refreshRoles();
      notice = 'Permission created.';
    });
  }

  async function handleCreateRole() {
    await withSaving('role', async () => {
      await createRole({
        name: roleForm.name,
        description: toOptionalString(roleForm.description),
        permission_ids: rolePermissionIds,
      });
      roleForm = { name: '', description: '' };
      rolePermissionIds = [];
      await refreshRoles();
      notice = 'Role created.';
    });
  }

  async function handleCreateGroup() {
    await withSaving('group', async () => {
      await createGroup({
        name: groupForm.name,
        description: toOptionalString(groupForm.description),
        role_ids: groupRoleIds,
      });
      groupForm = { name: '', description: '' };
      groupRoleIds = [];
      await refreshGroups();
      notice = 'Group created.';
    });
  }

  async function handleToggleUser(userProfile: UserProfile) {
    await withSaving(`user-${userProfile.id}`, async () => {
      if (userProfile.is_active) {
        await deactivateUser(userProfile.id);
      } else {
        await updateUser(userProfile.id, { is_active: true });
      }
      await refreshUsers();
      notice = 'User state updated.';
    });
  }

  async function handleToggleMfaEnforcement(userProfile: UserProfile) {
    await withSaving(`user-mfa-${userProfile.id}`, async () => {
      await updateUser(userProfile.id, { mfa_enforced: !userProfile.mfa_enforced });
      await refreshUsers();
      notice = 'MFA enforcement updated.';
    });
  }

  async function handleAssignRole(userId: string) {
    const roleId = selectedRoleByUser[userId];
    if (!roleId) return;

    await withSaving(`assign-role-${userId}`, async () => {
      await assignUserRole(userId, roleId);
      await refreshUsers();
      selectedRoleByUser = { ...selectedRoleByUser, [userId]: '' };
      notice = 'Role assigned.';
    });
  }

  async function handleRemoveRole(userId: string, roleName: string) {
    const roleId = roleIdByName(roleName);
    if (!roleId) return;

    await withSaving(`remove-role-${userId}-${roleId}`, async () => {
      await removeUserRole(userId, roleId);
      await refreshUsers();
      notice = 'Role removed.';
    });
  }

  async function handleAddUserToGroup(userId: string) {
    const groupId = selectedGroupByUser[userId];
    if (!groupId) return;

    await withSaving(`assign-group-${userId}`, async () => {
      await addUserToGroup(userId, groupId);
      await refreshUsers();
      await refreshGroups();
      selectedGroupByUser = { ...selectedGroupByUser, [userId]: '' };
      notice = 'User added to group.';
    });
  }

  async function handleRemoveUserFromGroup(userId: string, groupName: string) {
    const groupId = groupIdByName(groupName);
    if (!groupId) return;

    await withSaving(`remove-group-${userId}-${groupId}`, async () => {
      await removeUserFromGroup(userId, groupId);
      await refreshUsers();
      await refreshGroups();
      notice = 'User removed from group.';
    });
  }

  async function handleCreatePolicy() {
    await withSaving('policy', async () => {
      await createPolicy({
        name: policyForm.name,
        description: toOptionalString(policyForm.description),
        effect: policyForm.effect,
        resource: policyForm.resource,
        action: policyForm.action,
        conditions: parseJson(policyForm.conditions),
        row_filter: toOptionalString(policyForm.row_filter),
        enabled: policyForm.enabled,
      });
      policyForm = {
        name: '',
        description: '',
        effect: 'allow',
        resource: 'datasets',
        action: 'read',
        conditions: '{\n  "subject": {},\n  "resource": {}\n}',
        row_filter: '',
        enabled: true,
      };
      await refreshPolicies();
      notice = 'Policy created.';
    });
  }

  async function handleDeletePolicy(policyId: string) {
    await withSaving(`delete-policy-${policyId}`, async () => {
      await deletePolicy(policyId);
      await refreshPolicies();
      notice = 'Policy deleted.';
    });
  }

  async function handleEvaluatePolicy() {
    await withSaving('evaluate-policy', async () => {
      policyEvaluation = await evaluatePolicy({
        resource: policyEvaluationForm.resource,
        action: policyEvaluationForm.action,
        resource_attributes: parseJson(policyEvaluationForm.resource_attributes),
      });
      notice = 'Policy evaluation completed.';
    });
  }

  async function handleEnrollMfa() {
    await withSaving('enroll-mfa', async () => {
      mfaEnrollment = await enrollMfa();
      await refreshMfa();
      notice = 'MFA enrollment secret generated.';
    });
  }

  async function handleVerifyMfa() {
    await withSaving('verify-mfa', async () => {
      await verifyMfaSetup({ code: mfaVerifyCode });
      mfaVerifyCode = '';
      await refreshMfa();
      notice = 'MFA enabled.';
    });
  }

  async function handleDisableMfa() {
    await withSaving('disable-mfa', async () => {
      await disableMfa({ code: mfaDisableCode });
      mfaDisableCode = '';
      mfaEnrollment = null;
      await refreshMfa();
      notice = 'MFA disabled.';
    });
  }

  async function handleCreateApiKey() {
    await withSaving('api-key', async () => {
      newApiKey = await createApiKey({
        name: apiKeyForm.name,
        scopes: toScopes(apiKeyForm.scopes),
        expires_at: toIsoDateTime(apiKeyForm.expires_at),
      });
      apiKeyForm = { name: '', scopes: '', expires_at: '' };
      await refreshApiKeys();
      notice = 'API key created. Copy the token now; it will not be shown again.';
    });
  }

  async function handleRevokeApiKey(apiKeyId: string) {
    await withSaving(`api-key-${apiKeyId}`, async () => {
      await revokeApiKey(apiKeyId);
      await refreshApiKeys();
      notice = 'API key revoked.';
    });
  }

  async function handleCreateSsoProvider() {
    await withSaving('sso-provider', async () => {
      await createSsoProvider({
        slug: ssoForm.slug,
        name: ssoForm.name,
        provider_type: ssoForm.provider_type,
        enabled: ssoForm.enabled,
        client_id: toOptionalString(ssoForm.client_id),
        client_secret: toOptionalString(ssoForm.client_secret),
        issuer_url: toOptionalString(ssoForm.issuer_url),
        authorization_url: toOptionalString(ssoForm.authorization_url),
        token_url: toOptionalString(ssoForm.token_url),
        userinfo_url: toOptionalString(ssoForm.userinfo_url),
        scopes: toScopes(ssoForm.scopes),
        saml_metadata_url: toOptionalString(ssoForm.saml_metadata_url),
        saml_entity_id: toOptionalString(ssoForm.saml_entity_id),
        saml_sso_url: toOptionalString(ssoForm.saml_sso_url),
        saml_certificate: toOptionalString(ssoForm.saml_certificate),
        attribute_mapping: parseJson(ssoForm.attribute_mapping),
      });
      ssoForm = {
        slug: '',
        name: '',
        provider_type: 'oidc',
        enabled: true,
        client_id: '',
        client_secret: '',
        issuer_url: '',
        authorization_url: '',
        token_url: '',
        userinfo_url: '',
        scopes: 'openid,profile,email',
        saml_metadata_url: '',
        saml_entity_id: '',
        saml_sso_url: '',
        saml_certificate: '',
        attribute_mapping: '{\n  "subject": "sub",\n  "email": "email",\n  "name": "name"\n}',
      };
      await refreshSsoProviders();
      notice = 'SSO provider created.';
    });
  }

  async function handleDeleteSsoProvider(providerId: string) {
    await withSaving(`delete-sso-${providerId}`, async () => {
      await deleteSsoProvider(providerId);
      await refreshSsoProviders();
      notice = 'SSO provider deleted.';
    });
  }

  onMount(() => {
    loadSettings();
  });
</script>

<svelte:head>
  <title>Enterprise Auth Settings — OpenFoundry</title>
</svelte:head>

{#if loading}
  <div class="mx-auto max-w-7xl space-y-4">
    <div class="h-28 animate-pulse rounded-3xl bg-gray-100 dark:bg-gray-900"></div>
    <div class="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
      {#each Array.from({ length: 4 }) as _}
        <div class="h-32 animate-pulse rounded-3xl bg-gray-100 dark:bg-gray-900"></div>
      {/each}
    </div>
  </div>
{:else}
  <div class="mx-auto max-w-7xl space-y-8">
    <section class="rounded-[2rem] border border-gray-200 bg-white p-8 shadow-sm dark:border-gray-800 dark:bg-gray-900">
      <div class="flex flex-col gap-6 lg:flex-row lg:items-end lg:justify-between">
        <div>
          <div class="text-xs uppercase tracking-[0.3em] text-indigo-500">Phase 2.1</div>
          <h1 class="mt-3 text-3xl font-bold tracking-tight">Enterprise Authentication Control Plane</h1>
          <p class="mt-3 max-w-3xl text-sm text-gray-500 dark:text-gray-400">
            Manage platform access across RBAC, ABAC, MFA, API keys, and OIDC providers from a single surface.
          </p>
        </div>

        <div class="rounded-2xl bg-gray-50 px-5 py-4 text-sm dark:bg-gray-950">
          <div class="text-xs uppercase tracking-[0.24em] text-gray-400">Signed in as</div>
          <div class="mt-2 font-semibold">{$currentUser?.name ?? 'Unknown user'}</div>
          <div class="text-gray-500">{$currentUser?.email ?? ''}</div>
        </div>
      </div>

      {#if error}
        <div class="mt-6 rounded-2xl bg-red-50 px-4 py-3 text-sm text-red-700 dark:bg-red-950 dark:text-red-300">{error}</div>
      {/if}

      {#if notice}
        <div class="mt-6 rounded-2xl bg-emerald-50 px-4 py-3 text-sm text-emerald-700 dark:bg-emerald-950 dark:text-emerald-300">{notice}</div>
      {/if}
    </section>

    <section class="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
      <div class="rounded-3xl border border-gray-200 bg-white p-6 shadow-sm dark:border-gray-800 dark:bg-gray-900">
        <div class="text-xs uppercase tracking-[0.2em] text-gray-400">Users</div>
        <div class="mt-3 text-3xl font-semibold">{users.length}</div>
        <div class="mt-2 text-sm text-gray-500">{users.filter((entry) => entry.is_active).length} active identities</div>
      </div>
      <div class="rounded-3xl border border-gray-200 bg-white p-6 shadow-sm dark:border-gray-800 dark:bg-gray-900">
        <div class="text-xs uppercase tracking-[0.2em] text-gray-400">Roles</div>
        <div class="mt-3 text-3xl font-semibold">{roles.length}</div>
        <div class="mt-2 text-sm text-gray-500">{permissions.length} available permissions</div>
      </div>
      <div class="rounded-3xl border border-gray-200 bg-white p-6 shadow-sm dark:border-gray-800 dark:bg-gray-900">
        <div class="text-xs uppercase tracking-[0.2em] text-gray-400">MFA</div>
        <div class="mt-3 text-3xl font-semibold">{mfaStatus?.enabled ? 'On' : 'Off'}</div>
        <div class="mt-2 text-sm text-gray-500">{mfaStatus?.recovery_codes_remaining ?? 0} recovery codes remaining</div>
      </div>
      <div class="rounded-3xl border border-gray-200 bg-white p-6 shadow-sm dark:border-gray-800 dark:bg-gray-900">
        <div class="text-xs uppercase tracking-[0.2em] text-gray-400">SSO Providers</div>
        <div class="mt-3 text-3xl font-semibold">{ssoProviders.length}</div>
        <div class="mt-2 text-sm text-gray-500">{ssoProviders.filter((provider) => provider.enabled).length} enabled connections</div>
      </div>
    </section>

    {#if canReadUsers()}
      <section class="rounded-[2rem] border border-gray-200 bg-white p-8 shadow-sm dark:border-gray-800 dark:bg-gray-900">
        <div class="flex items-center justify-between gap-4">
          <div>
            <div class="text-xs uppercase tracking-[0.24em] text-gray-400">User directory</div>
            <h2 class="mt-2 text-2xl font-semibold">Users, roles and group membership</h2>
          </div>
        </div>

        <div class="mt-6 grid gap-4 xl:grid-cols-2">
          {#each users as platformUser}
            <article class="rounded-3xl border border-gray-200 bg-gray-50 p-5 dark:border-gray-800 dark:bg-gray-950">
              <div class="flex items-start justify-between gap-4">
                <div>
                  <h3 class="text-lg font-semibold">{platformUser.name}</h3>
                  <p class="text-sm text-gray-500">{platformUser.email}</p>
                </div>
                <div class="text-right text-xs text-gray-500">
                  <div class={`inline-flex rounded-full px-2 py-1 font-medium ${platformUser.is_active ? 'bg-emerald-100 text-emerald-700 dark:bg-emerald-950 dark:text-emerald-300' : 'bg-gray-200 text-gray-600 dark:bg-gray-800 dark:text-gray-300'}`}>
                    {platformUser.is_active ? 'Active' : 'Inactive'}
                  </div>
                  <div class="mt-2 uppercase tracking-[0.18em]">{platformUser.auth_source}</div>
                </div>
              </div>

              <div class="mt-4 grid gap-4 text-sm md:grid-cols-2">
                <div>
                  <div class="text-xs uppercase tracking-[0.2em] text-gray-400">Roles</div>
                  <div class="mt-2 flex flex-wrap gap-2">
                    {#if platformUser.roles.length > 0}
                      {#each platformUser.roles as roleName}
                        <span class="inline-flex items-center gap-2 rounded-full bg-indigo-100 px-3 py-1 text-xs font-medium text-indigo-700 dark:bg-indigo-950 dark:text-indigo-300">
                          {roleName}
                          {#if canManageRoles()}
                            <button onclick={() => handleRemoveRole(platformUser.id, roleName)} class="text-indigo-500 hover:text-indigo-700">x</button>
                          {/if}
                        </span>
                      {/each}
                    {:else}
                      <span class="text-gray-400">No direct roles</span>
                    {/if}
                  </div>
                </div>

                <div>
                  <div class="text-xs uppercase tracking-[0.2em] text-gray-400">Groups</div>
                  <div class="mt-2 flex flex-wrap gap-2">
                    {#if platformUser.groups.length > 0}
                      {#each platformUser.groups as groupName}
                        <span class="inline-flex items-center gap-2 rounded-full bg-amber-100 px-3 py-1 text-xs font-medium text-amber-700 dark:bg-amber-950 dark:text-amber-300">
                          {groupName}
                          {#if canManageGroups()}
                            <button onclick={() => handleRemoveUserFromGroup(platformUser.id, groupName)} class="text-amber-500 hover:text-amber-700">x</button>
                          {/if}
                        </span>
                      {/each}
                    {:else}
                      <span class="text-gray-400">No groups</span>
                    {/if}
                  </div>
                </div>
              </div>

              <div class="mt-4 text-sm text-gray-500">
                <span class="font-medium text-gray-700 dark:text-gray-200">Permissions:</span>
                {platformUser.permissions.length}
                <span class="mx-2">·</span>
                <span class="font-medium text-gray-700 dark:text-gray-200">Org:</span>
                {platformUser.organization_id ?? 'Not assigned'}
              </div>

              {#if canManageUsers() || canManageRoles() || canManageGroups()}
                <div class="mt-5 grid gap-3 md:grid-cols-2">
                  {#if canManageUsers()}
                    <div class="rounded-2xl border border-gray-200 bg-white p-4 dark:border-gray-800 dark:bg-gray-900">
                      <div class="text-xs uppercase tracking-[0.18em] text-gray-400">Identity controls</div>
                      <div class="mt-3 flex flex-wrap gap-2">
                        <button
                          onclick={() => handleToggleUser(platformUser)}
                          class="rounded-xl border border-gray-300 px-3 py-2 text-sm font-medium hover:border-indigo-400 dark:border-gray-700"
                        >
                          {platformUser.is_active ? 'Deactivate user' : 'Reactivate user'}
                        </button>
                        <button
                          onclick={() => handleToggleMfaEnforcement(platformUser)}
                          class="rounded-xl border border-gray-300 px-3 py-2 text-sm font-medium hover:border-indigo-400 dark:border-gray-700"
                        >
                          {platformUser.mfa_enforced ? 'Unset MFA enforcement' : 'Force MFA'}
                        </button>
                      </div>
                    </div>
                  {/if}

                  {#if canManageRoles() || canManageGroups()}
                    <div class="rounded-2xl border border-gray-200 bg-white p-4 dark:border-gray-800 dark:bg-gray-900">
                      <div class="text-xs uppercase tracking-[0.18em] text-gray-400">Access assignment</div>
                      {#if canManageRoles()}
                        <div class="mt-3 flex gap-2">
                          <select
                            value={selectedRoleByUser[platformUser.id] ?? ''}
                            onchange={(event) => {
                              selectedRoleByUser = {
                                ...selectedRoleByUser,
                                [platformUser.id]: (event.currentTarget as HTMLSelectElement).value,
                              };
                            }}
                            class="min-w-0 flex-1 rounded-xl border border-gray-300 bg-white px-3 py-2 text-sm dark:border-gray-700 dark:bg-gray-900"
                          >
                            <option value="">Assign role...</option>
                            {#each roles as role}
                              <option value={role.id}>{role.name}</option>
                            {/each}
                          </select>
                          <button onclick={() => handleAssignRole(platformUser.id)} class="rounded-xl bg-indigo-600 px-3 py-2 text-sm font-medium text-white hover:bg-indigo-500">Assign</button>
                        </div>
                      {/if}

                      {#if canManageGroups()}
                        <div class="mt-3 flex gap-2">
                          <select
                            value={selectedGroupByUser[platformUser.id] ?? ''}
                            onchange={(event) => {
                              selectedGroupByUser = {
                                ...selectedGroupByUser,
                                [platformUser.id]: (event.currentTarget as HTMLSelectElement).value,
                              };
                            }}
                            class="min-w-0 flex-1 rounded-xl border border-gray-300 bg-white px-3 py-2 text-sm dark:border-gray-700 dark:bg-gray-900"
                          >
                            <option value="">Add to group...</option>
                            {#each groups as group}
                              <option value={group.id}>{group.name}</option>
                            {/each}
                          </select>
                          <button onclick={() => handleAddUserToGroup(platformUser.id)} class="rounded-xl border border-gray-300 px-3 py-2 text-sm font-medium hover:border-indigo-400 dark:border-gray-700">Add</button>
                        </div>
                      {/if}
                    </div>
                  {/if}
                </div>
              {/if}
            </article>
          {/each}
        </div>
      </section>
    {/if}

    <div class="grid gap-8 xl:grid-cols-2">
      {#if canReadRoles()}
        <section class="rounded-[2rem] border border-gray-200 bg-white p-8 shadow-sm dark:border-gray-800 dark:bg-gray-900">
          <div class="text-xs uppercase tracking-[0.24em] text-gray-400">RBAC</div>
          <h2 class="mt-2 text-2xl font-semibold">Roles and permissions</h2>

          <div class="mt-6 space-y-4">
            {#each roles as role}
              <article class="rounded-2xl border border-gray-200 bg-gray-50 p-4 dark:border-gray-800 dark:bg-gray-950">
                <div class="flex items-start justify-between gap-4">
                  <div>
                    <h3 class="font-semibold">{role.name}</h3>
                    <p class="text-sm text-gray-500">{role.description ?? 'No description'}</p>
                  </div>
                  <div class="text-xs uppercase tracking-[0.18em] text-gray-400">{role.permissions.length} permissions</div>
                </div>
                <div class="mt-3 flex flex-wrap gap-2">
                  {#each role.permissions as permission}
                    <span class="rounded-full bg-indigo-100 px-3 py-1 text-xs font-medium text-indigo-700 dark:bg-indigo-950 dark:text-indigo-300">{permission}</span>
                  {/each}
                </div>
              </article>
            {/each}
          </div>

          {#if canManageRoles()}
            <form class="mt-8 space-y-4 rounded-3xl border border-dashed border-gray-300 p-5 dark:border-gray-700" onsubmit={(event) => { event.preventDefault(); handleCreateRole(); }}>
              <div class="text-xs uppercase tracking-[0.2em] text-gray-400">Create role</div>
              <input bind:value={roleForm.name} required placeholder="Role name" class="w-full rounded-xl border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
              <textarea bind:value={roleForm.description} rows="2" placeholder="Role description" class="w-full rounded-xl border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900"></textarea>
              <div>
                <div class="mb-2 text-sm font-medium">Permissions</div>
                <div class="grid max-h-52 gap-2 overflow-auto rounded-2xl border border-gray-200 p-3 text-sm dark:border-gray-800">
                  {#each permissions as permission}
                    <label class="flex items-center gap-2">
                      <input type="checkbox" checked={rolePermissionIds.includes(permission.id)} onchange={() => { rolePermissionIds = toggleSelection(rolePermissionIds, permission.id); }} />
                      <span>{permission.resource}:{permission.action}</span>
                    </label>
                  {/each}
                </div>
              </div>
              <button disabled={saving === 'role'} class="rounded-xl bg-indigo-600 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-500 disabled:opacity-50">Create role</button>
            </form>
          {/if}
        </section>
      {/if}

      {#if canReadGroups()}
        <section class="rounded-[2rem] border border-gray-200 bg-white p-8 shadow-sm dark:border-gray-800 dark:bg-gray-900">
          <div class="text-xs uppercase tracking-[0.24em] text-gray-400">Groups</div>
          <h2 class="mt-2 text-2xl font-semibold">Inherited access through groups</h2>

          <div class="mt-6 space-y-4">
            {#each groups as group}
              <article class="rounded-2xl border border-gray-200 bg-gray-50 p-4 dark:border-gray-800 dark:bg-gray-950">
                <div class="flex items-start justify-between gap-4">
                  <div>
                    <h3 class="font-semibold">{group.name}</h3>
                    <p class="text-sm text-gray-500">{group.description ?? 'No description'}</p>
                  </div>
                  <div class="text-xs uppercase tracking-[0.18em] text-gray-400">{group.member_count} members</div>
                </div>
                <div class="mt-3 flex flex-wrap gap-2">
                  {#each group.roles as roleName}
                    <span class="rounded-full bg-amber-100 px-3 py-1 text-xs font-medium text-amber-700 dark:bg-amber-950 dark:text-amber-300">{roleName}</span>
                  {/each}
                </div>
              </article>
            {/each}
          </div>

          {#if canManageGroups()}
            <form class="mt-8 space-y-4 rounded-3xl border border-dashed border-gray-300 p-5 dark:border-gray-700" onsubmit={(event) => { event.preventDefault(); handleCreateGroup(); }}>
              <div class="text-xs uppercase tracking-[0.2em] text-gray-400">Create group</div>
              <input bind:value={groupForm.name} required placeholder="Group name" class="w-full rounded-xl border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
              <textarea bind:value={groupForm.description} rows="2" placeholder="Group description" class="w-full rounded-xl border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900"></textarea>
              <div>
                <div class="mb-2 text-sm font-medium">Attach roles</div>
                <div class="grid max-h-48 gap-2 overflow-auto rounded-2xl border border-gray-200 p-3 text-sm dark:border-gray-800">
                  {#each roles as role}
                    <label class="flex items-center gap-2">
                      <input type="checkbox" checked={groupRoleIds.includes(role.id)} onchange={() => { groupRoleIds = toggleSelection(groupRoleIds, role.id); }} />
                      <span>{role.name}</span>
                    </label>
                  {/each}
                </div>
              </div>
              <button disabled={saving === 'group'} class="rounded-xl bg-indigo-600 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-500 disabled:opacity-50">Create group</button>
            </form>
          {/if}
        </section>
      {/if}
    </div>

    <div class="grid gap-8 xl:grid-cols-2">
      {#if canReadPermissions()}
        <section class="rounded-[2rem] border border-gray-200 bg-white p-8 shadow-sm dark:border-gray-800 dark:bg-gray-900">
          <div class="text-xs uppercase tracking-[0.24em] text-gray-400">Permission catalog</div>
          <h2 class="mt-2 text-2xl font-semibold">Permission registry</h2>

          <div class="mt-6 space-y-3">
            {#each permissions as permission}
              <div class="rounded-2xl border border-gray-200 bg-gray-50 px-4 py-3 text-sm dark:border-gray-800 dark:bg-gray-950">
                <div class="font-medium">{permission.resource}:{permission.action}</div>
                <div class="text-gray-500">{permission.description ?? 'No description'}</div>
              </div>
            {/each}
          </div>

          {#if canManagePermissions()}
            <form class="mt-8 space-y-4 rounded-3xl border border-dashed border-gray-300 p-5 dark:border-gray-700" onsubmit={(event) => { event.preventDefault(); handleCreatePermission(); }}>
              <div class="text-xs uppercase tracking-[0.2em] text-gray-400">Create permission</div>
              <input bind:value={permissionForm.resource} required placeholder="Resource, e.g. notebooks" class="w-full rounded-xl border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
              <input bind:value={permissionForm.action} required placeholder="Action, e.g. read" class="w-full rounded-xl border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
              <textarea bind:value={permissionForm.description} rows="2" placeholder="Description" class="w-full rounded-xl border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900"></textarea>
              <button disabled={saving === 'permission'} class="rounded-xl bg-indigo-600 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-500 disabled:opacity-50">Create permission</button>
            </form>
          {/if}
        </section>
      {/if}

      <section class="rounded-[2rem] border border-gray-200 bg-white p-8 shadow-sm dark:border-gray-800 dark:bg-gray-900">
        <div class="text-xs uppercase tracking-[0.24em] text-gray-400">Self-service access</div>
        <h2 class="mt-2 text-2xl font-semibold">MFA and API keys</h2>

        <div class="mt-6 rounded-3xl border border-gray-200 bg-gray-50 p-5 dark:border-gray-800 dark:bg-gray-950">
          <div class="flex items-start justify-between gap-4">
            <div>
              <h3 class="font-semibold">Multi-factor authentication</h3>
              <p class="mt-1 text-sm text-gray-500">
                {#if mfaStatus?.enabled}
                  MFA is active for your account.
                {:else if mfaStatus?.configured}
                  Enrollment is configured but still waiting for verification.
                {:else}
                  Protect your account with a TOTP authenticator.
                {/if}
              </p>
            </div>
            <div class={`rounded-full px-3 py-1 text-xs font-medium ${mfaStatus?.enabled ? 'bg-emerald-100 text-emerald-700 dark:bg-emerald-950 dark:text-emerald-300' : 'bg-gray-200 text-gray-600 dark:bg-gray-800 dark:text-gray-300'}`}>
              {mfaStatus?.enabled ? 'Enabled' : 'Disabled'}
            </div>
          </div>

          <div class="mt-4 flex flex-wrap gap-2">
            <button onclick={handleEnrollMfa} disabled={saving === 'enroll-mfa'} class="rounded-xl bg-indigo-600 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-500 disabled:opacity-50">Generate secret</button>
          </div>

          {#if mfaEnrollment}
            <div class="mt-5 rounded-2xl border border-dashed border-indigo-300 p-4 text-sm dark:border-indigo-800">
              <div class="font-medium">Authenticator secret</div>
              <div class="mt-2 break-all font-mono text-indigo-600 dark:text-indigo-300">{mfaEnrollment.secret}</div>
              <div class="mt-4 font-medium">Recovery codes</div>
              <div class="mt-2 grid gap-2 md:grid-cols-2">
                {#each mfaEnrollment.recovery_codes as code}
                  <div class="rounded-xl bg-white px-3 py-2 font-mono text-xs dark:bg-gray-900">{code}</div>
                {/each}
              </div>
              <div class="mt-4 flex gap-2">
                <input bind:value={mfaVerifyCode} placeholder="Enter TOTP code" class="min-w-0 flex-1 rounded-xl border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
                <button onclick={handleVerifyMfa} disabled={saving === 'verify-mfa'} class="rounded-xl bg-emerald-600 px-4 py-2 text-sm font-medium text-white hover:bg-emerald-500 disabled:opacity-50">Verify</button>
              </div>
            </div>
          {/if}

          {#if mfaStatus?.enabled}
            <div class="mt-5 flex gap-2">
              <input bind:value={mfaDisableCode} placeholder="Code or recovery code to disable" class="min-w-0 flex-1 rounded-xl border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
              <button onclick={handleDisableMfa} disabled={saving === 'disable-mfa'} class="rounded-xl border border-red-300 px-4 py-2 text-sm font-medium text-red-700 hover:border-red-400 dark:border-red-800 dark:text-red-300 disabled:opacity-50">Disable MFA</button>
            </div>
          {/if}
        </div>

        <div class="mt-6 rounded-3xl border border-gray-200 bg-gray-50 p-5 dark:border-gray-800 dark:bg-gray-950">
          <h3 class="font-semibold">API keys</h3>
          <p class="mt-1 text-sm text-gray-500">Issue scoped programmatic credentials for automation and service integrations.</p>

          {#if newApiKey}
            <div class="mt-4 rounded-2xl border border-dashed border-amber-300 bg-amber-50 p-4 text-sm dark:border-amber-800 dark:bg-amber-950/40">
              <div class="font-medium">New key token</div>
              <div class="mt-2 break-all font-mono text-xs text-amber-700 dark:text-amber-200">{newApiKey.token}</div>
            </div>
          {/if}

          <div class="mt-4 space-y-3">
            {#each apiKeys as apiKey}
              <div class="rounded-2xl border border-gray-200 bg-white p-4 dark:border-gray-800 dark:bg-gray-900">
                <div class="flex items-start justify-between gap-4">
                  <div>
                    <div class="font-medium">{apiKey.name}</div>
                    <div class="text-xs text-gray-500">{apiKey.prefix} • created {new Date(apiKey.created_at).toLocaleString()}</div>
                  </div>
                  <button onclick={() => handleRevokeApiKey(apiKey.id)} disabled={saving === `api-key-${apiKey.id}`} class="rounded-xl border border-gray-300 px-3 py-2 text-sm font-medium hover:border-indigo-400 dark:border-gray-700 disabled:opacity-50">Revoke</button>
                </div>
                <div class="mt-3 flex flex-wrap gap-2">
                  {#each apiKey.scopes as scope}
                    <span class="rounded-full bg-sky-100 px-3 py-1 text-xs font-medium text-sky-700 dark:bg-sky-950 dark:text-sky-300">{scope}</span>
                  {/each}
                </div>
              </div>
            {/each}
          </div>

          <form class="mt-6 grid gap-3 md:grid-cols-2" onsubmit={(event) => { event.preventDefault(); handleCreateApiKey(); }}>
            <input bind:value={apiKeyForm.name} required placeholder="Key name" class="rounded-xl border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
            <input bind:value={apiKeyForm.scopes} placeholder="Scopes, comma separated" class="rounded-xl border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
            <input bind:value={apiKeyForm.expires_at} type="datetime-local" class="rounded-xl border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
            <button disabled={saving === 'api-key'} class="rounded-xl bg-indigo-600 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-500 disabled:opacity-50">Create API key</button>
          </form>
        </div>
      </section>
    </div>

    {#if canReadPolicies()}
      <section class="rounded-[2rem] border border-gray-200 bg-white p-8 shadow-sm dark:border-gray-800 dark:bg-gray-900">
        <div class="grid gap-8 xl:grid-cols-[1.15fr_0.85fr]">
          <div>
            <div class="text-xs uppercase tracking-[0.24em] text-gray-400">ABAC</div>
            <h2 class="mt-2 text-2xl font-semibold">Policies and evaluation</h2>

            <div class="mt-6 space-y-4">
              {#each policies as policy}
                <article class="rounded-2xl border border-gray-200 bg-gray-50 p-4 dark:border-gray-800 dark:bg-gray-950">
                  <div class="flex items-start justify-between gap-4">
                    <div>
                      <h3 class="font-semibold">{policy.name}</h3>
                      <p class="text-sm text-gray-500">{policy.resource}:{policy.action} • {policy.effect}</p>
                    </div>
                    <div class={`rounded-full px-3 py-1 text-xs font-medium ${policy.enabled ? 'bg-emerald-100 text-emerald-700 dark:bg-emerald-950 dark:text-emerald-300' : 'bg-gray-200 text-gray-600 dark:bg-gray-800 dark:text-gray-300'}`}>
                      {policy.enabled ? 'Enabled' : 'Disabled'}
                    </div>
                  </div>
                  <pre class="mt-3 overflow-auto rounded-xl bg-white p-3 text-xs dark:bg-gray-900">{JSON.stringify(policy.conditions, null, 2)}</pre>
                  {#if policy.row_filter}
                    <div class="mt-3 rounded-xl bg-white px-3 py-2 text-xs dark:bg-gray-900">{policy.row_filter}</div>
                  {/if}
                  {#if canManagePolicies()}
                    <button onclick={() => handleDeletePolicy(policy.id)} disabled={saving === `delete-policy-${policy.id}`} class="mt-4 rounded-xl border border-red-300 px-3 py-2 text-sm font-medium text-red-700 hover:border-red-400 dark:border-red-800 dark:text-red-300 disabled:opacity-50">Delete</button>
                  {/if}
                </article>
              {/each}
            </div>
          </div>

          <div class="space-y-6">
            {#if canManagePolicies()}
              <form class="space-y-4 rounded-3xl border border-dashed border-gray-300 p-5 dark:border-gray-700" onsubmit={(event) => { event.preventDefault(); handleCreatePolicy(); }}>
                <div class="text-xs uppercase tracking-[0.2em] text-gray-400">Create policy</div>
                <input bind:value={policyForm.name} required placeholder="Policy name" class="w-full rounded-xl border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
                <textarea bind:value={policyForm.description} rows="2" placeholder="Description" class="w-full rounded-xl border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900"></textarea>
                <div class="grid gap-3 md:grid-cols-2">
                  <input bind:value={policyForm.resource} required placeholder="Resource" class="rounded-xl border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
                  <input bind:value={policyForm.action} required placeholder="Action" class="rounded-xl border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
                </div>
                <select bind:value={policyForm.effect} class="w-full rounded-xl border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900">
                  <option value="allow">Allow</option>
                  <option value="deny">Deny</option>
                </select>
                <textarea bind:value={policyForm.conditions} rows="7" class="w-full rounded-xl border border-gray-300 bg-white px-3 py-2 font-mono text-sm dark:border-gray-700 dark:bg-gray-900"></textarea>
                <input bind:value={policyForm.row_filter} placeholder="Optional row filter template" class="w-full rounded-xl border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
                <label class="flex items-center gap-2 text-sm">
                  <input type="checkbox" bind:checked={policyForm.enabled} />
                  Enabled
                </label>
                <button disabled={saving === 'policy'} class="rounded-xl bg-indigo-600 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-500 disabled:opacity-50">Create policy</button>
              </form>
            {/if}

            <form class="space-y-4 rounded-3xl border border-gray-200 p-5 dark:border-gray-800" onsubmit={(event) => { event.preventDefault(); handleEvaluatePolicy(); }}>
              <div class="text-xs uppercase tracking-[0.2em] text-gray-400">Evaluate access</div>
              <input bind:value={policyEvaluationForm.resource} required placeholder="Resource" class="w-full rounded-xl border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
              <input bind:value={policyEvaluationForm.action} required placeholder="Action" class="w-full rounded-xl border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
              <textarea bind:value={policyEvaluationForm.resource_attributes} rows="6" class="w-full rounded-xl border border-gray-300 bg-white px-3 py-2 font-mono text-sm dark:border-gray-700 dark:bg-gray-900"></textarea>
              <button disabled={saving === 'evaluate-policy'} class="rounded-xl border border-gray-300 px-4 py-2 text-sm font-medium hover:border-indigo-400 dark:border-gray-700 disabled:opacity-50">Evaluate</button>

              {#if policyEvaluation}
                <div class="rounded-2xl bg-gray-50 p-4 text-sm dark:bg-gray-950">
                  <div class="font-medium">{policyEvaluation.allowed ? 'Allowed' : 'Denied'}</div>
                  <div class="mt-2 text-gray-500">Matched: {policyEvaluation.matched_policy_ids.length} · Deny hits: {policyEvaluation.deny_policy_ids.length}</div>
                  <pre class="mt-3 overflow-auto rounded-xl bg-white p-3 text-xs dark:bg-gray-900">{JSON.stringify(policyEvaluation, null, 2)}</pre>
                </div>
              {/if}
            </form>
          </div>
        </div>
      </section>
    {/if}

    {#if canReadSso()}
      <section class="rounded-[2rem] border border-gray-200 bg-white p-8 shadow-sm dark:border-gray-800 dark:bg-gray-900">
        <div class="text-xs uppercase tracking-[0.24em] text-gray-400">SSO</div>
        <h2 class="mt-2 text-2xl font-semibold">Provider connections</h2>

        <div class="mt-6 grid gap-4 xl:grid-cols-2">
          {#each ssoProviders as provider}
            <article class="rounded-3xl border border-gray-200 bg-gray-50 p-5 dark:border-gray-800 dark:bg-gray-950">
              <div class="flex items-start justify-between gap-4">
                <div>
                  <h3 class="font-semibold">{provider.name}</h3>
                  <p class="text-sm text-gray-500">{provider.provider_type} • /{provider.slug}</p>
                </div>
                <div class={`rounded-full px-3 py-1 text-xs font-medium ${provider.enabled ? 'bg-emerald-100 text-emerald-700 dark:bg-emerald-950 dark:text-emerald-300' : 'bg-gray-200 text-gray-600 dark:bg-gray-800 dark:text-gray-300'}`}>
                  {provider.enabled ? 'Enabled' : 'Disabled'}
                </div>
              </div>
              <div class="mt-4 flex flex-wrap gap-2">
                {#each provider.scopes as scope}
                  <span class="rounded-full bg-slate-200 px-3 py-1 text-xs font-medium text-slate-700 dark:bg-slate-800 dark:text-slate-300">{scope}</span>
                {/each}
              </div>
              <pre class="mt-4 overflow-auto rounded-xl bg-white p-3 text-xs dark:bg-gray-900">{JSON.stringify(provider.attribute_mapping, null, 2)}</pre>
              {#if canManageSso()}
                <button onclick={() => handleDeleteSsoProvider(provider.id)} disabled={saving === `delete-sso-${provider.id}`} class="mt-4 rounded-xl border border-red-300 px-3 py-2 text-sm font-medium text-red-700 hover:border-red-400 dark:border-red-800 dark:text-red-300 disabled:opacity-50">Delete provider</button>
              {/if}
            </article>
          {/each}
        </div>

        {#if canManageSso()}
          <form class="mt-8 grid gap-4 rounded-3xl border border-dashed border-gray-300 p-5 dark:border-gray-700 xl:grid-cols-2" onsubmit={(event) => { event.preventDefault(); handleCreateSsoProvider(); }}>
            <div class="xl:col-span-2 text-xs uppercase tracking-[0.2em] text-gray-400">Create provider</div>
            <input bind:value={ssoForm.name} required placeholder="Display name" class="rounded-xl border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
            <input bind:value={ssoForm.slug} required placeholder="Slug" class="rounded-xl border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
            <select bind:value={ssoForm.provider_type} class="rounded-xl border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900">
              <option value="oidc">OIDC</option>
              <option value="saml">SAML</option>
            </select>
            <label class="flex items-center gap-2 rounded-xl border border-gray-300 px-3 py-2 text-sm dark:border-gray-700">
              <input type="checkbox" bind:checked={ssoForm.enabled} />
              Enabled
            </label>
            <input bind:value={ssoForm.client_id} placeholder="Client ID" class="rounded-xl border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
            <input bind:value={ssoForm.client_secret} placeholder="Client secret" class="rounded-xl border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
            <input bind:value={ssoForm.issuer_url} placeholder="Issuer URL" class="rounded-xl border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
            <input bind:value={ssoForm.authorization_url} placeholder="Authorization URL" class="rounded-xl border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
            <input bind:value={ssoForm.token_url} placeholder="Token URL" class="rounded-xl border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
            <input bind:value={ssoForm.userinfo_url} placeholder="Userinfo URL" class="rounded-xl border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
            <input bind:value={ssoForm.scopes} placeholder="Scopes, comma separated" class="rounded-xl border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900 xl:col-span-2" />
            <input bind:value={ssoForm.saml_metadata_url} placeholder="SAML metadata URL" class="rounded-xl border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
            <input bind:value={ssoForm.saml_entity_id} placeholder="SAML entity ID" class="rounded-xl border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
            <input bind:value={ssoForm.saml_sso_url} placeholder="SAML SSO URL" class="rounded-xl border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
            <input bind:value={ssoForm.saml_certificate} placeholder="SAML certificate" class="rounded-xl border border-gray-300 bg-white px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
            <textarea bind:value={ssoForm.attribute_mapping} rows="7" class="xl:col-span-2 w-full rounded-xl border border-gray-300 bg-white px-3 py-2 font-mono text-sm dark:border-gray-700 dark:bg-gray-900"></textarea>
            <button disabled={saving === 'sso-provider'} class="rounded-xl bg-indigo-600 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-500 disabled:opacity-50 xl:col-span-2 xl:w-max">Create provider</button>
          </form>
        {/if}
      </section>
    {/if}
  </div>
{/if}