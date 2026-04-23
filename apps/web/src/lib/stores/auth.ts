import { derived, get, writable } from 'svelte/store';
import api from '$api/client';
import {
  completeMfaLogin,
  completeSsoLogin,
  getMe,
  login as apiLogin,
  refreshToken,
  startSsoLogin as apiStartSsoLogin,
  type LoginResponse,
  type MfaRequiredResponse,
  type TokenResponse,
  type UserProfile,
} from '$api/auth';

const ACCESS_TOKEN_KEY = 'of_access_token';
const REFRESH_TOKEN_KEY = 'of_refresh_token';
const PENDING_MFA_KEY = 'of_pending_mfa';

type AuthFlowResult = { status: 'authenticated' } | MfaRequiredResponse;

function createAuthStore() {
  const token = writable<string | null>(null);
  const user = writable<UserProfile | null>(null);
  const loading = writable(false);
  const pendingChallenge = writable<MfaRequiredResponse | null>(null);

  const isAuthenticated = derived(token, ($token) => !!$token);
  let restorePromise: Promise<void> | null = null;

  async function login(email: string, password: string) {
    loading.set(true);
    try {
      const resp = await apiLogin({ email, password });
      return handleLoginResponse(resp);
    } finally {
      loading.set(false);
    }
  }

  async function completeMfa(code: string): Promise<{ status: 'authenticated' }> {
    const challenge = get(pendingChallenge);
    if (!challenge) {
      throw new Error('MFA challenge missing or expired');
    }

    loading.set(true);
    try {
      const resp = await completeMfaLogin({
        challenge_token: challenge.challenge_token,
        code,
      });
      await finalizeSession(resp);
      return { status: 'authenticated' };
    } finally {
      loading.set(false);
    }
  }

  async function handleSsoCallback(code: string, state: string): Promise<AuthFlowResult> {
    loading.set(true);
    try {
      const resp = await completeSsoLogin({ code, state });
      return handleLoginResponse(resp);
    } finally {
      loading.set(false);
    }
  }

  async function startSsoLogin(slug: string) {
    loading.set(true);
    try {
      const resp = await apiStartSsoLogin(slug);
      if (typeof window !== 'undefined') {
        window.location.assign(resp.authorization_url);
      }
    } finally {
      loading.set(false);
    }
  }

  function setSession(resp: TokenResponse) {
    token.set(resp.access_token);
    api.setToken(resp.access_token);
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(ACCESS_TOKEN_KEY, resp.access_token);
      localStorage.setItem(REFRESH_TOKEN_KEY, resp.refresh_token);
    }
  }

  function persistChallenge(challenge: MfaRequiredResponse) {
    pendingChallenge.set(challenge);
    if (typeof sessionStorage !== 'undefined') {
      sessionStorage.setItem(PENDING_MFA_KEY, JSON.stringify(challenge));
    }
  }

  function clearPendingChallenge() {
    pendingChallenge.set(null);
    if (typeof sessionStorage !== 'undefined') {
      sessionStorage.removeItem(PENDING_MFA_KEY);
    }
  }

  function hydratePendingChallenge() {
    if (typeof sessionStorage === 'undefined') return;
    const raw = sessionStorage.getItem(PENDING_MFA_KEY);
    if (!raw) return;

    try {
      const challenge = JSON.parse(raw) as MfaRequiredResponse;
      pendingChallenge.set(challenge);
    } catch {
      clearPendingChallenge();
    }
  }

  async function finalizeSession(resp: TokenResponse) {
    setSession(resp);
    const profile = await getMe();
    user.set(profile);
    clearPendingChallenge();
  }

  async function handleLoginResponse(resp: LoginResponse): Promise<AuthFlowResult> {
    if (resp.status === 'mfa_required') {
      persistChallenge(resp);
      return resp;
    }

    await finalizeSession(resp);
    return { status: 'authenticated' };
  }

  function logout() {
    token.set(null);
    user.set(null);
    api.setToken(null);
    clearPendingChallenge();
    if (typeof localStorage !== 'undefined') {
      localStorage.removeItem(ACCESS_TOKEN_KEY);
      localStorage.removeItem(REFRESH_TOKEN_KEY);
    }
  }

  async function restore() {
    if (restorePromise) {
      return restorePromise;
    }

    restorePromise = (async () => {
      hydratePendingChallenge();

      if (typeof localStorage === 'undefined') {
        return;
      }

      const savedAccessToken = localStorage.getItem(ACCESS_TOKEN_KEY);
      const savedRefreshToken = localStorage.getItem(REFRESH_TOKEN_KEY);

      if (savedAccessToken) {
        token.set(savedAccessToken);
        api.setToken(savedAccessToken);
        try {
          user.set(await getMe());
          return;
        } catch {
          token.set(null);
          api.setToken(null);
        }
      }

      if (savedRefreshToken) {
        try {
          const refreshed = await refreshToken(savedRefreshToken);
          await finalizeSession(refreshed);
          return;
        } catch {
          logout();
        }
      }
    })().finally(() => {
      restorePromise = null;
    });

    return restorePromise;
  }

  return {
    token,
    user,
    loading,
    isAuthenticated,
    pendingChallenge,
    login,
    completeMfa,
    handleSsoCallback,
    startSsoLogin,
    clearPendingChallenge,
    logout,
    restore,
  };
}

export const auth = createAuthStore();