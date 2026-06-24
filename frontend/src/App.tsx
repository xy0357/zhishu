import { useEffect, useState } from "react";
import { api, clearAuthToken, setAuthToken } from "./api";
import type {
  AgentRun,
  AuthSession,
  CategoryItem,
  CategoryFormPayload,
  DashboardSummary,
  HealthStatus,
  DocumentDetail,
  DocumentFileMeta,
  DocumentFormPayload,
  DocumentListItem,
  DocumentSegment,
  FaqItem,
  FaqFormPayload,
  FavoriteDocumentItem,
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

type ViewKey = "dashboard" | "documents" | "qa" | "people" | "manage" | "agentRuns";
type FormMode = "create" | "edit";

const emptySummary: DashboardSummary = {
  total_documents: 0,
  published_documents: 0,
  total_questions: 0,
  total_agent_runs: 0
};

const emptyHealth: HealthStatus = {
  service: "zhishu-backend",
  status: "unknown",
  storage_backend: "file",
  route_profile: "file",
  dependencies: {
    mysql: {
      configured: "",
      host: "127.0.0.1",
      port: 3306,
      required: false,
      reachable: false
    },
    redis: {
      configured: "",
      host: "127.0.0.1",
      port: 6379,
      required: false,
      reachable: false
    },
    qdrant: {
      configured: "",
      host: "127.0.0.1",
      port: 6333,
      required: false,
      reachable: false
    },
    minio: {
      configured: "",
      host: "127.0.0.1",
      port: 9000,
      required: false,
      reachable: false,
      bucket: "",
      mode: ""
    }
  }
};

const emptyForm: DocumentFormPayload = {
  title: "",
  summary: "",
  content: "",
  category_name: "制度流程",
  tags: [],
  change_note: "初始化文档"
};

const emptyCategoryForm: CategoryFormPayload = {
  category_name: "",
  description: ""
};

const emptyTagForm: TagFormPayload = {
  tag_name: "",
  description: ""
};

const emptyFaqForm: FaqFormPayload = {
  question: "",
  answer: ""
};

const emptyUserForm: UserCreatePayload = {
  username: "",
  role_name: "普通用户",
  department: "",
  email: "",
  password: ""
};

const dependencyOrder = ["mysql", "redis", "qdrant", "minio"] as const;

const qaProgressSteps = [
  "正在检索知识库",
  "正在匹配证据片段",
  "正在生成最终回答"
] as const;

function isLikelyGarbled(text: string) {
  const trimmed = text.trim();
  if (!trimmed) {
    return false;
  }

  const questionMarkCount = (trimmed.match(/[?]/g) || []).length;
  return questionMarkCount >= 4 && questionMarkCount / Math.max(trimmed.length, 1) > 0.08;
}

function normalizeDisplayText(text: string | null | undefined, fallback = "内容暂不可用") {
  const cleaned = (text ?? "").split("�").join("").split("\r\n").join("\n").trim();
  if (!cleaned) {
    return fallback;
  }

  if (isLikelyGarbled(cleaned)) {
    return fallback;
  }

  return cleaned;
}

function parseMetaJson(metaJson: string | null) {
  if (!metaJson) {
    return [] as Array<{ key: string; value: string }>;
  }

  try {
    const parsed = JSON.parse(metaJson) as Record<string, unknown>;
    return Object.entries(parsed).map(([key, value]) => ({
      key,
      value: typeof value === "string" ? value : JSON.stringify(value)
    }));
  } catch {
    return [{ key: "meta_json", value: metaJson }];
  }
}

export default function App() {
  const [view, setView] = useState<ViewKey>("dashboard");
  const [documents, setDocuments] = useState<DocumentListItem[]>([]);
  const [selectedDocument, setSelectedDocument] = useState<DocumentDetail | null>(null);
  const [dashboard, setDashboard] = useState<DashboardSummary>(emptySummary);
  const [health, setHealth] = useState<HealthStatus>(emptyHealth);
  const [categories, setCategories] = useState<CategoryItem[]>([]);
  const [tags, setTags] = useState<TagItem[]>([]);
  const [roles, setRoles] = useState<RoleItem[]>([]);
  const [users, setUsers] = useState<UserItem[]>([]);
  const [favoriteDocuments, setFavoriteDocuments] = useState<FavoriteDocumentItem[]>([]);
  const [recentReads, setRecentReads] = useState<ReadRecordItem[]>([]);
  const [documentFaqs, setDocumentFaqs] = useState<FaqItem[]>([]);
  const [documentSegments, setDocumentSegments] = useState<DocumentSegment[]>([]);
  const [agentRuns, setAgentRuns] = useState<AgentRun[]>([]);
  const [questionHistory, setQuestionHistory] = useState<QuestionHistoryItem[]>([]);
  const [documentFiles, setDocumentFiles] = useState<DocumentFileMeta[]>([]);
  const [questionText, setQuestionText] = useState("生产环境数据库只读权限如何申请？");
  const [answer, setAnswer] = useState<QaAnswer | null>(null);
  const [currentUser, setCurrentUser] = useState<UserItem | null>(null);
  const [loginForm, setLoginForm] = useState<LoginPayload>({
    username: "admin",
    password: "Admin@123456"
  });
  const [documentForm, setDocumentForm] = useState<DocumentFormPayload>(emptyForm);
  const [categoryForm, setCategoryForm] = useState<CategoryFormPayload>(emptyCategoryForm);
  const [tagForm, setTagForm] = useState<TagFormPayload>(emptyTagForm);
  const [faqForm, setFaqForm] = useState<FaqFormPayload>(emptyFaqForm);
  const [userForm, setUserForm] = useState<UserCreatePayload>(emptyUserForm);
  const [tagInput, setTagInput] = useState("");
  const [editingCategoryName, setEditingCategoryName] = useState<string | null>(null);
  const [editingTagName, setEditingTagName] = useState<string | null>(null);
  const [editingFaqId, setEditingFaqId] = useState<number | null>(null);
  const [editingUserId, setEditingUserId] = useState<number | null>(null);
  const [formMode, setFormMode] = useState<FormMode>("create");
  const [authLoading, setAuthLoading] = useState(true);
  const [authSubmitting, setAuthSubmitting] = useState(false);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [uploadingFile, setUploadingFile] = useState(false);
  const [asking, setAsking] = useState(false);
  const [qaProgressIndex, setQaProgressIndex] = useState(0);
  const [qaHistoryCollapsed, setQaHistoryCollapsed] = useState(true);
  const [expandedRunId, setExpandedRunId] = useState<number | null>(null);
  const [passwordResetValue, setPasswordResetValue] = useState("");
  const [error, setError] = useState<string | null>(null);

  const canManageTaxonomy = currentUser?.role_name === "系统管理员";
  const canManageContent = currentUser ? ["系统管理员", "知识管理员"].includes(currentUser.role_name) : false;
  const canViewUsers = currentUser?.role_name === "系统管理员";
  const canViewAgentRuns = currentUser?.role_name === "系统管理员";
  const dependencyCards = dependencyOrder.map((key) => ({
    key,
    label: key.toUpperCase(),
    item: health.dependencies[key]
  }));

  useEffect(() => {
    void initializeSession();
  }, []);

  useEffect(() => {
    if (!asking) {
      setQaProgressIndex(0);
      return;
    }

    const timer = window.setInterval(() => {
      setQaProgressIndex((current) => (current + 1) % qaProgressSteps.length);
    }, 1200);

    return () => window.clearInterval(timer);
  }, [asking]);

  async function initializeSession() {
    try {
      const user = await api.getCurrentUser();
      setCurrentUser(user);
      await bootstrap(undefined, user);
    } catch {
      clearAuthToken();
      setCurrentUser(null);
      setLoading(false);
    } finally {
      setAuthLoading(false);
    }
  }

  async function bootstrap(preferredDocumentId?: number, userOverride?: UserItem | null) {
    try {
      setLoading(true);
      const activeUser = userOverride ?? currentUser;
      const shouldLoadUsers = activeUser?.role_name === "系统管理员";
      const shouldLoadAgentRuns = activeUser?.role_name === "系统管理员";
      const shouldLoadDocumentFiles = !!activeUser && ["系统管理员", "知识管理员"].includes(activeUser.role_name);
      const [healthData, dashboardData, categoryData, tagData, favoriteData, recentReadData, documentData, questionHistoryData] = await Promise.all([
        api.getHealth(),
        api.getDashboard(),
        api.getCategories(),
        api.getTags(),
        api.getFavorites(),
        api.getRecentReads(),
        api.getDocuments(),
        api.getQuestionHistory()
      ]);

      const [roleData, userData, agentData] = await Promise.all([
        shouldLoadUsers ? api.getRoles() : Promise.resolve([]),
        shouldLoadUsers ? api.getUsers() : Promise.resolve([]),
        shouldLoadAgentRuns ? api.getAgentRuns() : Promise.resolve([])
      ]);
      const fileData = shouldLoadDocumentFiles ? await api.getDocumentFiles() : [];

      setHealth(healthData);
      setDashboard(dashboardData);
      setCategories(categoryData);
      setTags(tagData);
      setRoles(roleData);
      setUsers(userData);
      setFavoriteDocuments(favoriteData);
      setRecentReads(recentReadData);
      setDocuments(documentData);
      setAgentRuns(agentData);
      setQuestionHistory(questionHistoryData);
      setDocumentFiles(fileData);

      const documentId = preferredDocumentId ?? documentData[0]?.document_id;
      if (documentId) {
        const [detail, faqs, segments] = await Promise.all([
          api.getDocument(documentId),
          api.getDocumentFaqs(documentId),
          api.getDocumentSegments(documentId)
        ]);
        setSelectedDocument(detail);
        setDocumentFaqs(faqs);
        setDocumentSegments(segments);
      } else {
        setSelectedDocument(null);
        setDocumentFaqs([]);
        setDocumentSegments([]);
      }
      setError(null);
    } catch (err) {
      if (isUnauthorizedError(err)) {
        logout();
        setError("登录已失效，请重新登录。");
        return;
      }
      setError(err instanceof Error ? err.message : "加载失败");
    } finally {
      setLoading(false);
    }
  }

  async function submitLogin() {
    try {
      setAuthSubmitting(true);
      const session: AuthSession = await api.login(loginForm);
      setAuthToken(session.access_token);
      setCurrentUser(session.user);
      setError(null);
      await bootstrap(undefined, session.user);
    } catch (err) {
      clearAuthToken();
      setCurrentUser(null);
      setError(err instanceof Error ? err.message : "登录失败");
    } finally {
      setAuthSubmitting(false);
      setAuthLoading(false);
    }
  }

  function logout() {
    clearAuthToken();
    setCurrentUser(null);
    setDocuments([]);
    setSelectedDocument(null);
    setAnswer(null);
    setQuestionHistory([]);
    setFavoriteDocuments([]);
    setRecentReads([]);
    setRoles([]);
    setUsers([]);
    setLoading(false);
    setAuthLoading(false);
  }

  function isUnauthorizedError(err: unknown) {
    if (!(err instanceof Error)) {
      return false;
    }

    const message = err.message.toLowerCase();
    return message.includes("401") || message.includes("unauthorized");
  }

  function applyDocumentToForm(detail: DocumentDetail) {
    setDocumentForm({
      title: detail.title,
      summary: detail.summary,
      content: detail.content,
      category_name: detail.category_name,
      tags: detail.tags,
      change_note: "更新文档内容"
    });
    setTagInput(detail.tags.join(", "));
    setFormMode("edit");
  }

  async function openDocument(documentId: number) {
    try {
      await api.recordDocumentRead(documentId);
      const [detail, faqs, latestReads, segments] = await Promise.all([
        api.getDocument(documentId),
        api.getDocumentFaqs(documentId),
        api.getRecentReads(),
        api.getDocumentSegments(documentId)
      ]);
      setSelectedDocument(detail);
      setDocumentFaqs(faqs);
      setRecentReads(latestReads);
      setDocumentSegments(segments);
      applyDocumentToForm(detail);
      setView("documents");
    } catch (err) {
      if (isUnauthorizedError(err)) {
        logout();
        setError("登录已失效，请重新登录。");
        return;
      }
      setError(err instanceof Error ? err.message : "文档详情加载失败");
    }
  }


  function resetForm() {
    setDocumentForm(emptyForm);
    setTagInput("");
    setFormMode("create");
  }

  function resetCategoryForm() {
    setCategoryForm(emptyCategoryForm);
    setEditingCategoryName(null);
  }

  function resetTagForm() {
    setTagForm(emptyTagForm);
    setEditingTagName(null);
  }

  function resetFaqForm() {
    setFaqForm(emptyFaqForm);
    setEditingFaqId(null);
  }

  function resetUserForm() {
    setUserForm(emptyUserForm);
    setEditingUserId(null);
  }

  function syncTags(value: string) {
    setTagInput(value);
    setDocumentForm((prev) => ({
      ...prev,
      tags: value
        .split(",")
        .map((item) => item.trim())
        .filter(Boolean)
    }));
  }

  async function uploadSourceFile(file: File) {
    try {
      setUploadingFile(true);
      const uploaded: DocumentFileMeta = await api.uploadDocumentFile(file);
      setDocumentForm((prev) => ({ ...prev, source_file_id: uploaded.file_id }));
      setDocumentFiles((prev) => [uploaded, ...prev.filter((item) => item.file_id !== uploaded.file_id)]);
      setSelectedDocument((prev) =>
        prev
          ? {
              ...prev,
              source_file: uploaded
            }
          : prev
      );
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "文件上传失败");
    } finally {
      setUploadingFile(false);
    }
  }

  async function downloadSourceFile(file: DocumentFileMeta) {
    try {
      const blob = await api.downloadDocumentFile(file.file_id);
      const url = window.URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = file.original_name;
      document.body.appendChild(anchor);
      anchor.click();
      anchor.remove();
      window.URL.revokeObjectURL(url);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "文件下载失败");
    }
  }

  function beginEditUser(user: UserItem) {
    setUserForm({
      username: user.username,
      role_name: user.role_name,
      department: user.department,
      email: user.email,
      password: ""
    });
    setEditingUserId(user.user_id);
  }

  async function saveUser() {
    try {
      if (!canViewUsers) {
        setError("当前角色不可维护用户");
        return;
      }

      if (!editingUserId && !userForm.password.trim()) {
        setError("新建用户时必须填写初始密码");
        return;
      }

      setSaving(true);
      if (editingUserId) {
        const payload: UserUpdatePayload = {
          ...userForm,
          password: userForm.password.trim() ? userForm.password : undefined
        };
        await api.updateUser(editingUserId, payload);
      } else {
        await api.createUser(userForm);
      }
      resetUserForm();
      setPasswordResetValue("");
      await bootstrap(selectedDocument?.document_id);
      setView("people");
    } catch (err) {
      setError(err instanceof Error ? err.message : "用户保存失败");
    } finally {
      setSaving(false);
    }
  }

  async function saveDocument() {
    try {
      setSaving(true);
      const payload = {
        ...documentForm,
        tags: documentForm.tags.length ? documentForm.tags : ["未分类"]
      };

      let saved: DocumentDetail;
      if (formMode === "edit" && selectedDocument) {
        saved = await api.updateDocument(selectedDocument.document_id, payload);
      } else {
        saved = await api.createDocument(payload);
      }

      setSelectedDocument(saved);
      applyDocumentToForm(saved);
      await bootstrap(saved.document_id);
      setView("documents");
    } catch (err) {
      setError(err instanceof Error ? err.message : "文档保存失败");
    } finally {
      setSaving(false);
    }
  }

  async function publishCurrentDocument() {
    if (!selectedDocument) {
      return;
    }
    try {
      const detail = await api.publishDocument(selectedDocument.document_id);
      setSelectedDocument(detail);
      applyDocumentToForm(detail);
      await bootstrap(detail.document_id);
    } catch (err) {
      setError(err instanceof Error ? err.message : "发布失败");
    }
  }

  async function archiveCurrentDocument() {
    if (!selectedDocument) {
      return;
    }
    try {
      const detail = await api.archiveDocument(selectedDocument.document_id);
      setSelectedDocument(detail);
      applyDocumentToForm(detail);
      await bootstrap(detail.document_id);
    } catch (err) {
      setError(err instanceof Error ? err.message : "归档失败");
    }
  }

  async function toggleFavoriteCurrentDocument() {
    if (!selectedDocument) {
      return;
    }
    try {
      await api.toggleFavoriteDocument(selectedDocument.document_id);
      await bootstrap(selectedDocument.document_id);
    } catch (err) {
      setError(err instanceof Error ? err.message : "收藏操作失败");
    }
  }

  async function askQuestion() {
    const normalizedQuestion = questionText.trim();
    if (!normalizedQuestion) {
      setError("请输入问题后再发起问答");
      setView("qa");
      return;
    }

    try {
      setAsking(true);
      setQaProgressIndex(0);
      setQuestionText(normalizedQuestion);
      setView("qa");
      setError(null);
      const result = await api.askQuestion(normalizedQuestion);
      setAnswer(result);
      await bootstrap(selectedDocument?.document_id);
    } catch (err) {
      setError(err instanceof Error ? err.message : "问答请求失败");
    } finally {
      setAsking(false);
    }
  }

  async function refreshSession() {
    try {
      const session = await api.refreshSession();
      setAuthToken(session.access_token);
      setError(null);
    } catch (err) {
      clearAuthToken();
      setCurrentUser(null);
      setError(err instanceof Error ? err.message : "刷新登录态失败");
    }
  }

  async function resetSelectedUserPassword(userId: number) {
    try {
      if (!passwordResetValue.trim()) {
        setError("请先输入重置后的密码");
        return;
      }
      await api.resetUserPassword(userId, { password: passwordResetValue });
      setPasswordResetValue("");
      await bootstrap(selectedDocument?.document_id);
    } catch (err) {
      setError(err instanceof Error ? err.message : "密码重置失败");
    }
  }

  async function deleteManagedUser(userId: number) {
    try {
      await api.deleteUser(userId);
      if (editingUserId === userId) {
        resetUserForm();
      }
      await bootstrap(selectedDocument?.document_id);
    } catch (err) {
      setError(err instanceof Error ? err.message : "用户删除失败");
    }
  }

  async function reindexCurrentDocument() {
    if (!selectedDocument) {
      return;
    }
    try {
      const detail = await api.reindexDocument(selectedDocument.document_id);
      setSelectedDocument(detail);
      const segments = await api.getDocumentSegments(detail.document_id);
      setDocumentSegments(segments);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "重建分段失败");
    }
  }

  if (authLoading) {
    return <div className="loading auth-loading">正在恢复登录态...</div>;
  }

  if (!currentUser) {
    return (
      <div className="auth-shell">
        <section className="auth-card">
          <div className="brand auth-brand">
            <span className="brand-mark">知</span>
            <div>
              <h1>知枢</h1>
              <p>企业知识资产管理与智能检索平台</p>
            </div>
          </div>
          <h2>登录系统</h2>
          <p className="auth-copy">当前版本已接入基础鉴权。请先登录，再访问文档、问答和后台管理功能。</p>
          {error ? <div className="alert">{error}</div> : null}
          <div className="stack-form">
            <input
              className="compact-input"
              placeholder="用户名"
              value={loginForm.username}
              onChange={(event) => setLoginForm((prev) => ({ ...prev, username: event.target.value }))}
            />
            <input
              className="compact-input"
              type="password"
              placeholder="密码"
              value={loginForm.password}
              onChange={(event) => setLoginForm((prev) => ({ ...prev, password: event.target.value }))}
            />
            <button className="primary" onClick={() => void submitLogin()} disabled={authSubmitting}>
              {authSubmitting ? "登录中..." : "登录并进入系统"}
            </button>
          </div>
          <div className="credentials-tip">
            <strong>演示账号</strong>
            <p>`admin / Admin@123456`</p>
            <p>`editor / Editor@123456`</p>
          </div>
        </section>
      </div>
    );
  }

  async function saveCategory() {
    try {
      if (editingCategoryName) {
        await api.updateCategory(editingCategoryName, categoryForm);
      } else {
        await api.createCategory(categoryForm);
      }
      resetCategoryForm();
      await bootstrap(selectedDocument?.document_id);
      setView("manage");
    } catch (err) {
      setError(err instanceof Error ? err.message : "分类保存失败");
    }
  }

  async function deleteCategory(name: string) {
    try {
      await api.deleteCategory(name);
      if (editingCategoryName === name) {
        resetCategoryForm();
      }
      await bootstrap(selectedDocument?.document_id);
    } catch (err) {
      setError(err instanceof Error ? err.message : "分类删除失败");
    }
  }

  async function saveTag() {
    try {
      if (editingTagName) {
        await api.updateTag(editingTagName, tagForm);
      } else {
        await api.createTag(tagForm);
      }
      resetTagForm();
      await bootstrap(selectedDocument?.document_id);
      setView("manage");
    } catch (err) {
      setError(err instanceof Error ? err.message : "标签保存失败");
    }
  }

  async function deleteTag(name: string) {
    try {
      await api.deleteTag(name);
      if (editingTagName === name) {
        resetTagForm();
      }
      await bootstrap(selectedDocument?.document_id);
    } catch (err) {
      setError(err instanceof Error ? err.message : "标签删除失败");
    }
  }

  async function saveFaq() {
    if (!selectedDocument) {
      setError("请先选择一个文档后再维护 FAQ");
      return;
    }
    try {
      if (editingFaqId) {
        await api.updateFaq(editingFaqId, faqForm);
      } else {
        await api.createDocumentFaq(selectedDocument.document_id, faqForm);
      }
      resetFaqForm();
      await bootstrap(selectedDocument.document_id);
      setView("manage");
    } catch (err) {
      setError(err instanceof Error ? err.message : "FAQ 保存失败");
    }
  }

  async function deleteFaq(faqId: number) {
    try {
      await api.deleteFaq(faqId);
      if (editingFaqId === faqId) {
        resetFaqForm();
      }
      await bootstrap(selectedDocument?.document_id);
    } catch (err) {
      setError(err instanceof Error ? err.message : "FAQ 删除失败");
    }
  }

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <span className="brand-mark">知</span>
          <div>
            <h1>知枢</h1>
            <p>企业知识资产管理与智能检索平台</p>
          </div>
        </div>

        <nav className="nav">
          <button className={view === "dashboard" ? "active" : ""} onClick={() => setView("dashboard")}>
            首页看板
          </button>
          <button className={view === "documents" ? "active" : ""} onClick={() => setView("documents")}>
            文档中心
          </button>
          <button className={view === "qa" ? "active" : ""} onClick={() => setView("qa")}>
            智能问答
          </button>
          <button className={view === "people" ? "active" : ""} onClick={() => setView("people")}>
            用户与行为
          </button>
          {canManageContent ? (
            <button className={view === "manage" ? "active" : ""} onClick={() => setView("manage")}>
              配置管理
            </button>
          ) : null}
          {canViewAgentRuns ? (
            <button className={view === "agentRuns" ? "active" : ""} onClick={() => setView("agentRuns")}>
              Agent 记录
            </button>
          ) : null}
        </nav>

        <section className="question-card">
          <h2>快速提问</h2>
          <textarea value={questionText} onChange={(event) => setQuestionText(event.target.value)} rows={4} />
          <button className="primary" onClick={askQuestion} disabled={asking}>
            {asking ? "问答生成中..." : "发起问答"}
          </button>
        </section>
      </aside>

      <main className="main-panel">
        <header className="header">
          <div>
            <h2>可交付 MVP+</h2>
            <p>围绕 MySQL 主链路，串起文档管理、问答检索、对象镜像和向量演示的数据库课程答辩 Demo。</p>
          </div>
          <div className="header-actions">
            <div className="user-badge">
              <strong>{currentUser.username}</strong>
              <span>{currentUser.role_name}</span>
            </div>
            <button className="ghost" onClick={resetForm}>
              新建模式
            </button>
            <button className="ghost" onClick={() => void bootstrap(selectedDocument?.document_id)}>
              刷新数据
            </button>
            <button className="ghost" onClick={() => void refreshSession()}>
              刷新令牌
            </button>
            <button className="ghost" onClick={logout}>
              退出登录
            </button>
          </div>
        </header>

        {error ? <div className="alert">{error}</div> : null}
        {loading ? <div className="loading">正在加载演示数据...</div> : null}

        {!loading && view === "dashboard" ? (
          <>
            <section className="panel hero-panel">
              <div className="hero-copy">
                <span className="hero-kicker">数据库系统课程演示</span>
                <h3>这不是单纯的前端页面，而是一套可运行的知识库数据闭环</h3>
                <p>
                  前台展示的是知识检索体验，背后重点演示的是关系型数据库落库、文档版本化、分段索引、
                  问答引用和 Agent 留痕这几条核心数据链路。
                </p>
              </div>
              <div className="hero-metrics">
                <div className="hero-metric">
                  <span>主存储</span>
                  <strong>{health.storage_backend.toUpperCase()}</strong>
                  <small>{health.dependencies.mysql.reachable ? "MySQL 已连通" : "MySQL 待连接"}</small>
                </div>
                <div className="hero-metric">
                  <span>部署路线</span>
                  <strong>{health.route_profile.toUpperCase()}</strong>
                  <small>当前以本地可演示为目标</small>
                </div>
                <div className="hero-metric">
                  <span>检索增强</span>
                  <strong>{health.dependencies.qdrant.reachable ? "QDRANT" : "DEMO HASH"}</strong>
                  <small>{health.dependencies.qdrant.reachable ? "向量检索已打通" : "可先演示基础问答链路"}</small>
                </div>
              </div>
              <div className="hero-tags">
                <span className="tag">文档版本管理</span>
                <span className="tag">FAQ 主数据</span>
                <span className="tag">问答引用证据</span>
                <span className="tag">Agent 运行留痕</span>
              </div>
            </section>
            <section className="dashboard-grid">
              <SummaryCard label="文档总量" value={dashboard.total_documents} hint="知识资产主表规模" />
              <SummaryCard label="已发布文档" value={dashboard.published_documents} hint="可被检索与问答消费" />
              <SummaryCard label="累计问答" value={dashboard.total_questions} hint="问题与答案链路沉淀" />
              <SummaryCard label="Agent 运行数" value={dashboard.total_agent_runs} hint="自动化处理审计轨迹" />
            </section>
            <section className="content-layout dashboard-detail-layout">
              <div className="panel">
                <div className="panel-title">
                  <h3>分类概览</h3>
                  <span>{categories.length} 个分类</span>
                </div>
                <div className="history-list">
                  {categories.map((item) => (
                    <div className="history-item" key={item.category_name}>
                      <strong>{item.category_name}</strong>
                      <p>{item.description}</p>
                      <small>{item.document_count} 篇文档</small>
                    </div>
                  ))}
                </div>
              </div>

              <div className="panel">
                <div className="panel-title">
                  <h3>标签概览</h3>
                  <span>{tags.length} 个标签</span>
                </div>
                <div className="tag-cloud">
                  {tags.map((item) => (
                    <span className="tag large" key={item.tag_name}>
                      {item.tag_name} · {item.document_count}
                    </span>
                  ))}
                </div>
              </div>
            </section>
            <section className="content-layout dashboard-detail-layout">
              <div className="panel">
                <div className="panel-title">
                  <h3>数据库展示重点</h3>
                  <span>答辩建议从这里讲</span>
                </div>
                <div className="history-list">
                  <div className="history-item emphasis-item">
                    <strong>1. 文档不是一张表结束，而是版本化管理</strong>
                    <p>每次修改都会形成新的版本记录，便于追踪知识演进，体现数据库对历史状态的管理能力。</p>
                  </div>
                  <div className="history-item emphasis-item">
                    <strong>2. 问答结果不是黑盒，能回溯到片段与来源</strong>
                    <p>回答会绑定引用证据和 segment_id，说明检索结果可以被解释、被审计、被复查。</p>
                  </div>
                  <div className="history-item emphasis-item">
                    <strong>3. Agent 运行过程也落库</strong>
                    <p>摘要、问答、审计等自动任务都会留下运行记录，方便后续统计、排障和行为分析。</p>
                  </div>
                </div>
              </div>
              <div className="panel">
                <div className="panel-title">
                  <h3>依赖总览</h3>
                  <span>面向演示的运行状态</span>
                </div>
                <div className="dependency-grid">
                  {dependencyCards.map(({ key, label, item }) => (
                    <div className="dependency-card" key={key}>
                      <div className="dependency-head">
                        <strong>{label}</strong>
                        <span className={item.reachable ? "status-pill online" : "status-pill offline"}>
                          {item.reachable ? "在线" : item.required ? "待处理" : "可选"}
                        </span>
                      </div>
                      <p>
                        {item.host}:{item.port}
                      </p>
                      <small>{item.required ? "主链路依赖" : "增强能力依赖"}</small>
                    </div>
                  ))}
                </div>
              </div>
            </section>
          </>
        ) : null}

        {!loading && view === "documents" ? (
          <section className="content-layout">
            <div className="panel list-panel">
              <div className="panel-title">
                <h3>文档列表</h3>
                <span>{documents.length} 篇</span>
              </div>
              <div className="document-list">
                {documents.map((item) => (
                  <button key={item.document_id} className="document-item" onClick={() => void openDocument(item.document_id)}>
                    <strong>{item.title}</strong>
                    <span>{item.category_name}</span>
                    <small>{item.status} · {item.version_no}{item.is_favorite ? " · 已收藏" : ""}</small>
                    <p>{item.summary}</p>
                  </button>
                ))}
              </div>
            </div>

            <div className="panel detail-panel">
              <div className="panel-title">
                <div>
                  <h3>{formMode === "edit" ? "编辑文档" : "新建文档"}</h3>
                  <span>{selectedDocument ? `当前查看：${selectedDocument.title}` : "当前未选中文档"}</span>
                </div>
                <div className="panel-actions">
                  {selectedDocument ? (
                    <>
                      <button className="ghost" onClick={() => applyDocumentToForm(selectedDocument)}>
                        载入当前文档
                      </button>
                      <button className={selectedDocument.is_favorite ? "warning" : "ghost"} onClick={() => void toggleFavoriteCurrentDocument()}>
                        {selectedDocument.is_favorite ? "取消收藏" : "加入收藏"}
                      </button>
                      <button className="ghost" onClick={() => void reindexCurrentDocument()}>
                        重建分段
                      </button>
                      {canManageContent ? (
                        <>
                          <button className="primary" onClick={() => void publishCurrentDocument()}>
                            发布
                          </button>
                          <button className="danger" onClick={() => void archiveCurrentDocument()}>
                            归档
                          </button>
                        </>
                      ) : null}
                    </>
                  ) : null}
                </div>
              </div>

              <div className="form-grid">
                <label>
                  <span>标题</span>
                  <input
                    value={documentForm.title}
                    onChange={(event) => setDocumentForm((prev) => ({ ...prev, title: event.target.value }))}
                  />
                </label>
                <label>
                  <span>分类</span>
                  <input
                    value={documentForm.category_name}
                    onChange={(event) => setDocumentForm((prev) => ({ ...prev, category_name: event.target.value }))}
                  />
                </label>
                <label className="full">
                  <span>摘要</span>
                  <textarea
                    rows={3}
                    value={documentForm.summary}
                    onChange={(event) => setDocumentForm((prev) => ({ ...prev, summary: event.target.value }))}
                  />
                </label>
                <label className="full">
                  <span>正文</span>
                  <textarea
                    rows={8}
                    value={documentForm.content}
                    onChange={(event) => setDocumentForm((prev) => ({ ...prev, content: event.target.value }))}
                  />
                </label>
                <label>
                  <span>标签（逗号分隔）</span>
                  <input value={tagInput} onChange={(event) => syncTags(event.target.value)} />
                </label>
                <label>
                  <span>变更说明</span>
                  <input
                    value={documentForm.change_note}
                    onChange={(event) => setDocumentForm((prev) => ({ ...prev, change_note: event.target.value }))}
                  />
                </label>
                <label className="full">
                  <span>原始文件</span>
                  <select
                    value={documentForm.source_file_id ?? ""}
                    onChange={(event) =>
                      setDocumentForm((prev) => ({
                        ...prev,
                        source_file_id: event.target.value ? Number(event.target.value) : undefined
                      }))
                    }
                  >
                    <option value="">不绑定原始文件</option>
                    {documentFiles.map((file) => (
                      <option key={file.file_id} value={file.file_id}>
                        {file.original_name} · ID {file.file_id}
                      </option>
                    ))}
                  </select>
                  <input
                    type="file"
                    onChange={(event) => {
                      const file = event.target.files?.[0];
                      if (file) {
                        void uploadSourceFile(file);
                      }
                      event.currentTarget.value = "";
                    }}
                  />
                  <small>
                    {uploadingFile
                      ? "正在上传文件..."
                      : documentForm.source_file_id
                        ? `已挂接源文件 ID：${documentForm.source_file_id}`
                        : "当前未挂接原始文件"}
                  </small>
                </label>
              </div>

              <div className="form-actions">
                <button className="ghost" onClick={resetForm}>
                  清空表单
                </button>
                {canManageContent ? (
                  <button className="primary" onClick={() => void saveDocument()} disabled={saving}>
                    {saving ? "保存中..." : formMode === "edit" ? "保存并生成新版本" : "创建文档"}
                  </button>
                ) : null}
              </div>

              {selectedDocument ? (
                <>
                  <div className="tag-row">
                    {selectedDocument.tags.map((tag) => (
                      <span className="tag" key={tag}>{tag}</span>
                    ))}
                    {selectedDocument.is_favorite ? <span className="tag favorite">已收藏</span> : null}
                  </div>
                  <article className="article-card">
                    <h4>当前摘要</h4>
                    <p>{selectedDocument.summary}</p>
                    <h4>原始文件</h4>
                    {selectedDocument.source_file ? (
                      <>
                        <p>
                          {`${selectedDocument.source_file.original_name} · ${selectedDocument.source_file.mime_type} · ${selectedDocument.source_file.file_size} bytes · ${selectedDocument.source_file.object_key}`}
                        </p>
                        <div className="inline-actions">
                          <button
                            className="ghost"
                            onClick={() => void downloadSourceFile(selectedDocument.source_file!)}
                          >
                            下载原始文件
                          </button>
                        </div>
                      </>
                    ) : (
                      <p>当前文档未绑定原始文件</p>
                    )}
                    <h4>当前正文</h4>
                    <pre>{selectedDocument.content}</pre>
                  </article>
                  <section className="version-list">
                    <h4>版本记录</h4>
                    {selectedDocument.versions.map((version) => (
                      <div className="version-item" key={version.version_id}>
                        <strong>{version.version_no}</strong>
                        <span>{version.change_note}</span>
                        <small>{version.source_file_id ? `源文件ID ${version.source_file_id}` : "无源文件"}</small>
                        <small>{new Date(version.created_at).toLocaleString("zh-CN")}</small>
                      </div>
                    ))}
                  </section>
                  <section className="version-list">
                    <h4>当前分段</h4>
                    {documentSegments.length ? (
                      documentSegments.map((segment) => (
                        <div className="version-item" key={segment.segment_id}>
                          <strong>{`片段 #${segment.chunk_order}`}</strong>
                          <span>{segment.embedding_status}</span>
                          <p>{segment.chunk_text}</p>
                          <small>{`segment_id ${segment.segment_id} · version_id ${segment.version_id}`}</small>
                        </div>
                      ))
                    ) : (
                      <div className="empty-state">当前文档还没有可展示的分段</div>
                    )}
                  </section>
                  <section className="version-list">
                    <h4>FAQ 条目</h4>
                    {documentFaqs.length ? (
                      documentFaqs.map((faq) => (
                        <div className="version-item" key={faq.faq_id}>
                          <strong>{faq.question}</strong>
                          <p>{faq.answer}</p>
                          <small>{new Date(faq.created_at).toLocaleString("zh-CN")}</small>
                        </div>
                      ))
                    ) : (
                      <div className="empty-state">当前文档暂无 FAQ 条目</div>
                    )}
                  </section>
                </>
              ) : (
                <div className="empty-state">填写右侧表单后即可创建第一篇文档</div>
              )}
            </div>
          </section>
        ) : null}

        {!loading && view === "qa" ? (
          <section className={`content-layout qa-layout ${qaHistoryCollapsed ? "qa-layout-collapsed" : ""}`}>
            <div className="panel qa-main-panel">
              <div className="panel-title qa-main-title">
                <div>
                  <h3>智能问答</h3>
                  <span>类比对话助手的交互体验，并清晰展示检索与生成过程</span>
                </div>
                <button className="ghost qa-toggle-button" onClick={() => setQaHistoryCollapsed((prev) => !prev)}>
                  {qaHistoryCollapsed ? "展开历史" : "收起历史"}
                </button>
              </div>

              <div className="qa-chat-shell">
                <div className="qa-thread">
                  <article className="qa-message qa-message-assistant qa-message-intro">
                    <div className="qa-avatar">AI</div>
                    <div className="qa-bubble">
                      <strong>欢迎使用</strong>
                      <p>你可以直接询问制度流程、数据库权限、VPN 远程接入、发版回滚或数据导出规范，我会先检索知识库证据，再生成答案。</p>
                    </div>
                  </article>

                  {(asking || answer) && questionText.trim() ? (
                    <article className="qa-message qa-message-user">
                      <div className="qa-bubble">
                        <strong>我的问题</strong>
                        <p>{normalizeDisplayText(questionText, "请输入一个具体问题")}</p>
                      </div>
                    </article>
                  ) : null}

                  {asking ? (
                    <article className="qa-message qa-message-assistant">
                      <div className="qa-avatar">AI</div>
                      <div className="qa-bubble qa-bubble-loading">
                        <strong>处理中</strong>
                        <p>系统正在依次执行检索、证据匹配和回答生成，你可以把这个过程理解成一个可追踪的 Agent 执行链路。</p>
                        <div className="qa-progress-list">
                          {qaProgressSteps.map((step, index) => (
                            <div
                              className={`qa-progress-item ${index < qaProgressIndex ? "done" : ""} ${index === qaProgressIndex ? "active" : ""}`}
                              key={step}
                            >
                              <span className="qa-progress-dot">{index + 1}</span>
                              <div>
                                <strong>{step}</strong>
                                <small>
                                  {index < qaProgressIndex
                                    ? "已完成"
                                    : index === qaProgressIndex
                                      ? "进行中"
                                      : "等待执行"}
                                </small>
                              </div>
                            </div>
                          ))}
                        </div>
                      </div>
                    </article>
                  ) : null}

                  {answer ? (
                    <article className="qa-message qa-message-assistant">
                      <div className="qa-avatar">AI</div>
                      <div className="qa-bubble">
                        <strong>系统回答</strong>
                        <p>{normalizeDisplayText(answer.answer_text, "当前未生成可展示的回答，请重新提问")}</p>
                        <div className="qa-answer-meta">
                          <span>置信度 {answer.confidence_score.toFixed(2)}</span>
                          <span>{answer.citations.length} 条引用证据</span>
                        </div>
                      </div>
                    </article>
                  ) : null}

                  {!asking && !answer ? (
                    <div className="empty-state qa-empty-state">在下方输入问题后，页面会按“提问 → 检索证据 → 生成回答”的顺序展示整个过程。</div>
                  ) : null}
                </div>

                <section className="qa-composer qa-composer-chat">
                  <label htmlFor="qa-question">问题输入区</label>
                  <textarea
                    id="qa-question"
                    className="compact-input qa-textarea"
                    rows={4}
                    placeholder="例如：生产环境数据库只读权限如何申请？"
                    value={questionText}
                    onChange={(event) => setQuestionText(event.target.value)}
                  />
                  <div className="inline-actions qa-composer-actions">
                    <button className="ghost" onClick={() => setQuestionText("生产环境数据库只读权限如何申请？")}>
                      恢复示例问题
                    </button>
                    <button className="primary" onClick={() => void askQuestion()} disabled={asking}>
                      {asking ? qaProgressSteps[qaProgressIndex] : "提交问题"}
                    </button>
                  </div>
                </section>

                {answer?.citations.length ? (
                  <section className="citation-list qa-citation-list">
                    <h4>引用证据</h4>
                    {answer.citations.map((citation) => (
                      <div className="citation-item" key={citation.cite_order}>
                        <strong>{normalizeDisplayText(citation.document_title, "未命名文档")}</strong>
                        <span>
                          {citation.version_no} | 得分 {citation.score.toFixed(2)} | {citation.segment_id ? `片段 ${citation.segment_id}` : "无片段编号"}
                        </span>
                        <p>{normalizeDisplayText(citation.snippet_text, "该证据片段暂不可展示")}</p>
                      </div>
                    ))}
                  </section>
                ) : null}
              </div>
            </div>

            {!qaHistoryCollapsed ? (
              <div className="panel qa-side-panel">
                <div className="panel-title">
                  <h3>问答历史</h3>
                  <span>{questionHistory.length} 条</span>
                </div>
                <div className="history-list">
                  {questionHistory.map((item) => (
                    <button
                      className="history-item history-button"
                      key={item.question_id}
                      onClick={() => setQuestionText(item.question_text)}
                    >
                      <strong>{normalizeDisplayText(item.question_text, "历史问题")}</strong>
                      <p>{normalizeDisplayText(item.answer_preview, "这条历史记录存在旧数据编码噪声，建议重新提问生成新答案。")}</p>
                      <small>{new Date(item.created_at).toLocaleString("zh-CN")}</small>
                    </button>
                  ))}
                </div>
              </div>
            ) : null}
          </section>
        ) : null}

        {!loading && view === "people" ? (
          <section className="content-layout people-layout">
            <div className="panel">
              <div className="panel-title">
                <h3>角色概览</h3>
                <span>{roles.length} 个角色</span>
              </div>
              <div className="history-list">
                {canViewUsers ? roles.map((item) => (
                  <div className="history-item" key={item.role_name}>
                    <strong>{item.role_name}</strong>
                    <p>{item.description || "未配置角色说明"}</p>
                    <small>{item.user_count} 位用户</small>
                  </div>
                )) : <div className="empty-state">当前角色不可查看完整用户目录</div>}
              </div>
            </div>

            <div className="panel">
              <div className="panel-title">
                <h3>用户目录</h3>
                <span>{users.length} 位</span>
              </div>
              {canViewUsers ? (
                <>
                  <div className="stack-form">
                    <input
                      className="compact-input"
                      placeholder="用户名"
                      value={userForm.username}
                      onChange={(event) => setUserForm((prev) => ({ ...prev, username: event.target.value }))}
                    />
                    <select
                      className="compact-input"
                      value={userForm.role_name}
                      onChange={(event) => setUserForm((prev) => ({ ...prev, role_name: event.target.value }))}
                    >
                      {roles.map((item) => (
                        <option key={item.role_name} value={item.role_name}>
                          {item.role_name}
                        </option>
                      ))}
                    </select>
                    <input
                      className="compact-input"
                      placeholder="所属部门"
                      value={userForm.department}
                      onChange={(event) => setUserForm((prev) => ({ ...prev, department: event.target.value }))}
                    />
                    <input
                      className="compact-input"
                      placeholder="邮箱"
                      value={userForm.email}
                      onChange={(event) => setUserForm((prev) => ({ ...prev, email: event.target.value }))}
                    />
                    <input
                      className="compact-input"
                      type="password"
                      placeholder={editingUserId ? "留空则保持原密码" : "初始密码"}
                      value={userForm.password}
                      onChange={(event) => setUserForm((prev) => ({ ...prev, password: event.target.value }))}
                    />
                    <div className="inline-actions">
                      <button className="ghost" onClick={resetUserForm}>清空</button>
                      <button className="primary" onClick={() => void saveUser()} disabled={saving}>
                        {saving ? "保存中..." : editingUserId ? "保存用户" : "新增用户"}
                      </button>
                    </div>
                  </div>
                  <div className="history-list">
                    {users.map((item) => (
                      <div className="history-item" key={item.user_id}>
                        <strong>{item.username}</strong>
                        <p>{item.role_name} · {item.department || "未分配部门"}</p>
                        <small>{item.email || "未配置邮箱"}</small>
                        <input
                          className="compact-input"
                          type="password"
                          placeholder="重置密码"
                          value={editingUserId === item.user_id ? passwordResetValue : ""}
                          onFocus={() => setEditingUserId(item.user_id)}
                          onChange={(event) => {
                            setEditingUserId(item.user_id);
                            setPasswordResetValue(event.target.value);
                          }}
                        />
                        <div className="inline-actions">
                          <button className="ghost" onClick={() => beginEditUser(item)}>
                            编辑
                          </button>
                          <button className="ghost" onClick={() => void resetSelectedUserPassword(item.user_id)}>
                            重置密码
                          </button>
                          {item.user_id !== currentUser?.user_id ? (
                            <button className="danger" onClick={() => void deleteManagedUser(item.user_id)}>
                              删除
                            </button>
                          ) : null}
                        </div>
                      </div>
                    ))}
                  </div>
                </>
              ) : (
                <div className="empty-state">当前角色不可查看完整用户目录</div>
              )}
            </div>

            <div className="panel">
              <div className="panel-title">
                <h3>我的收藏</h3>
                <span>{favoriteDocuments.length} 篇</span>
              </div>
              <div className="history-list">
                {favoriteDocuments.length ? favoriteDocuments.map((item) => (
                  <button className="history-item history-button" key={item.document_id} onClick={() => void openDocument(item.document_id)}>
                    <strong>{item.title}</strong>
                    <p>{item.category_name} · {item.status} · {item.version_no}</p>
                    <small>{new Date(item.favorite_time).toLocaleString("zh-CN")}</small>
                  </button>
                )) : <div className="empty-state">当前还没有收藏文档</div>}
              </div>
            </div>

            <div className="panel">
              <div className="panel-title">
                <h3>最近阅读</h3>
                <span>{recentReads.length} 条</span>
              </div>
              <div className="history-list">
                {recentReads.length ? recentReads.map((item) => (
                  <button className="history-item history-button" key={item.read_id} onClick={() => void openDocument(item.document_id)}>
                    <strong>{item.title}</strong>
                    <p>{item.category_name} · {item.status} · {item.version_no}</p>
                    <small>{new Date(item.read_time).toLocaleString("zh-CN")}</small>
                  </button>
                )) : <div className="empty-state">当前还没有阅读记录</div>}
              </div>
            </div>
          </section>
        ) : null}

        {!loading && view === "manage" && canManageContent ? (
          <section className="content-layout people-layout">
            <div className="panel">
              <div className="panel-title">
                <h3>依赖状态</h3>
                <span>{health.storage_backend} / {health.route_profile}</span>
              </div>
              <div className="dependency-grid">
                {dependencyCards.map(({ key, label, item }) => {
                   return (
                    <div className="dependency-card detail" key={key}>
                      <div className="dependency-head">
                        <strong>{label}</strong>
                        <span className={item.reachable ? "status-pill online" : "status-pill offline"}>
                          {item.reachable ? "已连通" : item.required ? "未连通" : "增强项"}
                        </span>
                      </div>
                      <p>
                        {item.required ? "当前主链路依赖" : "当前增强依赖"}
                      </p>
                      <small>
                        {item.host}:{item.port}
                        {key === "minio" && item.bucket ? ` · bucket ${item.bucket}` : ""}
                        {key === "minio" && item.mode ? ` · ${item.mode}` : ""}
                      </small>
                      <small>{item.configured}</small>
                    </div>
                  );
                })}
              </div>
            </div>

            {canManageTaxonomy ? (
              <>
                <div className="panel">
                  <div className="panel-title">
                    <h3>分类管理</h3>
                    <span>{categories.length} 个分类</span>
                  </div>
                  <div className="stack-form">
                    <input
                      className="compact-input"
                      placeholder="分类名称"
                      value={categoryForm.category_name}
                      onChange={(event) => setCategoryForm((prev) => ({ ...prev, category_name: event.target.value }))}
                    />
                    <textarea
                      className="compact-input"
                      rows={3}
                      placeholder="分类说明"
                      value={categoryForm.description}
                      onChange={(event) => setCategoryForm((prev) => ({ ...prev, description: event.target.value }))}
                    />
                    <div className="inline-actions">
                      <button className="ghost" onClick={resetCategoryForm}>清空</button>
                      <button className="primary" onClick={() => void saveCategory()}>
                        {editingCategoryName ? "保存分类" : "新增分类"}
                      </button>
                    </div>
                  </div>
                  <div className="history-list">
                    {categories.map((item) => (
                      <div className="history-item" key={item.category_name}>
                        <strong>{item.category_name}</strong>
                        <p>{item.description}</p>
                        <small>{item.document_count} 篇文档</small>
                        <div className="inline-actions">
                          <button
                            className="ghost"
                            onClick={() => {
                              setCategoryForm({
                                category_name: item.category_name,
                                description: item.description
                              });
                              setEditingCategoryName(item.category_name);
                            }}
                          >
                            编辑
                          </button>
                          <button className="danger" onClick={() => void deleteCategory(item.category_name)}>
                            删除
                          </button>
                        </div>
                      </div>
                    ))}
                  </div>
                </div>

                <div className="panel">
                  <div className="panel-title">
                    <h3>标签管理</h3>
                    <span>{tags.length} 个标签</span>
                  </div>
                  <div className="stack-form">
                    <input
                      className="compact-input"
                      placeholder="标签名称"
                      value={tagForm.tag_name}
                      onChange={(event) => setTagForm((prev) => ({ ...prev, tag_name: event.target.value }))}
                    />
                    <textarea
                      className="compact-input"
                      rows={3}
                      placeholder="标签说明"
                      value={tagForm.description}
                      onChange={(event) => setTagForm((prev) => ({ ...prev, description: event.target.value }))}
                    />
                    <div className="inline-actions">
                      <button className="ghost" onClick={resetTagForm}>清空</button>
                      <button className="primary" onClick={() => void saveTag()}>
                        {editingTagName ? "保存标签" : "新增标签"}
                      </button>
                    </div>
                  </div>
                  <div className="history-list">
                    {tags.map((item) => (
                      <div className="history-item" key={item.tag_name}>
                        <strong>{item.tag_name}</strong>
                        <p>{item.description}</p>
                        <small>{item.document_count} 篇文档</small>
                        <div className="inline-actions">
                          <button
                            className="ghost"
                            onClick={() => {
                              setTagForm({
                                tag_name: item.tag_name,
                                description: item.description
                              });
                              setEditingTagName(item.tag_name);
                            }}
                          >
                            编辑
                          </button>
                          <button className="danger" onClick={() => void deleteTag(item.tag_name)}>
                            删除
                          </button>
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              </>
            ) : (
              <div className="panel">
                <div className="panel-title">
                  <h3>分类与标签</h3>
                  <span>仅管理员可维护</span>
                </div>
                <div className="empty-state">当前角色只能维护文档内容与 FAQ，不能修改分类和标签主数据。</div>
              </div>
            )}

            <div className="panel">
              <div className="panel-title">
                <div>
                  <h3>FAQ 管理</h3>
                  <span>{selectedDocument ? `当前文档：${selectedDocument.title}` : "请选择文档"}</span>
                </div>
              </div>
              <div className="stack-form">
                <select
                  className="compact-input"
                  value={selectedDocument?.document_id ?? ""}
                  onChange={(event) => {
                    const value = Number(event.target.value);
                    if (value) {
                      void openDocument(value);
                    }
                  }}
                >
                  <option value="">请选择文档</option>
                  {documents.map((item) => (
                    <option key={item.document_id} value={item.document_id}>
                      {item.title}
                    </option>
                  ))}
                </select>
                <input
                  className="compact-input"
                  placeholder="FAQ 问题"
                  value={faqForm.question}
                  onChange={(event) => setFaqForm((prev) => ({ ...prev, question: event.target.value }))}
                />
                <textarea
                  className="compact-input"
                  rows={4}
                  placeholder="FAQ 回答"
                  value={faqForm.answer}
                  onChange={(event) => setFaqForm((prev) => ({ ...prev, answer: event.target.value }))}
                />
                <div className="inline-actions">
                  <button className="ghost" onClick={resetFaqForm}>清空</button>
                  <button className="primary" onClick={() => void saveFaq()}>
                    {editingFaqId ? "保存 FAQ" : "新增 FAQ"}
                  </button>
                </div>
              </div>
              <div className="history-list">
                {documentFaqs.length ? documentFaqs.map((item) => (
                  <div className="history-item" key={item.faq_id}>
                    <strong>{item.question}</strong>
                    <p>{item.answer}</p>
                    <small>{new Date(item.created_at).toLocaleString("zh-CN")}</small>
                    <div className="inline-actions">
                      <button
                        className="ghost"
                        onClick={() => {
                          setFaqForm({
                            question: item.question,
                            answer: item.answer
                          });
                          setEditingFaqId(item.faq_id);
                        }}
                      >
                        编辑
                      </button>
                      <button className="danger" onClick={() => void deleteFaq(item.faq_id)}>
                        删除
                      </button>
                    </div>
                  </div>
                )) : <div className="empty-state">当前文档暂无 FAQ，可直接新增</div>}
              </div>
            </div>
          </section>
        ) : null}

        {!loading && view === "agentRuns" && canViewAgentRuns ? (
          <section className="panel">
            <div className="panel-title">
              <h3>Agent 运行记录</h3>
              <span>摘要、问答、审计执行链路</span>
            </div>
            <div className="run-table">
              {agentRuns.map((run) => {
                const contextKeys = [
                  run.document_id ? `文档 ${run.document_id}` : "",
                  run.version_id ? `版本 ${run.version_id}` : "",
                  run.question_id ? `问题 ${run.question_id}` : "",
                  run.answer_id ? `回答 ${run.answer_id}` : ""
                ].filter(Boolean);
                const metaItems = parseMetaJson(run.meta_json);
                const isExpanded = expandedRunId === run.run_id;

                return (
                  <div className="run-row" key={run.run_id}>
                    <div className="run-header">
                      <div>
                        <strong>{run.agent_type === "summary" ? "文档摘要任务" : "问答任务"}</strong>
                        <p>{run.trigger_type} · {run.status}</p>
                      </div>
                      <span className={`status-pill ${run.status === "success" ? "online" : "offline"}`}>
                        {run.status === "success" ? "成功" : run.status}
                      </span>
                    </div>

                    <div className="run-preview-grid">
                      <div className="run-preview-card">
                        <strong>输入摘要</strong>
                        <p>{normalizeDisplayText(run.input_text, "输入内容为空")}</p>
                      </div>
                      <div className="run-preview-card">
                        <strong>输出摘要</strong>
                        <p>{normalizeDisplayText(run.output_text, "该条 Agent 输出存在编码噪声")}</p>
                      </div>
                    </div>

                    <div className="run-context-row">
                      <span>{contextKeys.join(" · ") || "无上下文主键"}</span>
                      <span>
                        {new Date(run.started_at).toLocaleString("zh-CN")}
                        {run.finished_at ? ` → ${new Date(run.finished_at).toLocaleString("zh-CN")}` : ""}
                      </span>
                    </div>

                    <div className="inline-actions run-actions">
                      <button
                        className="ghost"
                        onClick={() => setExpandedRunId((current) => (current === run.run_id ? null : run.run_id))}
                      >
                        {isExpanded ? "收起详情" : "展开详情"}
                      </button>
                    </div>

                    {isExpanded ? (
                      <div className="run-detail-grid">
                        <div className="run-detail-card">
                          <strong>完整输入</strong>
                          <p>{normalizeDisplayText(run.input_text, "输入内容为空")}</p>
                        </div>
                        <div className="run-detail-card">
                          <strong>完整输出</strong>
                          <p>{normalizeDisplayText(run.output_text, "该条 Agent 输出存在编码噪声")}</p>
                        </div>
                        {metaItems.length ? (
                          <div className="run-detail-card run-meta-card">
                            <strong>执行元数据</strong>
                            <div className="run-meta-list">
                              {metaItems.map((item) => (
                                <div className="run-meta-item" key={`${run.run_id}-${item.key}`}>
                                  <span>{item.key}</span>
                                  <code>{normalizeDisplayText(item.value, "元数据为空")}</code>
                                </div>
                              ))}
                            </div>
                          </div>
                        ) : null}
                      </div>
                    ) : null}
                  </div>
                );
              })}
            </div>
          </section>
        ) : null}
      </main>
    </div>
  );
}

function SummaryCard(props: { label: string; value: number; hint: string }) {
  return (
    <div className="summary-card">
      <span>{props.label}</span>
      <strong>{props.value}</strong>
      <small>{props.hint}</small>
    </div>
  );
}




