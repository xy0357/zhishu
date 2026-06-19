# 知枢

企业知识资产管理与智能检索平台的首版可交付工程。

## 目录结构

```text
.
├── backend/                # Rust + Axum 后端
├── frontend/               # React + TypeScript 前端
├── infra/                  # docker-compose 与环境模板
├── docs/                   # 接口与实施文档
├── 开发计划.md
├── PROJECT_BRIEF.md
├── PAGE_MAP.md
├── DATA_MAP.md
├── CHANGELOG.md
└── 知枢_企业知识资产管理与智能检索平台_闭环落地版_修订稿.docx
```

## 当前交付范围

- 文档、版本、问答、引用证据、Agent 运行记录的数据模型
- 用户目录、阅读记录、收藏记录的演示业务链路
- 分类、标签、FAQ 的后台管理链路
- 基础登录鉴权、当前用户上下文与演示角色
- 角色概览、用户创建、用户编辑与动态登录演示链路
- 带盐迭代密码哈希与动态签发访问令牌
- MySQL 初始化脚本
- Rust 后端骨架与带文件持久化的 MVP API
- React 前端骨架、演示页面与文档编辑面板
- Docker Compose 基础依赖编排

## 启动说明

### 推荐启动路径

- 如果只是本地快速演示，优先使用默认 `file` 模式，不需要先启动 Docker、MySQL、Redis、Qdrant、MinIO。
- 如果要验证真实数据库链路，再启动 `infra/docker-compose.yml` 中的依赖，并切换到 `mysql` 模式。
- 前端默认运行在 `http://localhost:5173`
- 后端默认运行在 `http://127.0.0.1:8080`
- 前端默认接口基址为 `http://localhost:8080/api`，即使不创建 `.env.development` 也可直接联调。

### 1. 最小可运行方式：file 模式

先启动后端：

```powershell
.\infra\scripts\start-demo-backend.ps1
```

或使用等价命令：

```powershell
cd backend
cargo run
```

再启动前端：

```powershell
cd frontend
npm.cmd install
npm.cmd run dev
```

说明：

- 默认存储模式是 `file`，演示数据会写入 `backend/data/demo-store.json`。
- `file` 模式下不依赖 MySQL，因此不必先执行 `docker compose up -d`。
- 当前仓库已自带 `frontend/package-lock.json`，首次启动执行一次 `npm.cmd install` 即可。
- 若本机尚未安装 Rust 或 Node.js，可先执行环境检查脚本。

### 2. 完整依赖检查

```powershell
.\infra\scripts\check-prerequisites.ps1
```

注意：

- 该脚本会同时检查 `cargo`、`npm`、`docker`、`mysql`。
- 因此它更适合检查“完整依赖链路”是否齐全，而不是 `file` 模式的必跑前置步骤。
- 如果你只跑默认演示链路，只要 `cargo` 和 `npm` 可用即可。

### 2.1 路线 B：不用 Docker Desktop 的检查方式

如果你已经选择“不安装 Docker Desktop / 不使用 WSL”，请改用下面这条检查脚本：

```powershell
.\infra\scripts\check-route-b-prerequisites.ps1
```

说明：

- 该脚本只强制检查 `cargo`、`npm`、`mysql` 与本机 `MySQL:3306`。
- `Redis / Qdrant / MinIO` 在当前工程里仍属于增强依赖，会展示端口或健康状态，但不会因为缺少 `docker` 而直接判失败。
- 当前仓库的文件上传链路仍可先使用 `OBJECT_STORAGE_DIR` 本地对象目录镜像，不会因为真实 MinIO 未接入而阻塞 MySQL 主链路。
- 如果后续继续走路线 B，推荐口径是：
  - `MySQL`：本机安装并加入 `PATH`
  - `Redis`：使用 Windows 兼容实现，例如 `Memurai`
  - `Qdrant`：使用外部服务或后续再接入
  - `MinIO`：优先保持本地对象目录镜像，待找到可用的社区版 S3 兼容服务后再切换

路线 B 的一键验收入口：

```powershell
.\infra\scripts\run-route-b-acceptance.ps1
```

说明：

- 该脚本会依次执行 `check-route-b-prerequisites.ps1`、`run-mysql-acceptance.ps1`、`run-full-acceptance.ps1`
- 该脚本还会执行 `check-object-storage-mirror.ps1`
- 如果检测到 Qdrant 可达，它还会自动执行 demo points 导出、同步和检查
- 如果你在当前终端设置了 `QDRANT_URL` 与 `QDRANT_API_KEY`，它也会优先使用这组 Cloud 配置执行 Qdrant demo 验证
- 默认要求后端已经通过 `start-mysql-backend.ps1` 启动

如果你还想单独核对“文件元数据 -> 本地对象镜像”是否一致，可以执行：

```powershell
.\infra\scripts\check-object-storage-mirror.ps1
```

### 2.2 Qdrant demo 同步脚本

如果你后续拉起了 Qdrant，可先用下面两条脚本打通“文档分段 -> Qdrant collection / points”演示链路：

```powershell
$env:QDRANT_URL="https://your-cluster.cloud.qdrant.io"
$env:QDRANT_API_KEY="your-api-key"
.\infra\scripts\export-qdrant-demo-points.ps1
.\infra\scripts\sync-qdrant-demo.ps1
.\infra\scripts\run-qdrant-demo-check.ps1
.\infra\scripts\search-qdrant-demo.ps1
```

说明：

