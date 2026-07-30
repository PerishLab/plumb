# Plumb v0.18.4

## 锁定的 JSR specifier 按范围匹配，不按文本

依赖读取把 `deno.json` 里写的 requirement 原样拼成查找键，再与 lockfile 的 specifier
逐字比较。Deno 并不原样写回：在零主版本内它把 caret 规范化为 tilde，于是 `^0.3.1`
被记作 `~0.3.1`。键永远对不上，该依赖报告零个解析结果——而 v0.18.3 起 blind 会让退出码
非零，于是整个 doctor 在证据明明齐备且正确的情况下失败。

凡是用 caret 声明 first-party JSR 依赖的仓库都这样不可读，而这个 blind 还顺带遮住了
依赖法条对它的其余判断，包括 v0.18.3 刚开始执行的陈旧检查。

逐字匹配仍是第一顺位——lockfile 里同一个包携带两个 range 时，要靠精确键把它们分开。
只有在它一无所获时，查找才会读取该包的全部 specifier，并保留所声明 requirement 允许的解析。

## 继承而来的 Locus 观测，默认静默

Plumb 拥有自己 CLI 观测的含义，Locus 拥有它们的上下文、采集、生成与上报。
运行时策略经由一个有类型的 `PLUMB_LOCUS_*` 段跨越这条边界；其总闸默认 `false`，
并在 Locus bootstrap 之前就返回——因此已配置的 collector 或 reporter 可以被继承着放在那里，
而不会凭空长出一条观测路径。契约由 skill 的 `references/audit.md` 命名。

## 本次未改变的

法条、standing 与声明面均未移动。此前 true to the skeleton 的仓库依旧如此；
此前在这条上 blind 的仓库，现在会读出它真正的 findings —— 对一个陈旧的 first-party
依赖而言，那是一条它先前根本看不见的 finding。
