# Plumb v0.18.27

## 一套 Forgejo 方言

Plumb 现在依赖已发布的 Runseal library，并调用与
`runseal :perish @forgejo` 相同的结构化 Forgejo 资源 verb。Plumb 继续持有 Git
操作、发布拓扑、重试、轮询与结果解释；它不再携带第二套已鉴权 Forgejo HTTP client。

原 `vendor/forgejo` 实现与 `tea.yml` token parser 已删除。Forgejo authority 通过
`FORGEJO_URL` 与 `FORGEJO_TOKEN_FILE` 或 `FORGEJO_TOKEN` 显式进入。land、stable
准备与 packport、release dispatch 与 task 结果感知、recovery、retire 均使用
Runseal 0.16.3 的结构化调用。

Court fixture 现在覆盖共享 HTTP 路径，包括分支保护读回、可复用 workflow task
消歧、guard status 与 merge payload。
