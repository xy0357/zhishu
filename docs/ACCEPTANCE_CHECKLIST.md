# 知枢验收清单

## 基础环境
- 路线 A：`infra/scripts/check-prerequisites.ps1` 通过
- 路线 A：`infra/scripts/start-full-infra.ps1` 已启动 MySQL / Redis / Qdrant / MinIO
- 路线 B：`infra/scripts/check-route-b-prerequisites.ps1` 通过
- 路线 B：本机 `MySQL:3306` 可连接，`mysql --version` 可执行
- `infra/scripts/start-mysql-backend.ps1` 已启动后端
- `frontend` 下 `npm run dev` 已启动前端

## 核心链路
- 登录成功，`/api/auth/refresh` 可刷新令牌
- 文档可创建、更新、发布、归档
- 原始文件可上传、绑定、下载
- 发布后自动生成 `document_segments`
- `/api/documents/{id}/segments` 可读取分段
- 问答引用可返回真实 `segment_id`
- 管理员可创建用户、重置密码、删除用户
- 管理员可执行 `/api/documents/{id}/reindex`

## 脚本验收
- `infra/scripts/run-mysql-acceptance.ps1`
- `infra/scripts/run-full-acceptance.ps1`
- `infra/scripts/run-route-b-acceptance.ps1`
- `infra/scripts/check-object-storage-mirror.ps1`
- 如 Qdrant 可用：`infra/scripts/export-qdrant-demo-points.ps1` + `infra/scripts/sync-qdrant-demo.ps1` + `infra/scripts/run-qdrant-demo-check.ps1` + `infra/scripts/search-qdrant-demo.ps1`

## 路线 B 说明
- 在无 Docker 场景下，`Redis / Qdrant / MinIO` 当前允许继续保留为增强依赖，不阻塞 MySQL 主链路验收
- 当前文件上传与下载仍可依赖 `OBJECT_STORAGE_DIR` 本地对象目录镜像完成验收
- `run-full-acceptance.ps1` 已可验证刷新令牌、文档发布、分段生成、重建分段、问答真实片段引用、用户重置与删除
- `run-route-b-acceptance.ps1` 当前还会自动补做对象镜像一致性检查，并在 Qdrant 可达时自动执行 demo 向量同步链路
- 如果路线 B 使用的是 Qdrant Cloud，则先在当前终端设置 `QDRANT_URL` 与 `QDRANT_API_KEY`，再执行 `run-route-b-acceptance.ps1`

## 交付前检查
- `backend` 下 `cargo test`
- `frontend` 下 `npm run build`
- 核对 `PROJECT_BRIEF.md`、`PAGE_MAP.md`、`DATA_MAP.md`、`CHANGELOG.md`、`开发计划.md`
