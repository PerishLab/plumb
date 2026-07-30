# 迁移到 v0.17.1

把仓库迁移到受支持的 Sealkit `^0.2.1` 版本线之前，先安装 Plumb v0.17.1。
仓库不需要规避写法：保留声明中的 caret，让标准 Deno install 命令写出它的
规范化锁键。

不要把 lock 中的 `~0.2.1` 手工改回 `^0.2.1`。Lockfile 属于 Deno；本版本让
Plumb 学会读取 Deno 的表示。
