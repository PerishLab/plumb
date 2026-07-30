# 迁移到 Plumb v0.18.4

对仓库没有任何要求。法条、声明与配置面均未变动，本来干净的 doctor 依旧干净。

如果你的 doctor 对某个 first-party 包报出 `deno.lock` 的 blind——例如
`@perish/sealkit has 0 matching resolutions for jsr:@perish/sealkit@^0.3.1`——就升级。
在本次发布之前没有本地出路：requirement 法条把 tilde 判为 pin 而拒绝，
而 lock 查找又只找得到 Deno 写下的那个 tilde。现在两头都成立；
v0.18.3 要求的 first-party 无版本写法从来不受此影响。

预期本次发布是**让 findings 浮现**而不是消除它们。那个 blind 一直压着该包的依赖法条，
所以藏在它后面的陈旧解析会在升级后的第一次运行中现身——那正是 v0.18.3 本就打算报告的。
