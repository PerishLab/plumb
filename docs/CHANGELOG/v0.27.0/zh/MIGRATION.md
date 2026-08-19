# 迁移到 Plumb v0.27.0

## 在下一次 stable 发布之前，重新渲染 lane

**由 v0.26.0 渲染出来的 lane 只移动 manager，从不推进共识指针。** 走它发一次 stable，会把
所有对象都发出去、报告"已激活"，而通道仍指着上一个版本;smoke 因为装到仍然 canonical 的
那一版而失败，**而这次发布没法从操作者座位上收尾** —— 写指针需要的凭据只在 lane 内部(#346)。

**用这一版跑 `plumb lane --write` 并落地，然后再发下一次 stable。** exact 发布不受影响。
已经被劈成两半的发布，靠重新渲染并对同一个 stable 版本再派发一次即可修复:publish 是
create-only 且自校验的，重跑只会补上指针。

## `plumb stable` 现在是 `plumb release`

**每个动词都搬了家，参数一个没变**:`prepare`、`pick`、`freeze`、`rejoin`、`retract` 现在都是
`plumb release` 的动词，而 `plumb release stamp` 造一个发布点。**`stable` 仍然是正当的词** ——
通道、latest、指针都还这么叫 —— 所以它**不进退休词典**，用到它不会 out of true。搬的只是命令，
而且没有任何 lane 调用这些动词。

## `plumb stable packport` 现在是 `plumb release rejoin`

`packport` 已进退休词典，所以任何仍带着它的**被跟踪字节**，在装上这一版之后即 out of true ——
在这个仓库，也在这次发布到达的每个仓库。改掉调用，也改掉它周围的散文。共享发布 lane 会探测
新旧两种拼写，所以尚未迁移的仓库照样跑得起来。

## stable 发布改为先声明、后运行

`freeze` 不再打 stable 点。用 `plumb release stamp --version <stable>` 打;在它立到线头之前，
stable 派发会拒绝。exact 发布用同一个动词，取代 `git tag`。

## 干跑需要网络和凭据

操作者动词的 `--dry-run` 现在**做完所有读、只跳过写**，于是它印的是「对着远端此刻的样子将会
发生什么」。它需要和真实运行同样的凭据，并在无法核实时**拒绝**而不是印出计划。
`plumb retire --dry-run` 未变。

## agent 与 design 的文档预算改为随源伸缩

原本写死 320 行，现在量自己所描述的源，落在 240 到 800 的走廊里。**小仓库降到 240，
一份此前合格的 `AGENTS.md` 可能因此 out of true。**

## npm 重发会比对 tarball

从前发布一个已经立在那儿的版本，是**凭名字**跳过的。现在它把 `dist.integrity` 与刚造出来的
归档比对，不符即拒为 `published module drift`。内容未变的重跑照常通过。

## 介质要等它那次发布的 seal 读得回来才投影

对带二进制的产品，`plumb ship cargo|npm|oci|chart publish` 会先从公网 authority 把发布 seal
读回来。**在 lane 里什么都不变**;在二进制发布尚未发出去时手工跑其中一个，现在会拒绝。

## 两件不需要你做什么的

封印的 inputs 多了 `oci` 对象，所以这一版之后的第一次发布会把每个对象**重新计时一次**。
逐介质的宽度表被一个 `ceiling = 10` 取代，**原本被允许的没有一样变成拒绝**。
