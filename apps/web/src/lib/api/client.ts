const API_BASE = '/api/v1';

interface RequestOptions {
  method?: string;
  body?: unknown;
  headers?: Record<string, string>;
}

class ApiClient {
  private token: string | null = null;

  setToken(token: string | null) {
    this.token = token;
  }

  authorizationHeaders() {
    return this.token
      ? ({ Authorization: `Bearer ${this.token}` } satisfies Record<string, string>)
      : ({} satisfies Record<string, string>);
  }

  async fetch<T>(path: string, options: RequestOptions = {}): Promise<T> {
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      ...options.headers,
    };

    if (this.token) {
      headers['Authorization'] = `Bearer ${this.token}`;
    }

    const response = await fetch(`${API_BASE}${path}`, {
      method: options.method ?? 'GET',
      headers,
      body: options.body ? JSON.stringify(options.body) : undefined,
    });

    if (!response.ok) {
      const error = await response.json().catch(() => ({ error: response.statusText }));
      throw new ApiError(response.status, error.error ?? 'Unknown error');
    }

    if (response.status === 204) return undefined as T;
    return response.json();
  }

  get<T>(path: string) {
    return this.fetch<T>(path);
  }

  post<T>(path: string, body: unknown) {
    return this.fetch<T>(path, { method: 'POST', body });
  }

  put<T>(path: string, body: unknown) {
    return this.fetch<T>(path, { method: 'PUT', body });
  }

  patch<T>(path: string, body: unknown) {
    return this.fetch<T>(path, { method: 'PATCH', body });
  }

  delete<T>(path: string) {
    return this.fetch<T>(path, { method: 'DELETE' });
  }
}

export class ApiError extends Error {
  constructor(
    public status: number,
    message: string,
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

export const api = new ApiClient();
export default api;
