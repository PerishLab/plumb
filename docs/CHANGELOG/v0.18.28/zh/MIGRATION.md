# 迁移到 Plumb v0.18.28

命令和配置名均不改变。site 操作继续提供 `PLUMB_SITE_*`；只有经过显式确认的
retire 才读取 `PLUMB_RETIRE_*`。

构建 `plumb` crate `vendor` feature 的消费方必须解析到精确 Runseal 0.17.0。
私有的 `plumb::vendor::cloudflare` 模块已经移除；原子 Cloudflare 操作请直接调用
`runseal::tool::cloudflare`。

本地契约测试可以使用 loopback HTTP Cloudflare endpoint；所有非 loopback site
endpoint 仍然必须使用 HTTPS。
