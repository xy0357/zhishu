import { useEffect, useState } from "react";
import { api, clearAuthToken, setAuthToken } from "./api";
import type {
  AgentRun,
  AuthSession,
  CategoryItem,
  CategoryFormPayload,
  DashboardSummary,
  DocumentDetail,
  DocumentFormPayload,
  DocumentListItem,
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

export default function App() {
  const [view, setView] = useState<ViewKey>("dashboard");
  const [documents, setDocuments] = useState<DocumentListItem[]>([]);
  const [selectedDocument, setSelectedDocument] = useState<DocumentDetail | null>(null);
  const [dashboard, setDashboard] = useState<DashboardSummary>(emptySummary);
  const [categories, setCategories] = useState<CategoryItem[]>([]);
  const [tags, setTags] = useState<TagItem[]>([]);
  const [roles, setRoles] = useState<RoleItem[]>([]);
  const [users, setUsers] = useState<UserItem[]>([]);
  const [favoriteDocuments, setFavoriteDocuments] = useState<FavoriteDocumentItem[]>([]);
  const [recentReads, setRecentReads] = useState<ReadRecordItem[]>([]);
  const [documentFaqs, setDocumentFaqs] = useState<FaqItem[]>([]);
  const [agentRuns, setAgentRuns] = useState<AgentRun[]>([]);
  const [questionHistory, setQuestionHistory] = useState<QuestionHistoryItem[]>([]);
  const [questionText, setQuestionText] = useState("如何申请数据库权限？");
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
  const [error, setError] = useState<string | null>(null);

  const canManageTaxonomy = currentUser?.role_name === "系统管理员";
  const canManageContent = currentUser ? ["系统管理员", "知识管理员"].includes(currentUser.role_name) : false;
  const canViewUsers = currentUser?.role_name === "系统管理员";
  const canViewAgentRuns = currentUser?.role_name === "系统管理员";

  useEffect(() => {
    void initializeSession();
  }, []);

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
      const [dashboardData, categoryData, tagData, favoriteData, recentReadData, documentData, questionHistoryData] = await Promise.all([
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

      const documentId = preferredDocumentId ?? documentData[0]?.document_id;
      if (documentId) {
        const [detail, faqs] = await Promise.all([
          api.getDocument(documentId),
          api.getDocumentFaqs(documentId)
        ]);
        setSelectedDocument(detail);
        setDocumentFaqs(faqs);
      } else {
        setSelectedDocument(null);
        setDocumentFaqs([]);
      }
      setError(null);
    } catch (err) {
      if (err instanceof Error && err.message.includes("401")) {
        logout();
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
      const [detail, faqs, latestReads] = await Promise.all([
        api.getDocument(documentId),
        api.getDocumentFaqs(documentId),
        api.getRecentReads()
      ]);
      setSelectedDocument(detail);
      setDocumentFaqs(faqs);
      setRecentReads(latestReads);
      applyDocumentToForm(detail);
      setView("documents");
    } catch (err) {
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
    try {
      const result = await api.askQuestion(questionText);
      setAnswer(result);
      setView("qa");
      await bootstrap(selectedDocument?.document_id);
    } catch (err) {
      setError(err instanceof Error ? err.message : "问答请求失败");
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
          <button className="primary" onClick={askQuestion}>
            发起问答
          </button>
        </section>
      </aside>

      <main className="main-panel">
        <header className="header">
          <div>
            <h2>可交付 MVP+</h2>
            <p>在文档、版本、问答和 Agent 留痕基础上，继续补齐了分类、标签与 FAQ 的管理能力。</p>
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
            <button className="ghost" onClick={logout}>
              退出登录
            </button>
          </div>
        </header>

        {error ? <div className="alert">{error}</div> : null}
        {loading ? <div className="loading">正在加载演示数据...</div> : null}

        {!loading && view === "dashboard" ? (
          <>
            <section className="dashboard-grid">
              <SummaryCard label="文档总量" value={dashboard.total_documents} />
              <SummaryCard label="已发布文档" value={dashboard.published_documents} />
              <SummaryCard label="累计问答" value={dashboard.total_questions} />
              <SummaryCard label="Agent 运行数" value={dashboard.total_agent_runs} />
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
                    <h4>当前正文</h4>
                    <pre>{selectedDocument.content}</pre>
                  </article>
                  <section className="version-list">
                    <h4>版本记录</h4>
                    {selectedDocument.versions.map((version) => (
                      <div className="version-item" key={version.version_id}>
                        <strong>{version.version_no}</strong>
                        <span>{version.change_note}</span>
                        <small>{new Date(version.created_at).toLocaleString("zh-CN")}</small>
                      </div>
                    ))}
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
          <section className="content-layout qa-layout">
            <div className="panel">
              <div className="panel-title">
                <h3>智能问答</h3>
                <span>RAG + 引用证据</span>
              </div>
              {answer ? (
                <>
                  <article className="article-card">
                    <h4>答案</h4>
                    <p>{answer.answer_text}</p>
                    <p>置信度：{answer.confidence_score}</p>
                  </article>
                  <section className="citation-list">
                    <h4>引用证据</h4>
                    {answer.citations.map((citation) => (
                      <div className="citation-item" key={citation.cite_order}>
                        <strong>{citation.document_title}</strong>
                        <span>{citation.version_no} · 得分 {citation.score}</span>
                        <p>{citation.snippet_text}</p>
                      </div>
                    ))}
                  </section>
                </>
              ) : (
                <div className="empty-state">左侧输入问题后可在此查看回答</div>
              )}
            </div>

            <div className="panel">
              <div className="panel-title">
                <h3>问答历史</h3>
                <span>{questionHistory.length} 条</span>
              </div>
              <div className="history-list">
                {questionHistory.map((item) => (
                  <div className="history-item" key={item.question_id}>
                    <strong>{item.question_text}</strong>
                    <p>{item.answer_preview}</p>
                    <small>{new Date(item.created_at).toLocaleString("zh-CN")}</small>
                  </div>
                ))}
              </div>
            </div>
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
                        <div className="inline-actions">
                          <button className="ghost" onClick={() => beginEditUser(item)}>
                            编辑
                          </button>
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
              {agentRuns.map((run) => (
                <div className="run-row" key={run.run_id}>
                  <div>
                    <strong>{run.agent_type}</strong>
                    <p>{run.trigger_type} · {run.status}</p>
                  </div>
                  <div>
                    <p>{run.input_text}</p>
                    <small>{run.output_text}</small>
                  </div>
                  <small>{new Date(run.started_at).toLocaleString("zh-CN")}</small>
                </div>
              ))}
            </div>
          </section>
        ) : null}
      </main>
    </div>
  );
}

function SummaryCard(props: { label: string; value: number }) {
  return (
    <div className="summary-card">
      <span>{props.label}</span>
      <strong>{props.value}</strong>
    </div>
  );
}
