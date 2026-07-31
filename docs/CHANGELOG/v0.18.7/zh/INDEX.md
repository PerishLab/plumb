# Plumb v0.18.7

## Guard 跟随仓库 authority

Doctor 现在识别两种受支持仓库 authority 各自的规范 guard seat：Forgejo 使用
`.forgejo/workflows/guard.yml`，GitHub 使用
`.github/workflows/quality.yml`。一个仓库只持有其中一个；两者同时存在会因自动化
authority 歧义而拒绝。

无论选择哪种 authority，workflow 都提供同一组 guard 证据：规范 concurrency、
Plumb 与 Ectropy 调用、Rust release profile 覆盖、滚动 CI container，以及受管
web 构建。过渡期 guard wrapper 仍可贡献证据，但不会取代 workflow 事实。

## Trace 与 span 检视

Plumb 现在在根 `locus.toml` 中，分别以 trace 与 span 分辨率声明相同的粗粒度
representation size 和 dominant prefix analyzer。它仅是只读检视声明，不能启用
audit collection，也不替 operator 选择 report。