- `export-qdrant-demo-points.ps1` 会登录后端、读取文档分段，并导出 `artifacts/qdrant-demo-points.json`
- `sync-qdrant-demo.ps1` 会根据导出文件创建 `zhishu_segments_demo` collection，并 upsert points；支持通过 `QDRANT_API_KEY` 访问 Cloud 集群
- `run-qdrant-demo-check.ps1` 会读取 `zhishu_segments_demo` collection 状态，验证向量维度与 points 数量
- `search-qdrant-demo.ps1` 会对问题文本生成同口径 `demo_hash_v1` 向量，并调用 Qdrant `Query API`
- 当前向量是 `demo_hash_v1`，只用于打通 Qdrant 同步与演示链路，不代表真实 Embedding 质量
- 不要把真实 `API key` 写进仓库文件，统一通过环境变量传入

### 3. MySQL 模式

先启动基础依赖：

```powershell
cd infra
docker compose up -d
```

然后启动后端：

```powershell
.\infra\scripts\start-mysql-backend.ps1
```

或使用等价命令：

```powershell
$env:APP_STORAGE_BACKEND='mysql'
$env:MYSQL_URL='mysql://zhishu:zhishu@127.0.0.1:3306/zhishu'
cd backend
cargo run
```

说明：

- `start-mysql-backend.ps1` 会设置 `APP_STORAGE_BACKEND=mysql` 与默认 `MYSQL_URL`。
- MySQL 模式启动时会自动执行 `backend/migrations/` 下尚未执行的迁移，无需手工先跑 SQL。
- `backend/.env.example` 提供了完整示例环境变量，其中 `APP_ACCESS_TOKEN_SECRET` 建议在非演示环境中自行覆盖。
- `infra/.env.example` 提供了 MySQL、Redis、Qdrant、MinIO 的默认连接参数参考。
- 当前仓库已实现 MySQL 仓储代码并通过 `cargo check` 与单元测试，但本机尚未完成真实 MySQL 运行验收。

如果你走的是路线 B，则不必先执行 `docker compose up -d`。只要本机 MySQL 已可连接，就可以直接启动：

```powershell
.\infra\scripts\start-mysql-backend.ps1
```

然后前端照常启动：

```powershell
cd frontend
npm.cmd run dev
```

### 4. 演示账号与权限

- `backend/migrations/001_init.sql` 已包含默认种子数据，供当前 MVP 逻辑直接使用。
- 当前演示账号：
  - `admin / Admin@123456`
  - `editor / Editor@123456`
- 管理员可新建/编辑用户，新增用户会立即写入文件仓储或 MySQL，并可直接使用新密码登录。
- 登录成功后返回的是动态签发的访问令牌，不再依赖固定 demo token。
- 当前角色边界：
  - `admin`：可访问用户目录、分类/标签管理、Agent 记录、文档与 FAQ 管理
  - `editor`：可访问文档与 FAQ 管理、问答、收藏、阅读；不可访问用户目录、分类/标签管理、Agent 记录

## 首版已实现的接口

- `POST /api/auth/login`
- `GET /api/auth/me`
- `GET /api/health`
- `GET /api/dashboard`
- `GET /api/categories`
- `POST /api/categories`
- `PUT /api/categories/:name`
- `DELETE /api/categories/:name`
- `GET /api/tags`
- `POST /api/tags`
- `PUT /api/tags/:name`
- `DELETE /api/tags/:name`
- `GET /api/users`
- `POST /api/users`
- `PUT /api/users/:id`
- `GET /api/roles`
- `GET /api/favorites`
- `GET /api/read-records/recent`
- `GET /api/documents`
- `POST /api/documents`
- `GET /api/documents/:id`
- `GET /api/documents/:id/faqs`
- `POST /api/documents/:id/faqs`
- `PUT /api/faqs/:id`
- `DELETE /api/faqs/:id`
- `POST /api/documents/:id/read`
- `POST /api/documents/:id/favorite`
- `PUT /api/documents/:id`
- `GET /api/documents/:id/versions`
- `POST /api/documents/:id/publish`
- `POST /api/documents/:id/archive`
- `POST /api/qa/ask`
- `GET /api/questions/history`
- `GET /api/agent-runs`

## 说明

- 当前版本优先保证“可演示、可扩展、与 docx 方案一致”。
- 后端默认用文件持久化的轻量仓储提供完整演示链路，因此即使未配置 MySQL 也能跑通页面与核心流程。
- 后端已具备 `file/mysql` 双仓储结构，下一步可以直接切入真实 MySQL。
- 后端已新增内存仓储闭环测试，覆盖文档创建、更新生成版本、发布、归档、问答历史与持久化回读。
- 后端已新增路由级集成测试，覆盖分类、标签、FAQ 管理接口的真实 Router 调用。
- 后端已新增鉴权路由测试，覆盖登录与当前用户接口。
- 后端已新增角色权限测试，验证 editor 对 admin 能力返回 `403`，同时保留内容管理能力。
- 后端已新增用户管理测试，验证管理员创建/编辑用户后，新账号可立即登录。
- 后端已新增安全测试，覆盖密码哈希校验与签名访问令牌生成。
- 当前前端已覆盖登录、Dashboard、文档中心、智能问答、用户与行为、配置管理、Agent 记录 7 个一级视图。
- 当前前端已在“用户与行为”视图补齐角色概览、用户创建与用户编辑表单。
- 当前前端已新增“配置管理”视图，可直接维护分类、标签与 FAQ。
- 当前前端会按角色隐藏不允许访问的后台入口。
- 数据库、向量、对象存储的生产化接入点已经在目录与脚本中预留。
