import type {
  AgentRun,
  ApiResponse,
  AuthSession,
  CategoryItem,
  CategoryFormPayload,
  DashboardSummary,
  DeletedResource,
  DocumentDetail,
  DocumentFormPayload,
  DocumentListItem,
  FaqItem,
  FaqFormPayload,
  FavoriteDocumentItem,
  FavoriteState,
  LoginPayload,
  QaAnswer,
  QuestionHistoryItem,
  ReadRecordItem,
  RoleItem,
  TagItem,
  TagFormPayload,
  UserCreatePayload,
  UserItem,
  UserUpdatePayload
} from "./types";

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? "http://localhost:8080/api";
const AUTH_STORAGE_KEY = "zhishu_access_token";

function getAuthToken(): string | null {
  return window.localStorage.getItem(AUTH_STORAGE_KEY);
}

export function setAuthToken(token: string) {
  window.localStorage.setItem(AUTH_STORAGE_KEY, token);
}

export function clearAuthToken() {
  window.localStorage.removeItem(AUTH_STORAGE_KEY);
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const token = getAuthToken();
  const response = await fetch(`${API_BASE_URL}${path}`, {
    headers: {
      "Content-Type": "application/json",
      ...(token ? { Authorization: `Bearer ${token}` } : {})
    },
    ...init
  });

  if (!response.ok) {
    throw new Error(`Request failed: ${response.status}`);
  }

  const result = (await response.json()) as ApiResponse<T>;
  return result.data;
}

export const api = {
  login: (payload: LoginPayload) =>
    request<AuthSession>("/auth/login", {
      method: "POST",
      body: JSON.stringify(payload)
    }),
  getCurrentUser: () => request<UserItem>("/auth/me"),
  getRoles: () => request<RoleItem[]>("/roles"),
  getDashboard: () => request<DashboardSummary>("/dashboard"),
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
  getFavorites: () => request<FavoriteDocumentItem[]>("/favorites"),
  getRecentReads: () => request<ReadRecordItem[]>("/read-records/recent"),
  getDocuments: () => request<DocumentListItem[]>("/documents"),
  getDocument: (id: number) => request<DocumentDetail>(`/documents/${id}`),
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
