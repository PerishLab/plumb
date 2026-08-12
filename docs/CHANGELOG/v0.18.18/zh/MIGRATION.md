# 迁移到 Plumb v0.18.18

现有仓库无需迁移。`[release.retire]` 是可选的，没有它的 manifest 解析行为不变。

应当可被退役的产品声明两个不可再推导的事实：

```toml
[release.retire]
bucket = "perish-<product>-releases"
zone = "<32 位小写十六进制 zone id>"
```

不声明是安全的默认值，意味着该产品完全无法被这条命令退役。自定义域名由发布权威
推导，不要重复声明。

退役需要环境中存在一个 Cloudflare 令牌工厂，它在 Plumb 之外供给：

```sh
PLUMB_RETIRE_ACCOUNT=<account id>
PLUMB_RETIRE_TOKEN=<factory token>
```

`PLUMB_RETIRE_API` 默认指向 Cloudflare v4 端点。Plumb 消费这份凭据，从不铸造
它；供给它仍然是资源所有者的工作。

动手之前先读干跑。它不读取凭据，也不改变任何状态：

```sh
plumb retire
```

启用过 `forge` Cargo feature 的库消费者改为启用 `vendor`，并从
`plumb::vendor::forgejo` 而非 `plumb::forge` 取用 Forgejo 客户端。没有任何已发布
的消费者启用过 `forge`，因此预计这不会影响 Plumb 自身之外的任何仓库。
