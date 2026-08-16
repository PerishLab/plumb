# Plumb v0.21.0

## doctor 开始读发布声明

doctor 从来没有解析过 `[release]`。它对着裸 TOML 只看两个键，还把解析失败吞掉，
于是一份已经无法解析的声明，在三次守卫、两个 PR、一次落地全程报
`true to the skeleton`，只在发布当天才炸。

两条规则现在用「每个发布动词都在用的那个严格 spec」去读它。`release.spec-declared`
离开散文成为机制化：声明要么整份解析通过，要么 doctor 点名拒绝理由。
`release.attachment-deliverable` 答后一半——一个已声明的附件必须 resolve 到一条
交付它、且本仓库确实调用了的共享 lane。`publishes` 因此报的是声明面，
而不是从目录名拼出来的猜测。

## 门问声明，永不问环境

`ship <attachment> publish` 经 `PLUMB_RELEASE_OUTPUT` 向每一次发布索要一份已编译的
capsule。而只声明附件的发布永远设不出这个变量：没有东西为它编译 capsule，
也没有权威存放它。这道门拒绝了一种 plumb 自己定义的产品形状，拒在写 registry 的
前一步、在所有便宜的检查都已通过之后。

一次发布带不带封印是一条声明事实，门现在问 `Spec::binary()`。image 与 chart 附件
同样声明它们据以认证的 forge `account`——账号是公开名字，由 lane 提供就等于 lane
持有了产品的决定。`PLUMB_RELEASE_REGISTRY_ACCOUNT` 已移除。

## 拒绝不再指向无关的东西

投影此前在参数位置解析凭据，排在「没有这个附件就早返回」之前，于是
`ship oci publish` 会因为没设账号而拒绝一个根本没有镜像的产品。

`plumb land` 在别处犯同一个形状的错。forge 列出的是全部 pull 而不只是开着的，
而落地分支在 PR 合并后被保留，于是只按 ref 匹配会返回一个早已关闭的 PR，
落地就去等一个永远不会再跑的守卫。同一条主题分支的第二次落地必然挂死。

## 模块 adaptor 改说 pnpm

只有 pnpm 会在打包时把 `catalog:` 与 `workspace:` 解析成具体版本；npm 把它们原样
抄进已发布的 manifest，而工作区之外没有任何消费者解析得了。它同时改为在一个自有
座位上发布 `pack` 产出的那个归档——发出去的字节就是被校验过的字节，
构建从跑两遍变成一遍。

## 已经发生过的投影不再重来

chart 与 image 此前可以静默替换一个已发布的版本，而 module 干脆整个拒绝重跑。
三者现在都先问 registry 已经持有什么：chart 逐字节比，image 比 plumb 打在它身上的
payload，module 比 registry 只收一次的那个 identity。image 比 payload 是因为推送会
重写 image config，没有任何摘要能穿过那趟往返。

## lane 承接每一个已声明的投影

`ship oci`、`ship chart`、`ship npm` 在全域零调用点。共享二进制 lane 现在在封印前
准备它们、在封印后投影它们。

`packages/plumb` 定下本域一个已发布模块的形状，`charts/plumb` 承接一个最小工作
负载。image 与 chart 这一版都还没有被声明：`account` 字段随本版落地，
而一份声明它的 manifest 只能被下一版读取。
