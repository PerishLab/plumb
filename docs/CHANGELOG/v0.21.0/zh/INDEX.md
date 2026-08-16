# Plumb v0.21.0

## doctor 开始读发布声明

doctor 从来没有解析过 `[release]`。它对着裸 TOML 只看两个键，还把解析失败吞掉，
于是**一份已经无法解析的声明，在三次守卫、两个 PR、一次落地全程报 `true to the skeleton`**，
只在发布当天才炸。

两条规则现在用「每个发布动词都在用的那个严格 spec」去读它。`release.spec-declared`
离开散文成为机制化：声明要么整份解析通过，要么 doctor 点名拒绝理由。
`release.attachment-deliverable` 答后一半——一个已声明的附件必须 resolve 到一条
**交付它、且本仓库确实调用了的**共享 lane。

`publishes` 因此报的是声明面，而不是从目录名拼出来的猜测。一个有 `packages/*`
却没有声明模块附件的仓库，不再被报成发 npm。

## 门问声明，永不问环境

`ship <attachment> publish` 经 `PLUMB_RELEASE_OUTPUT` 向每一次发布索要一份已编译的
capsule。而只声明附件的发布**永远设不出这个变量**：没有东西为它编译 capsule，也没有
权威存放它。这道门拒绝了一种 plumb 自己定义的产品形状，拒在写 registry 的前一步、
在所有便宜的检查都已通过之后。

**一次发布带不带封印，是一条声明事实。** 门现在问 `Spec::binary()`——从早已算过它的
校验里提出来，两处共用一个定义。声明了产品与权威的发布，约束与从前一字不差；
只声明附件的发布，由接收它的 registry 来约束。

image 与 chart 附件现在声明它们据以认证的 forge `account`。账号是公开名字，
由 lane 提供就等于 lane 持有了产品的决定；凭据仍留在环境里。
`PLUMB_RELEASE_REGISTRY_ACCOUNT` 已移除。

## 拒绝不再指向无关的东西

投影此前在**参数位置**解析凭据，排在「没有这个附件就早返回」之前，于是
`ship oci publish` 会因为没设账号而拒绝一个根本没有镜像的产品。现在每个 adaptor
都在确认自己有东西可投影之后才读凭据。

`plumb land` 在另一个界面上犯同一个形状的错。forge 列出的是**全部** pull 而不只是
开着的，而落地分支在 PR 合并后被保留，于是只按 ref 匹配会返回一个早已关闭的 PR，
落地就去等一个永远不会再跑的守卫。**同一条主题分支的第二次落地必然挂死。**
而干跑一直在打印 `GET /pulls?state=open`——代码里从来没有那个过滤。

## 模块 adaptor 改说 pnpm

只有 pnpm 会在打包时把 `catalog:` 与 `workspace:` 解析成具体版本。npm 把它们原样
抄进已发布的 manifest，而工作区之外没有任何消费者解析得了。

它同时改为发布 `pack` 产出的那个归档，并在一个自有座位上发。**发出去的字节就是被
校验过的字节**，构建从跑两遍变成一遍，动词也不再要求一棵被自己刻意改写过的工作树
看起来干净。

## 已经发生过的投影不再重来

chart 与 image 此前可以**静默替换**一个已发布的版本，而 module 干脆整个拒绝重跑。
三者现在都先问 registry 已经持有什么：chart 拉回来逐字节比，image 比 plumb 打在它
身上的 payload，module 比 registry 只收一次的那个 identity。

image 比 payload 而不是 digest，是因为推送会重写 image config——本地构建的镜像与它
推上去再读回来的形态**不共享任何摘要**，而它由之构建的 payload 能穿过这一趟往返。

一次到达了部分介质的发布，现在靠重跑续上。

## lane 承接每一个已声明的投影

`ship oci`、`ship chart`、`ship npm` 在全域**零调用点**：三个 adaptor 应答、doctor
接受它们的附件表、没有任何 lane 交付。共享二进制 lane 现在在封印前准备它们、
在封印后投影它们，与它本来就承接的 Cargo 附件并列。

## 载体开始承接内容

`packages/plumb` 定下本域一个已发布模块的形状：`build`、`test`、`typecheck`、
`prepack`，每个工具都经工作区 catalog 钉版本。`charts/plumb` 承接一个集群会接纳的
最小工作负载。

image 与 chart 这一版**都还没有被声明**。`account` 字段随本版落地，而一份声明它的
manifest 只能被下一版读取——正如 adaptor 枚举先于它的投影落地。
