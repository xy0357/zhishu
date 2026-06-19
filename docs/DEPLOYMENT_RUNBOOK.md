# 知枢部署运行手册

## 0. 两种部署路线

- 路线 A：使用 `Docker Desktop + docker compose`
- 路线 B：不用 Docker Desktop，直接使用本机或外部依赖服务

当前如果你选择路线 B，请优先保证：

- `MySQL` 已本机安装并可通过 `mysql --version` 访问
- 后端可连接 `127.0.0.1:3306`
- 文件上传先继续使用 `OBJECT_STORAGE_DIR` 本地对象目录镜像
- `Redis / Qdrant / MinIO` 暂按“可选增强依赖”处理，后续再逐步替换为真实服务

## 1. 启动基础依赖
```powershell
cd E:\zhishu\zhishu1
powershell -ExecutionPolicy Bypass -File infra\scripts\start-full-infra.ps1
```

说明：

- 这一节只适用于路线 A。
- 如果你选择路线 B，请跳过本节，改用 [infra/scripts/check-route-b-prerequisites.ps1](/E:/zhishu/zhishu1/infra/scripts/check-route-b-prerequisites.ps1:1) 做本机检查。

## 2. 启动后端
```powershell
cd E:\zhishu\zhishu1
powershell -ExecutionPolicy Bypass -File infra\scripts\start-mysql-backend.ps1
```

## 3. 启动前端
```powershell
cd E:\zhishu\zhishu1\frontend
npm run dev
```

## 4. 运行验收
```powershell
cd E:\zhishu\zhishu1
powershell -ExecutionPolicy Bypass -File infra\scripts\run-full-acceptance.ps1
```

## 5. 默认地址
- 前端：`http://localhost:5173`
- 后端：`http://127.0.0.1:8080`
- API：`http://127.0.0.1:8080/api`
- Qdrant：`http://127.0.0.1:6333`
- MinIO：`http://127.0.0.1:9000`
- MinIO Console：`http://127.0.0.1:9001`

## 6. 当前限制
- Redis、Qdrant、MinIO 目前主要完成启动脚本和配置占位，生产级真实 SDK 接入仍需继续推进。
- 向量化当前已完成分段与最小检索闭环，后续需接真实 Embedding 与 Qdrant 写入。

## 7. 路线 B 建议口径

- `MySQL`：本机安装，继续作为当前最主要的真实依赖
- `Redis`：Windows 环境建议使用兼容实现，例如 `Memurai`
- `Qdrant`：建议使用外部服务地址，或等后续有容器环境后再切回本机
- `MinIO`：当前项目仍先走 `OBJECT_STORAGE_DIR` 本地对象目录镜像，暂不强制接真实 S3 服务

推荐检查命令：

```powershell
cd E:\zhishu\zhishu1
powershell -ExecutionPolicy Bypass -File infra\scripts\check-route-b-prerequisites.ps1
```

推荐验收命令：

```powershell
cd E:\zhishu\zhishu1
powershell -ExecutionPolicy Bypass -File infra\scripts\run-route-b-acceptance.ps1
```

该脚本默认还会检查：

- `document_files` 与 `OBJECT_STORAGE_DIR` 本地对象镜像是否一致
- 若 Qdrant 当前可达，则自动执行 demo points 导出、同步与 collection 校验
- 若当前终端已设置 `QDRANT_URL` 与 `QDRANT_API_KEY`，则会优先使用这组 Cloud 配置执行 Qdrant demo 验证

如果要单独核对文件上传落地情况：

```powershell
cd E:\zhishu\zhishu1
powershell -ExecutionPolicy Bypass -File infra\scripts\check-object-storage-mirror.ps1
```

如果已具备可用的 Qdrant 服务，可继续执行：

```powershell
cd E:\zhishu\zhishu1
$env:QDRANT_URL="https://your-cluster.cloud.qdrant.io"
$env:QDRANT_API_KEY="your-api-key"
powershell -ExecutionPolicy Bypass -File infra\scripts\export-qdrant-demo-points.ps1
powershell -ExecutionPolicy Bypass -File infra\scripts\sync-qdrant-demo.ps1
powershell -ExecutionPolicy Bypass -File infra\scripts\run-qdrant-demo-check.ps1
powershell -ExecutionPolicy Bypass -File infra\scripts\search-qdrant-demo.ps1
```

这两条脚本会以 `demo_hash_v1` 向量模式创建 `zhishu_segments_demo` collection，并把当前文档分段同步进去。
如使用 Qdrant Cloud，请不要把 `API key` 直接写入脚本文件，统一通过 `QDRANT_API_KEY` 环境变量提供。
