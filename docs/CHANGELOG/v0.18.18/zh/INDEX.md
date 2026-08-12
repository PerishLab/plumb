# Plumb v0.18.18

## 退役一条交付链

`plumb retire` 销毁一条已声明的交付链。它是发布的镜像：被它移除的每一样东西，
都是 Plumb 自己声明、发布或保护过的。

产品通过在 `plumb.toml` 的 `[release.retire]` 下写明发布 bucket 与 Cloudflare
zone 来选择加入。自定义域名由已声明的发布权威推导而来，因此不再重复声明。没有
声明的产品无法被退役。

干跑是默认行为，不读取任何凭据。只有当 `--confirm-repo`、`--confirm-bucket`、
`--confirm-domain` 各自逐字等于其目标时，`--execute` 才会动手。销毁顺序固定：
清点、归档并清除凭据、撤销写入令牌、摘除域名、清空并删除 bucket、删除仓库、
移除本地 escrow。

凭据清除针对的是 Plumb 自己写入的 secret 名称，并把已删除的数量与本就不存在的
数量分开报告，因此这一行无法宣称一次它没有执行的清除。

## 一次性权威

退役自行铸造受限令牌，而不是持有常驻令牌。一个短时账号令牌与一个 bucket 条目
令牌从 `PLUMB_RETIRE_ACCOUNT`、`PLUMB_RETIRE_TOKEN`、`PLUMB_RETIRE_API` 声明的
工厂中切出，带有效期，并在一个无论清扫成功与否都会执行的分支里撤销。两个权限组
都在任何令牌被切出之前解析完毕，因此没有任何失败路径会遗留已铸令牌。派生令牌
永不进入命令参数或日志。

## 重画后的冷启动法条

旧法条称 Plumb 既不创建也不改写外部资源。这一点早已不成立：Plumb 写分支保护、
切发布分支、开启并合并 pull、向 R2 写入对象、部署 Cloudflare worker。

Plumb 现在声明：它拥有受管发布面的完整生命，退役包含在内；并且可以为处置它所
治理的对象派生一次性权威。冷启动禁令得以保留，并改写为实现约束：Plumb 不供给
任何尚不存在的资源，而其保证在于此处根本没有写下任何供给调用。

## vendor 层

对外服务层此前叫 `forge`，但它当时已经导出一个通用 HTTP 发送器，而且只与
Forgejo 通话。现在它是 `vendor`，Forgejo 客户端位于 `vendor::forgejo`，
Cloudflare 客户端位于 `vendor::cloudflare`。

共享发送器此前把 Forgejo 的认证方案写死，这正是 site 通道携带一份重复 HTTP
实现的原因。它现在接收完整的头部值。Site 的绑定查询、令牌校验与 worker 查找
折入共享客户端；公开指纹回读保留独立的 fetch，因为该探针读取 HTML，不能声明
JSON `Accept` 头。

`vendor` Cargo feature 取代 `forge`。没有任何已发布的消费者启用过旧的那个。
