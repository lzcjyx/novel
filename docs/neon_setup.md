# Neon PostgreSQL + pgvector 设置指南

## 1. 创建 Neon 项目

1. 访问 [neon.tech](https://neon.tech) 并注册/登录。
2. 点击 **Create project**。
3. 项目名称：`novel-factory`。
4. Region：选择离你 n8n 服务器最近的区域（推荐 `ap-northeast-1` 对应 Asia/Tokyo）。
5. 点击 **Create project**。

## 2. 创建数据库

在 Neon 控制台中：

1. 进入 **SQL Editor**。
2. 运行：
   ```sql
   CREATE DATABASE novel_factory;
   ```
   或者点击 **Databases** → **Create database** → 输入 `novel_factory`。

## 3. 获取连接字符串

进入 Neon Dashboard → 你的 project → **Connection Details**：

| 连接类型 | Host 格式 | 用途 |
|---------|----------|------|
| Pooled | `ep-xxx-pooler.region.aws.neon.tech` | n8n 日常读写 |
| Direct | `ep-xxx.region.aws.neon.tech` | SQL migration / 管理 |

- **Pooled connection string**（用于 `.env` 的 `NEON_DATABASE_URL_POOLED`）：
  ```
  postgresql://USER:PASSWORD@ep-xxx-pooler.region.aws.neon.tech/novel_factory?sslmode=require
  ```

- **Direct connection string**（用于 `.env` 的 `NEON_DATABASE_URL_DIRECT`）：
  ```
  postgresql://USER:PASSWORD@ep-xxx.region.aws.neon.tech/novel_factory?sslmode=require
  ```

> ⚠️ 两个连接字符串的 Host 不同。Pooled 的 host 包含 `-pooler`。

## 4. 运行 SQL Migration

**重要：Migration 必须使用 Direct Connection。**

```bash
# 设置环境变量
export NEON_DATABASE_URL_DIRECT="postgresql://USER:PASSWORD@ep-xxx.region.aws.neon.tech/novel_factory?sslmode=require"

# 依次执行
psql "$NEON_DATABASE_URL_DIRECT" -f sql/000_neon_setup.sql
psql "$NEON_DATABASE_URL_DIRECT" -f sql/001_init_schema.sql
psql "$NEON_DATABASE_URL_DIRECT" -f sql/002_pgvector_indexes.sql
```

所有 migration 文件都是幂等的（使用 `IF NOT EXISTS` / `ON CONFLICT DO NOTHING`），可以安全重复执行。

## 5. 在 n8n 中创建 Postgres Credential

1. 进入 n8n → **Credentials** → **Add Credential**。
2. 选择 **Postgres**。
3. 填写：
   | 字段 | 值 |
   |------|-----|
   | Host | `ep-xxx-pooler.region.aws.neon.tech`（Pooled host） |
   | Port | `5432` |
   | Database | `novel_factory` |
   | User | Neon 控制台中的 User |
   | Password | Neon 控制台中的 Password |
   | SSL | **Require** |

4. Credential 名称：`Neon Pooled (n8n)`。

> ⚠️ n8n 日常操作使用 Pooled connection。Migration 和索引操作使用 Direct connection（命令行）。

## 6. 连接用途区分

| 操作 | 连接类型 | 说明 |
|------|---------|------|
| n8n 查询章节/角色/设定 | Pooled | 通过 n8n Postgres 节点 |
| n8n 向量检索 | Pooled | 通过 n8n Postgres 节点 |
| SQL migration | Direct | `psql` 命令行 |
| 创建索引 | Direct | `psql` 命令行 |
| 数据库备份 | Direct | `pg_dump` 命令行 |
| 数据库管理 | Direct | `psql` 命令行 |

## 7. 重要约束

1. **不使用 session-level advisory lock**。Neon pooled connection 使用 transaction pooling，session 不持久。
2. **幂等控制使用唯一约束 + `INSERT ... ON CONFLICT DO NOTHING RETURNING id`**。
3. **所有连接必须 SSL Require**。
4. **不要在 n8n workflow 中硬编码连接字符串**——使用 n8n credentials。

## 8. Embedding 维度说明

- 默认 `EMBEDDING_DIMENSION=1536`（OpenAI `text-embedding-3-small`）。
- `vector_documents.embedding` 列定义为 `vector(1536)`。
- 如果更换 embedding 模型：
  1. 更新 `.env` 中的 `EMBEDDING_DIMENSION`。
  2. 修改表定义：`ALTER TABLE vector_documents ALTER COLUMN embedding TYPE vector(NEW_DIM);`
  3. 删除旧索引：`DROP INDEX idx_vector_documents_embedding_hnsw;`
  4. 重新创建索引：参考 `sql/002_pgvector_indexes.sql`。
  5. 重新生成所有向量文档的 embedding。
