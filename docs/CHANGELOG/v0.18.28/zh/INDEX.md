# Plumb v0.18.28

## 一套 Cloudflare 方言

Plumb 现在通过 Runseal 0.17.0 执行 Cloudflare 控制面操作。site inspect 使用其
token verify、Worker service 与 Worker domain verb；retire 使用其 account token
factory，以及 R2 bucket/custom domain verb。

原 `vendor::cloudflare` HTTP client 已删除。Plumb 继续拥有 site 与 retire 编排、
资源解释、破坏性确认，以及临时 token 必定撤销的生命周期。

mint 出来的 token 值在执行有边界的 R2 操作并撤销之前，始终保留在 Runseal
禁止明文调试、退出时清零的 secret result 中。Worker 或 bucket 不存在等依赖
status 的行为使用结构化 Cloudflare fault，不再解析错误文本。
