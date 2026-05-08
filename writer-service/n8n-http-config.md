# n8n HTTP Request 节点 — writer-service 配置说明

## 1. Credential 准备

### 1.1 Writer Service Token (HTTP Header Auth)

| 字段 | 值 |
|------|-----|
| Credential Name | `Writer Service Token` |
| Name | `Authorization` |
| Value | `Bearer {{$env.WRITER_SERVICE_TOKEN}}` |

### 1.2 环境变量

在 n8n 中设置以下环境变量：

| 变量 | 值 |
|------|-----|
| `WRITER_SERVICE_URL` | `http://writer-service:8787`（Docker）或 `http://localhost:8787`（本地） |
| `WRITER_SERVICE_TOKEN` | 与 writer-service `.env` 中一致 |

## 2. POST /generate-chapter

### 节点配置

| 参数 | 值 |
|------|-----|
| Method | `POST` |
| URL | `{{$env.WRITER_SERVICE_URL}}/generate-chapter` |
| Authentication | Generic Credential Type → `Writer Service Token` |
| Send Body | `true` |
| Specify Body | `JSON` |

### JSON Body

```json
{
  "job_id": "{{ $json.job_id }}",
  "project_id": "{{ $json.project_id }}",
  "chapter_plan_id": "{{ $json.chapter_plan_id }}",
  "writing_brief": {{ JSON.stringify($json.writing_brief) }},
  "prompt_type": "draft_writer"
}
```

> ⚠️ `writing_brief` 字段必须使用 `JSON.stringify()` 而非 `{{ }}` 模板语法，
> 否则包含换行和特殊字符的 JSON 会破坏请求体。

### Options

| 选项 | 值 | 说明 |
|------|-----|------|
| Timeout | `600000` ms (10 min) | Claude Code 生成一章可能耗时数分钟 |
| On Error | Continue (使用 error branch) | 允许下游处理失败 |

### 成功响应

```json
{
  "ok": true,
  "data": {
    "title": "第X章 标题",
    "body_markdown": "...",
    "summary": "...",
    "word_count": 3100
  },
  "stderr": "",
  "duration_ms": 45200
}
```

### 失败响应

```json
{
  "ok": false,
  "error": "Claude Code exited with code 1",
  "stderr": "...",
  "exitCode": 1,
  "duration_ms": 12000,
  "raw_stdout": "..."
}
```

### 下游处理

```
Call Writer Service
    ↓
Writer OK? (IF node)
    ├─ true  → Save Chapter
    └─ false → Handle Writer Error → Record Failure
```

IF 条件：`{{ $json.ok }}` equals `true`

## 3. POST /revise-chapter

### 节点配置

与 `/generate-chapter` 相同，替换 URL 和 body：

| 参数 | 值 |
|------|-----|
| URL | `{{$env.WRITER_SERVICE_URL}}/revise-chapter` |

### JSON Body

```json
{
  "job_id": "{{ $json.job_id }}",
  "project_id": "{{ $json.project_id }}",
  "writing_brief": {{ JSON.stringify($json.writing_brief) }},
  "prompt_type": "revision_writer"
}
```

> `writing_brief` 必须包含 `initial_draft`（原稿）和 `review_reports`（审稿报告），
> 这些已由 `Build Revision Input` 节点打包。

## 4. GET /health

监控用，不需要 auth。

| 参数 | 值 |
|------|-----|
| Method | `GET` |
| URL | `{{$env.WRITER_SERVICE_URL}}/health` |

响应：
```json
{
  "ok": true,
  "service": "novel-writer-service",
  "version": "1.0.0",
  "uptime_s": 3600,
  "active_requests": 1,
  "max_concurrent": 2
}
```

## 5. 故障排查

### 5.1 401 Unauthorized

- 检查 `WRITER_SERVICE_TOKEN` 在 writer-service 和 n8n 中是否一致。
- 检查 Header 格式：`Authorization: Bearer <token>`。

### 5.2 503 Max concurrency

- 增加 `WRITER_SERVICE_MAX_CONCURRENT`。
- 或等待当前任务完成。

### 5.3 Timeout (600s)

- 检查 Claude Code 是否正常运行：`claude --version`。
- 增加 `CLAUDE_CODE_TIMEOUT_MS`。
- 减少 `CLAUDE_CODE_MAX_TURNS`。

### 5.4 Claude Code not found

- 确认 Claude Code CLI 已安装：`npm install -g @anthropic-ai/claude-code`。
- 确认 `ANTHROPIC_API_KEY` 已设置。
- 使用 `GET /ready` 检查服务状态。

### 5.5 响应 JSON 解析失败

writer-service 会尝试从 Claude 输出中提取 JSON：
1. 先尝试直接 `JSON.parse(stdout)`。
2. 失败后尝试正则提取 `{...}`。
3. 两者都失败则返回 `ok: false`。

常见原因：
- Claude 输出了额外的解释文本（已用 `--bare` 抑制）。
- prompt 中没有明确要求只输出 JSON。
- `--output-format json` 未被 Claude 正确解析。
