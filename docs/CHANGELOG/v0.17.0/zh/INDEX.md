# Plumb v0.17.0

Plumb 现在把 cargo 家族视为仓库发布的东西。此前的分类只认识 binary 与 JSR 包，
于是一个发到 registry 的 Rust 库族既不声明什么也不对什么负责：没有 release lane
被要求，没有 operator wrapper 被核对，那套区分「部署站点」与「发布包」的词汇
也够不到它。

声明了 binaries 的 release 表发布 binary，声明了 cargo 的发布 crate。此前的判定
只问 release 键在不在，于是仅声明 cargo 的仓库会被读成 binary 发布者，被要求
它用不上的 lane。

Plumb 两种发布面都声明，因此按自己那条「发布 registry 包的仓库暴露一个 operator
wrapper」的法，补上了该有的 wrapper。它调用本地构建——与 guard 调用自己的 doctor
同一个道理：工具不能让自己的发布经过一个尚不存在的自身副本。

skill 现在写明了自己的管辖。它此前说自己治理「这里的每个仓库」，而读者无法
拿这句话去检验眼前的目录，于是带着这份 brief 的 agent 就把它带去了每个地方。
plumb 管辖根上有 `plumb.toml` 的仓库；在别处这些法沉默，而它们的沉默不值一提。

生成的 manager 现在可以直接接管旧格式的 stable 安装，不再要求操作者先卸载。
接管边界刻意收紧：必须同时证明 canonical stable authority、旧 root 与 version
marker，以及所有当前入口都属于同一个 version seat。自定义 authority、预发布渠道、
多义 seat 或已经漂移的入口仍会被拒绝。
