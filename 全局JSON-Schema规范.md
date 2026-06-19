# 全局 JSON Schema 规范

## 1. 文档目的

这份文档用于统一本项目所有 JSON 请求体、响应体和内部对象结构的定义方式。

说白了，它解决三个问题：

- 前后端字段名要不要统一
- 哪些字段必填，哪些字段可空
- 文档、问答、分类、标签、引用证据这些对象到底应该怎么写才规范

本规范适用于：

- 后端接口请求体
- 后端接口响应体
- 前端 TypeScript 类型
- 交付文档中的 JSON 示例
- 后续如果补 OpenAPI / JSON Schema 文件时的统一口径

---

## 2. 总体原则

### 2.1 字段命名统一使用 `snake_case`

本项目现有前后端模型已经统一使用下划线命名，因此后续继续保持：

- 正确：`document_id`
- 正确：`question_text`
- 正确：`created_at`
- 错误：`documentId`
- 错误：`questionText`

### 2.2 JSON 根结构优先统一为响应包装对象

除极少数特殊场景外，接口响应统一使用：

```json
{
  "success": true,
  "message": "document detail",
  "data": {}
}
```

对应当前项目中的统一响应模型：

```json
{
  "success": "布尔值，表示请求是否成功",
  "message": "字符串，表示结果说明",
  "data": "实际业务数据"
}
```

### 2.3 一个字段只表达一个意思

不要让一个字段既承担显示含义，又承担状态含义。

例如：

- `status` 只表示状态
- `title` 只表示标题
- `summary` 只表示摘要
- `created_at` 只表示创建时间

不要出现这种不清楚的字段：

- `info`
- `extra`
- `value`
- `content_data`

### 2.4 概念层字段名优先稳定，不随界面文案变化

比如：

- 用 `category_name`，不要因为页面改成“分类标题”就改成 `category_title`
- 用 `question_text`，不要因为展示时写成“问题内容”就改字段名

---

## 3. JSON Schema 编写约定

### 3.1 每个对象都应明确以下内容

每个 Schema 至少应说明：

- `type`
- `properties`
- `required`
- `additionalProperties`

推荐写法：

```json
{
  "type": "object",
  "properties": {
    "document_id": { "type": "integer" },
    "title": { "type": "string" }
  },
  "required": ["document_id", "title"],
  "additionalProperties": false
}
```

### 3.2 默认禁止未声明字段

除非明确需要扩展字段，否则对象统一建议：

```json
{
  "additionalProperties": false
}
```

原因很简单：

- 防止前端乱传字段
- 防止后端偷偷多返回不稳定字段
- 防止不同接口同名对象结构漂移

### 3.3 主键字段统一使用整数

像下面这些编号字段，统一用整数：

- `document_id`
- `version_id`
- `question_id`
- `answer_id`
- `faq_id`
- `user_id`
- `run_id`

JSON Schema 推荐写法：

```json
{
  "type": "integer",
  "minimum": 1
}
```

### 3.4 文本字段统一使用字符串

例如：

- `title`
- `summary`
- `content`
- `question_text`
- `answer_text`
- `description`

如果需要进一步限制，可加：

- `minLength`
- `maxLength`
- `pattern`

---

## 4. 必填、可选、可空规则

### 4.1 必填和可空不是一回事

- 必填：字段必须出现
- 可空：字段允许值为 `null`

例如：

```json
{
  "type": "object",
  "properties": {
    "password": {
      "type": ["string", "null"]
    }
  },
  "required": ["password"]
}
```

这表示 `password` 必须出现，但值可以是 `null`。

### 4.2 本项目默认策略

默认推荐：

- 能不为空就不要设成可空
- 能省略就不要强行传 `null`
- 请求体中的可选字段，优先“省略字段”，其次才是 `null`

举例：

- `UserUpdateRequest.password` 适合可选
- `DocumentDetail.title` 不适合可空
- `CategoryItem.description` 如果项目允许空描述，更推荐空字符串而不是 `null`

### 4.3 推荐规则

- 标识字段：必填，不可空
- 状态字段：必填，不可空
- 时间字段：必填，不可空
- 业务正文：通常必填，不可空
- 更新密码这类字段：可选

---

## 5. 时间字段规范

### 5.1 统一使用 ISO 8601 字符串

本项目当前后端使用 `chrono::DateTime<Utc>`，前端读取为字符串，因此时间字段统一为：

- `created_at`
- `updated_at`
- `started_at`
- `favorite_time`
- `read_time`

JSON Schema 推荐写法：

```json
{
  "type": "string",
  "format": "date-time"
}
```

### 5.2 时间一律使用 UTC 语义

接口层统一按标准时间字符串表达，不在 JSON 里混入本地时区业务文案。

正确示例：

```json
"created_at": "2026-06-08T08:30:00Z"
```

不推荐：

```json
"created_at": "2026年6月8日 16:30"
```

---

## 6. 枚举与状态字段规范

### 6.1 状态字段要尽量收敛为枚举

像下面这些字段，后续都应优先收敛成有限集合：

