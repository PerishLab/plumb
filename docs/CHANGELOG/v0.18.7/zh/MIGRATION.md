# 迁移到 Plumb v0.18.7

已经使用 `.forgejo/workflows/guard.yml` 的 Forgejo 仓库无需移动 workflow。

GitHub 仓库以 `.github/workflows/quality.yml` 作为规范 guard seat，并应在该
workflow 中加入共享 concurrency block：

```yaml
concurrency:
  group: guard-${{ github.event.pull_request.number || github.ref }}
  cancel-in-progress: true
```

如果 Forgejo guard 与 GitHub quality workflow 同时存在，应移除已经失效的第二个
规范 seat。所选 guard seat 之外的产品专属 workflow 不受影响。

无需迁移 runtime audit 配置。已有 Plumb audit report 的 operator 可以显式检视：

```sh
locus inspect . < /path/to/plumb-audit.jsonl
```

检视不会启用 collection，也不会修改 report。
