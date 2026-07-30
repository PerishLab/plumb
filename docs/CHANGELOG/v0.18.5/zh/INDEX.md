# Plumb v0.18.5

## Rust 仓库的 guard 必须走一遍 release profile

`structure.guard-checks-release-profile` 已 mechanized。带着 `Cargo.toml` 与 guard
wrapper 的仓库,必须在 guard 里的某处以 `--release` 调用 cargo。

它堵的是**一个没人编译的 profile**。guard 通常跑 `cargo fmt`、`cargo clippy --all-targets`
与 `cargo test`——全是 debug。于是只在 `debug_assertions` 下能通过类型检查的代码会一路绿灯发出去,
直到第一次 release 构建才炸;而对一个库来说,那第一次发生在**下游消费者的打包步骤**里,
不在它自己的仓库里。库的 release lane 也堵不住:它发的是 crate,而 `cargo publish` 的校验同样是 debug。

Keel v0.10.0 与 v0.10.1 无法以 release 构建,正是这个原因——一个 `debug_assert!` 的参数
调用了带 `#[cfg(debug_assertions)]` 的方法,而该宏在每个 profile 下都做类型检查、方法却只存在于一个。
两个版本发出去之后,才被下游的 Dockerfile 发现。

这条法要的是**那个 profile**,不是一次完整构建。`cargo check --release` 即满足,
而这正是该缺陷类所需:它们是不同 `cfg` 下的类型错误,check 几秒钟,build 要几分钟。

## 它对仓库的要求

Rust 仓库的 guard 里若没有 `--release` 调用,升级后即 out of true。往 guard 加一步即可settle。
没有 `Cargo.toml` 的仓库不受任何要求。