- `status`
- `agent_type`
- `trigger_type`
- `token_type`
- `role_name`

### 6.2 Schema 中推荐显式使用 `enum`

示例：

```json
{
  "type": "string",
  "enum": ["draft", "published", "archived"]
}
```

如果当前项目还没完全定死枚举值，也建议至少在文档里先约束语义。

---

## 7. 数组字段规范

### 7.1 数组必须声明元素类型

例如 `tags`：

```json
{
  "type": "array",
  "items": {
    "type": "string"
  },
  "default": []
}
```

### 7.2 空集合优先用 `[]`

像这些字段，如果没有数据，统一返回空数组，而不是 `null`：

- `tags`
- `versions`
- `citations`

原因：

- 前端更好处理
- 类型更稳定
- 不用到处判空

### 7.3 数组顺序要有含义

如果数组有顺序语义，要在文档说明。

例如：

- `versions`：通常按版本时间倒序或版本号倒序
- `citations`：按 `cite_order` 顺序

---

## 8. 分页对象规范

本项目当前很多接口还是直接返回列表，但如果后续需要分页，统一建议使用下面结构：

```json
{
  "success": true,
  "message": "documents",
  "data": {
    "items": [],
    "page": 1,
    "page_size": 20,
    "total": 100
  }
}
```

分页对象建议 Schema：

```json
{
  "type": "object",
  "properties": {
    "items": { "type": "array" },
    "page": { "type": "integer", "minimum": 1 },
    "page_size": { "type": "integer", "minimum": 1 },
    "total": { "type": "integer", "minimum": 0 }
  },
  "required": ["items", "page", "page_size", "total"],
  "additionalProperties": false
}
```

---

## 9. 错误响应规范

### 9.1 建议错误响应也保持统一包装结构

推荐形式：

```json
{
  "success": false,
  "message": "unauthorized",
  "data": null
}
```

### 9.2 如果后续需要更详细错误信息

建议扩展为：

```json
{
  "success": false,
  "message": "validation failed",
  "data": null,
  "error": {
    "code": "VALIDATION_ERROR",
    "details": [
      {
        "field": "title",
        "reason": "must not be empty"
      }
    ]
  }
}
```

但如果引入 `error` 对象，必须全局统一，不能有些接口有、有些接口没有。

---

## 10. 本项目核心对象 Schema 约定

## 10.1 通用响应包装

```json
{
  "$id": "ApiResponse",
  "type": "object",
  "properties": {
    "success": { "type": "boolean" },
    "message": { "type": "string" },
    "data": {}
  },
  "required": ["success", "message", "data"],
  "additionalProperties": false
}
```

## 10.2 文档列表项

```json
{
  "$id": "DocumentListItem",
  "type": "object",
  "properties": {
    "document_id": { "type": "integer", "minimum": 1 },
    "title": { "type": "string", "minLength": 1 },
    "summary": { "type": "string" },
    "category_name": { "type": "string", "minLength": 1 },
    "status": { "type": "string" },
    "version_no": { "type": "string" },
    "is_favorite": { "type": "boolean" },
    "updated_at": { "type": "string", "format": "date-time" }
  },
  "required": [
    "document_id",
    "title",
    "summary",
    "category_name",
    "status",
    "version_no",
    "is_favorite",
    "updated_at"
  ],
  "additionalProperties": false
}
```

## 10.3 文档详情

```json
{
  "$id": "DocumentDetail",
  "type": "object",
  "properties": {
    "document_id": { "type": "integer", "minimum": 1 },
    "title": { "type": "string", "minLength": 1 },
    "summary": { "type": "string" },
    "content": { "type": "string", "minLength": 1 },
    "category_name": { "type": "string", "minLength": 1 },
    "status": { "type": "string" },
    "version_no": { "type": "string" },
    "is_favorite": { "type": "boolean" },
    "tags": {
      "type": "array",
      "items": { "type": "string" },
      "default": []
    },
    "versions": {
      "type": "array",
      "items": { "$ref": "DocumentVersion" },
      "default": []
    }
  },
  "required": [
    "document_id",
    "title",
    "summary",
    "content",
    "category_name",
    "status",
    "version_no",
    "is_favorite",
    "tags",
    "versions"
  ],
  "additionalProperties": false
}
```

## 10.4 文档创建/更新请求

```json
{
  "$id": "DocumentFormPayload",
  "type": "object",
  "properties": {
    "title": { "type": "string", "minLength": 1 },
    "summary": { "type": "string" },
    "content": { "type": "string", "minLength": 1 },
    "category_name": { "type": "string", "minLength": 1 },
    "tags": {
      "type": "array",
      "items": { "type": "string" },
      "default": []
    },
    "change_note": { "type": "string" }
  },
  "required": ["title", "summary", "content", "category_name", "tags", "change_note"],
  "additionalProperties": false
}
```

## 10.5 引用证据

