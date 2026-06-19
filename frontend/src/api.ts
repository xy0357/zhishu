import type {
  AgentRun,
  ApiResponse,
  AuthSession,
  CategoryItem,
  CategoryFormPayload,
  DashboardSummary,
  HealthStatus,
  DeletedResource,
  DocumentDetail,
  DocumentFileMeta,
  DocumentFormPayload,
  DocumentListItem,
  DocumentSegment,
  FaqItem,
  FaqFormPayload,
  FavoriteDocumentItem,
  FavoriteState,
  LoginPayload,
  QaAnswer,
  QuestionHistoryItem,
  ReadRecordItem,
  RefreshSession,
  RegisterDocumentFilePayload,
  ResetPasswordPayload,
  RoleItem,
  TagItem,
  TagFormPayload,
  UploadDocumentFilePayload,
  UserCreatePayload,
  UserItem,
  UserUpdatePayload
} from "./types";

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? "http://localhost:8080/api";
const AUTH_STORAGE_KEY = "zhishu_access_token";
let refreshPromise: Promise<string | null> | null = null;

function getAuthToken(): string | null {
  return window.localStorage.getItem(AUTH_STORAGE_KEY);
}

export function setAuthToken(token: string) {
  window.localStorage.setItem(AUTH_STORAGE_KEY, token);
}

export function clearAuthToken() {
  window.localStorage.removeItem(AUTH_STORAGE_KEY);
}

function arrayBufferToBase64(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer);
  let binary = "";
  const chunkSize = 0x8000;
  for (let index = 0; index < bytes.length; index += chunkSize) {
    const chunk = bytes.subarray(index, index + chunkSize);
    binary += String.fromCharCode(...chunk);
  }
  return window.btoa(binary);
}

async function parseError(response: Response): Promise<Error> {
  const contentType = response.headers.get("content-type") ?? "";
  if (contentType.includes("application/json")) {
    try {
      const payload = (await response.json()) as Partial<ApiResponse<unknown>>;
      if (payload.message) {
        return new Error(payload.message);
      }
    } catch {
      // Ignore parse failures and fall back to status code.
    }
  }

  return new Error(`Request failed: ${response.status}`);
}

async function refreshAccessToken(): Promise<string | null> {
  if (!refreshPromise) {
    refreshPromise = (async () => {
      const token = getAuthToken();
      if (!token) {
        return null;
      }

      const response = await fetch(`${API_BASE_URL}/auth/refresh`, {
        method: "POST",
        headers: {
          Authorization: `Bearer ${token}`
        }
      });

      if (!response.ok) {
        clearAuthToken();
        return null;
      }

      const result = (await response.json()) as ApiResponse<RefreshSession>;
      setAuthToken(result.data.access_token);
      return result.data.access_token;
    })().finally(() => {
      refreshPromise = null;
    });
  }

  return refreshPromise;
}

async function doFetch(path: string, init?: RequestInit, tokenOverride?: string | null): Promise<Response> {
  const token = tokenOverride ?? getAuthToken();
  const initHeaders = new Headers(init?.headers);
  if (!initHeaders.has("Content-Type")) {
    initHeaders.set("Content-Type", "application/json");
  }
  if (token && !initHeaders.has("Authorization")) {
    initHeaders.set("Authorization", `Bearer ${token}`);
  }

  return fetch(`${API_BASE_URL}${path}`, {
    ...init
    ,
    headers: initHeaders
  });
}

async function request<T>(path: string, init?: RequestInit, options?: { skipAuthRefresh?: boolean }): Promise<T> {
  let response = await doFetch(path, init);

  if (response.status === 401 && !options?.skipAuthRefresh && path !== "/auth/login" && path !== "/auth/refresh") {
    const refreshedToken = await refreshAccessToken();
    if (refreshedToken) {
      response = await doFetch(path, init, refreshedToken);
    }
  }

  if (!response.ok) {
    throw await parseError(response);
  }

  const result = (await response.json()) as ApiResponse<T>;
  return result.data;
}

async function download(path: string): Promise<Blob> {
  let response = await doFetch(path, { headers: {} });
  if (response.status === 401) {
    const refreshedToken = await refreshAccessToken();
    if (refreshedToken) {
      response = await doFetch(path, { headers: {} }, refreshedToken);
    }
  }

  if (!response.ok) {
    throw await parseError(response);
  }

  return response.blob();
}

