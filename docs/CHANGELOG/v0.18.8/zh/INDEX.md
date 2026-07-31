# Plumb v0.18.8

## 发布操作归 Plumb

`plumb release dispatch` 现在承接通用 Forgejo workflow dispatch。exact 发布
继续允许 operator 自选 channel 与分支；只有 stable 会派生
`release/vX.Y.Z`、要求 exact promotion，并接受更严格的分支生命周期。

`plumb stable prepare`、`pick`、`freeze`、`packport` 共同拥有这条生命周期。
分支保护会逐字段 readback。packport 保留 merge 拓扑，证明已发布 commit 已成为
`main` 的 ancestor，并永久保留冻结的 release 分支。发布身份不再要求新建 Git tag。

## Site 操作归 Plumb

`plumb site plan`、`inspect`、`deploy` 从仓库唯一的
`apps/*/wrangler.jsonc` 推导应用、package、产物目录与 worker。部署分别报告上传、
平台绑定和公网可达性；readback 必须包含刚刚构建的 index 所声明的 fingerprint。

Cloudflare authority 通过类型化 `PLUMB_SITE_*` 配置进入。token 只通过环境或 curl
配置输入传递，绝不进入命令参数。

## 产品仓不再持有通用控制面

Doctor 不再把 release 或 site wrapper 当作能力证据。Plumb 仓库自身改用 env-only
Runseal profile，也不再持有 release、ship 或 cold-start wrapper。

发布资源 authority 是已经 provision 的输入。建仓以及资源、域名、token、escrow
和 secret 同步继续属于各自控制面，不由 Plumb 推断，也不再复制进产品仓。
