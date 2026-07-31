# Plumb v0.18.6

## 仓库事实是 workflow

受管仓库现在只需保有一条规范 guard workflow，不再被要求复制通用 wrapper 或
仓库自有 Git hook。workflow 本身证明 guard 会运行 Plumb 与 Ectropy、走过 Rust
release profile，并在存在受管 web package 时完成构建。

过渡期及产品专属 wrapper 仍会被 Doctor 观察，也仍须拥有已知职责；但 wrapper
不存在已经是合法形状。通用 init、land 与 release 行为属于拥有它的底层入口，
不应以复制文件的方式散落在每个仓库。

## 严格 typed cascade

Cascade derive 新增顶层 `#[cascade(strict)]` 与嵌套 section 的
`#[cascade(section, strict)]`。严格 file partial 会拒绝未知字段，不再静默接受
拼错或已经废弃的 profile key。

严格性保持显式；现有 cascade 在产品词汇准备好关闭文件边界之前不会改变行为。