export const api = {
  login: (payload: LoginPayload) =>
    request<AuthSession>("/auth/login", {
      method: "POST",
      body: JSON.stringify(payload)
    }, { skipAuthRefresh: true }),
  refreshSession: () =>
    request<RefreshSession>("/auth/refresh", {
      method: "POST"
    }, { skipAuthRefresh: true }),
  getCurrentUser: () => request<UserItem>("/auth/me"),
  getRoles: () => request<RoleItem[]>("/roles"),
  getDashboard: () => request<DashboardSummary>("/dashboard"),
  getHealth: () => request<HealthStatus>("/health"),
  getCategories: () => request<CategoryItem[]>("/categories"),
  createCategory: (payload: CategoryFormPayload) =>
    request<CategoryItem>("/categories", {
      method: "POST",
      body: JSON.stringify(payload)
    }),
  updateCategory: (currentName: string, payload: CategoryFormPayload) =>
    request<CategoryItem>(`/categories/${encodeURIComponent(currentName)}`, {
      method: "PUT",
      body: JSON.stringify(payload)
    }),
  deleteCategory: (name: string) =>
    request<DeletedResource>(`/categories/${encodeURIComponent(name)}`, {
      method: "DELETE"
    }),
  getTags: () => request<TagItem[]>("/tags"),
  createTag: (payload: TagFormPayload) =>
    request<TagItem>("/tags", {
      method: "POST",
      body: JSON.stringify(payload)
    }),
  updateTag: (currentName: string, payload: TagFormPayload) =>
    request<TagItem>(`/tags/${encodeURIComponent(currentName)}`, {
      method: "PUT",
      body: JSON.stringify(payload)
    }),
  deleteTag: (name: string) =>
    request<DeletedResource>(`/tags/${encodeURIComponent(name)}`, {
      method: "DELETE"
    }),
  getUsers: () => request<UserItem[]>("/users"),
  createUser: (payload: UserCreatePayload) =>
    request<UserItem>("/users", {
      method: "POST",
      body: JSON.stringify(payload)
    }),
  updateUser: (userId: number, payload: UserUpdatePayload) =>
    request<UserItem>(`/users/${userId}`, {
      method: "PUT",
      body: JSON.stringify(payload)
    }),
  resetUserPassword: (userId: number, payload: ResetPasswordPayload) =>
    request<UserItem>(`/users/${userId}/reset-password`, {
      method: "POST",
      body: JSON.stringify(payload)
    }),
  deleteUser: (userId: number) =>
    request<DeletedResource>(`/users/${userId}`, {
      method: "DELETE"
    }),
  getFavorites: () => request<FavoriteDocumentItem[]>("/favorites"),
  getRecentReads: () => request<ReadRecordItem[]>("/read-records/recent"),
  getDocumentFiles: () => request<DocumentFileMeta[]>("/document-files"),
  getDocumentFile: (fileId: number) => request<DocumentFileMeta>(`/document-files/${fileId}`),
  downloadDocumentFile: (fileId: number) => download(`/document-files/${fileId}/download`),
  registerDocumentFile: (payload: RegisterDocumentFilePayload) =>
    request<DocumentFileMeta>("/document-files", {
      method: "POST",
      body: JSON.stringify(payload)
    }),
  uploadDocumentFile: async (file: File) => {
    const payload: UploadDocumentFilePayload = {
      original_name: file.name,
      mime_type: file.type || "application/octet-stream",
      content_base64: arrayBufferToBase64(await file.arrayBuffer())
    };
    return request<DocumentFileMeta>("/document-files/upload", {
      method: "POST",
      body: JSON.stringify(payload)
    });
  },
  getDocuments: () => request<DocumentListItem[]>("/documents"),
  getDocument: (id: number) => request<DocumentDetail>(`/documents/${id}`),
  getDocumentSegments: (id: number) => request<DocumentSegment[]>(`/documents/${id}/segments`),
  getDocumentFaqs: (id: number) => request<FaqItem[]>(`/documents/${id}/faqs`),
  createDocumentFaq: (documentId: number, payload: FaqFormPayload) =>
    request<FaqItem>(`/documents/${documentId}/faqs`, {
      method: "POST",
      body: JSON.stringify(payload)
    }),
  updateFaq: (faqId: number, payload: FaqFormPayload) =>
    request<FaqItem>(`/faqs/${faqId}`, {
      method: "PUT",
      body: JSON.stringify(payload)
    }),
  deleteFaq: (faqId: number) =>
    request<DeletedResource>(`/faqs/${faqId}`, {
      method: "DELETE"
    }),
  recordDocumentRead: (id: number) =>
    request<ReadRecordItem>(`/documents/${id}/read`, { method: "POST" }),
  toggleFavoriteDocument: (id: number) =>
    request<FavoriteState>(`/documents/${id}/favorite`, { method: "POST" }),
  updateDocument: (id: number, payload: DocumentFormPayload) =>
    request<DocumentDetail>(`/documents/${id}`, {
      method: "PUT",
      body: JSON.stringify(payload)
    }),
  publishDocument: (id: number) =>
    request<DocumentDetail>(`/documents/${id}/publish`, { method: "POST" }),
  archiveDocument: (id: number) =>
    request<DocumentDetail>(`/documents/${id}/archive`, { method: "POST" }),
  reindexDocument: (id: number) =>
    request<DocumentDetail>(`/documents/${id}/reindex`, { method: "POST" }),
  createDocument: (payload: DocumentFormPayload) =>
    request<DocumentDetail>("/documents", {
      method: "POST",
      body: JSON.stringify(payload)
    }),
  askQuestion: (questionText: string) =>
    request<QaAnswer>("/qa/ask", {
      method: "POST",
      body: JSON.stringify({ question_text: questionText })
    }),
  getQuestionHistory: () => request<QuestionHistoryItem[]>("/questions/history"),
  getAgentRuns: () => request<AgentRun[]>("/agent-runs")
};
