# 迁移到 Plumb v0.18.6

受管仓库需要一条 `.forgejo/workflows/guard.yml`。该 lane 必须直接调用
`plumb doctor` 与 Ectropy；Rust 仓库必须走过 release profile，受管 web 仓库
必须构建具名 web package。

当 workflow 已承载这些证据后，可以移除通用 guard、init、land wrapper 与仓库
自有 Git hook。产品专属或过渡期 wrapper 可以继续存在，但仍须具备已知职责。

配置消费者可以选择关闭文件词汇：

```rust
#[derive(Cascade)]
#[cascade(strict)]
struct Profile {
    #[cascade(section)]
    isolation: Isolation,
}

#[derive(Cascade)]
#[cascade(section, strict)]
struct Isolation {
    root: PathBuf,
}
```

仅在未知字段应该失败的边界启用 strict；未标注的 cascade 行为不变。
