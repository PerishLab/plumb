# 迁移到 Plumb v0.18.5

Rust 仓库往 guard 加一步。任何以 `--release` 调用 cargo 的形式都满足这条法;
类型检查就够,而且正是该缺陷类所需:

```ts
{
  label: "cargo release",
  runs: [["cargo", ["check", "--locked", "--workspace", "--all-targets", "--release"]]],
}
```

预期升级后的第一次运行是**失败**而不是报出这条新 finding——如果该仓库从未编译过那个 profile,
这一步可能会翻出真正等在里面的错误。那正是它的用意:在那里把它们修掉,而不是把这一步删掉。

没有 `Cargo.toml` 的仓库不受影响,其余法条、声明与配置面均未变动。