```json
{
  "$id": "Citation",
  "type": "object",
  "properties": {
    "cite_order": { "type": "integer", "minimum": 1 },
    "document_title": { "type": "string" },
    "version_no": { "type": "string" },
    "snippet_text": { "type": "string" },
    "score": { "type": "number", "minimum": 0 }
  },
  "required": ["cite_order", "document_title", "version_no", "snippet_text", "score"],
  "additionalProperties": false
}
```

## 10.6 问答结果

```json
{
  "$id": "QaAnswer",
  "type": "object",
  "properties": {
    "answer_id": { "type": "integer", "minimum": 1 },
    "answer_text": { "type": "string", "minLength": 1 },
    "confidence_score": { "type": "number", "minimum": 0, "maximum": 1 },
    "citations": {
      "type": "array",
      "items": { "$ref": "Citation" },
      "default": []
    },
    "created_at": { "type": "string", "format": "date-time" }
  },
  "required": ["answer_id", "answer_text", "confidence_score", "citations", "created_at"],
  "additionalProperties": false
}
```

## 10.7 分类与标签

```json
{
  "$id": "CategoryItem",
  "type": "object",
  "properties": {
    "category_name": { "type": "string", "minLength": 1 },
    "description": { "type": "string" },
    "document_count": { "type": "integer", "minimum": 0 }
  },
  "required": ["category_name", "description", "document_count"],
  "additionalProperties": false
}
```

```json
{
  "$id": "TagItem",
  "type": "object",
  "properties": {
    "tag_name": { "type": "string", "minLength": 1 },
    "description": { "type": "string" },
    "document_count": { "type": "integer", "minimum": 0 }
  },
  "required": ["tag_name", "description", "document_count"],
  "additionalProperties": false
}
```

## 10.8 FAQ

```json
{
  "$id": "FaqItem",
  "type": "object",
  "properties": {
    "faq_id": { "type": "integer", "minimum": 1 },
    "document_id": { "type": "integer", "minimum": 1 },
    "question": { "type": "string", "minLength": 1 },
    "answer": { "type": "string", "minLength": 1 },
    "created_at": { "type": "string", "format": "date-time" }
  },
  "required": ["faq_id", "document_id", "question", "answer", "created_at"],
  "additionalProperties": false
}
```

## 10.9 用户与登录

```json
{
  "$id": "UserItem",
  "type": "object",
  "properties": {
    "user_id": { "type": "integer", "minimum": 1 },
    "username": { "type": "string", "minLength": 1 },
    "role_name": { "type": "string", "minLength": 1 },
    "department": { "type": "string" },
    "email": { "type": "string", "format": "email" }
  },
  "required": ["user_id", "username", "role_name", "department", "email"],
  "additionalProperties": false
}
```

```json
{
  "$id": "LoginPayload",
  "type": "object",
  "properties": {
    "username": { "type": "string", "minLength": 1 },
    "password": { "type": "string", "minLength": 1 }
  },
  "required": ["username", "password"],
  "additionalProperties": false
}
```

---

## 11. 本项目字段级统一建议

### 11.1 编号字段统一

- 主键统一：`xxx_id`
- 版本号统一：`version_no`
- 数量统一：`xxx_count`

### 11.2 布尔字段统一

布尔字段优先使用能直接读懂的问题式命名：

- `is_favorite`
- `success`

不要写成：

- `favorite_flag`
- `success_flag`

### 11.3 文本类字段统一

- 标题：`title`
- 摘要：`summary`
- 正文：`content`
- 问题正文：`question_text`
- 回答正文：`answer_text`
- 变更说明：`change_note`
- 描述说明：`description`

### 11.4 时间字段统一

- 创建时间：`created_at`
- 更新时间：`updated_at`
- 开始时间：`started_at`
- 阅读时间：`read_time`
- 收藏时间：`favorite_time`

---

## 12. 不推荐的写法

下面这些写法后续应避免：

### 12.1 同一语义多种命名并存

例如：

- 一会儿 `question`
- 一会儿 `question_text`

如果对象表示完整问题实体，建议固定一种语义：

- FAQ 场景可用 `question`
- 问答输入场景可用 `question_text`

但同一类对象内部不要混用。

### 12.2 空数组写成 `null`

不推荐：

```json
{
  "tags": null
}
```

推荐：

```json
{
  "tags": []
}
```

### 12.3 状态值自由发挥

不推荐不同接口各写各的：

- `done`
- `success`
- `finished`
- `ok`

状态值应统一成一套枚举。

---

## 13. 落地建议

这份规范后续建议这样使用：

1. 后端新增请求/响应结构时，先按本规范命名字段。
2. 前端 `types.ts` 里的类型定义，与后端结构保持一一对应。
3. 如果以后补 `OpenAPI`，把这里的对象直接转成正式 Schema。
4. 如果新增接口返回格式和这里冲突，优先改新接口，不要继续扩散不一致写法。

---

## 14. 一句话总结

这个项目的 JSON Schema 规范，核心就三句话：

- 字段统一用 `snake_case`
- 响应统一用 `success + message + data`
- 结构尽量稳定、明确、可校验，不要让同一个对象在不同接口里长得不一样
