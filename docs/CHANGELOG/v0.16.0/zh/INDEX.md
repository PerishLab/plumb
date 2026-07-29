# plumb v0.16.0

## standing 改为可查询，不再靠誊抄

编进二进制的规则目录（rule catalog）现在是规则身份、standing、解释、证据与
归属的**唯一**来源。`plumb rule list` 与 `plumb rule show <id>` 从中读取；
`plumb doctor --json` 另行报告整个目录的覆盖率，与本仓库的 findings 分开。

skill 的 `Standing` 一节此前是三份手工维护的清单，这让「声称已强制而实际未
强制」成为长期风险——正是那份文档自己说「唯一承担不起」的缺陷。现在它指向
目录本身。namespace、tag、owner、standing 都是编进同一个二进制的封闭词汇表，
未知的选择器会**拒绝**，而不是返回一个可能被误当作结论的空结果。

`blind` 是 finding 的等级而非一种 standing：它表示机检器跑了但读不到所需证据。
prose-only 的规则不会仅仅因为没有评估器就变成 blind。见 `docs/rules.md`。

## 骨架自己的散文也进了检查器

`ectropy.toml` 现在扫描 `skills/**/*.md` 并把 `skills/*` 当作模块根，于是
plumb 交付的那份 brief 与交付它的代码受同一套语法法约束。

## `plumb changelog` 不再靠猜来解释拒绝

传入空的或只有空白的 `--version` 时，该命令会回答
「the repository declares none, so pass --version」——这是一句关于仓库的断言，
而它**根本没读过**仓库。现在空白旗标等同于未传，改由仓库自己声明的版本作答。

这个缺陷是在把变更日志门控推广到其余六个 manager 时暴露的。`stim` 把
`inputs.version_override` 直接接到 `RELEASE_VERSION`，而该输入是可选的，
所以操作者一旦忘记填，旗标就是空的。拒绝本身是对的，给出的理由是编的。
