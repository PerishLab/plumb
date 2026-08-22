# Plumb v0.30.0

## 外部轮廓冻结，内部可以重建

本版本冻结四个表面：`plumb` 库的公开 API、17 个顶层命令及其参数、121
个规则标识，以及渲染到 `ectropy.toml` 的形状。此后 command、shape 与
judge 的内部实现可以重建，而无需让下游仓库跟随内部移动。

## 发布声明会在发布日之前开口

Doctor 现在使用所有发布动词共用的严格读取器解析 `[release]`。坏掉的声明会
被判为 `out of true`，包括被附件子表意外吞入的键。声明了附件却没有任何
可交付它的 lane，也会在本地被点名。

当 Plumb 审查自身仓库时，如果已发布二进制内嵌的构建提交是新树的祖先，
Doctor 会判 `blind`。开发构建与普通产品仓不会作出这项推断。

## Stable 祖先关系读取最新远端真相

`release prepare` 与 `release freeze` 会先 fetch origin，再判断上一条 stable
是否已经进入 main。陈旧的 `origin/main` 不再能让下一条 stable 线假通过。
rejoin 仍是显式的人工 line 动作；Plumb 自动化的是门，不是合并判断。

## path 预算变为 3

Ectropy 默认 path 上限从 4 降为 3，与 fanout 10 配对。旧 Plumb 命令树中的
五个文件暂由窄且有名的 boundary 承担，直到 closure rewrite 删除它们。
depot 发布窗口同时接受编译值与携带值，发布完成后收敛到新值。

## 渲染出的 guard 会先采纳 depot

每条渲染 guard 都会先同步 Plumb depot，再询问本次需要重复哪些工作。规则、
分类法、policy 常量与条件、lane 模板和帮助资源因此能作为一个不可变 depot
版本共同移动。
