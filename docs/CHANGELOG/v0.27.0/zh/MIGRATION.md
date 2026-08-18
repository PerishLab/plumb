# 迁移到 Plumb v0.27.0

## 在下一次 stable 发布之前，重新渲染 lane

**由 v0.26.0 渲染出来的 lane 只移动 manager，从不推进共识指针。** 于是走它发一次 stable，
会把所有对象都发出去、报告"已激活"，而通道仍然指着上一个版本。smoke 因为装到的是仍然
canonical 的那一版而失败，**而这次发布没法从操作者座位上收尾** —— 写指针那个动作需要的凭据
只存在于 lane 内部(issue #346)。

**用这一版跑 `plumb lane --write` 并落地，然后再发下一次 stable。** exact 发布不受影响。
已经被劈成两半的发布，靠重新渲染并对同一个 stable 版本再派发一次即可修复:publish 是
create-only 且自校验的，所以重跑只会补上指针，别的什么都不动。

## `plumb stable packport` 现在是 `plumb stable rejoin`

**命令表面发生了破坏性变更。** `packport` 已进退休词典，所以任何仍带着这个词的**被跟踪字节**，
在装上这一版之后即 out of true —— 在这个仓库，也在这次发布到达的每个仓库。改掉调用，也改掉
它周围的散文。

共享发布 lane 会探测新旧两种拼写，所以尚未迁移的仓库照样跑得起来。全域迁完之后，那个探测撤掉。

## 干跑需要网络和凭据

操作者动词的 `--dry-run` —— `stable prepare|pick|freeze|rejoin|retract` 与
`ship binary dispatch` —— 现在**做完所有读、只跳过写**。它解析真实运行会解析的那些条件，
于是它印出来的是「对着远端此刻的样子将会发生什么」，而不是写在代码旁边的一个猜测。

后果有两条：它需要和真实运行**同样的凭据**；以及它在无法核实时**拒绝**，而不是印出一份
它没能核实的计划。对着一条不存在的线做预览，现在会直说；从前它会印出冻结这条线的计划。

`plumb retire --dry-run` 未变，仍然不读任何凭据。

## agent 与 design 的文档预算改为随源伸缩

原本是写死的 320 行 Markdown。现在它们量自己所描述的源，落在 240 到 800 的走廊里。**大仓库
得到余量；小仓库降到 240，一份此前合格的 `AGENTS.md` 可能因此 out of true。** 报告会点名它
实际施加的预算，所以那个数字不用猜。

## npm 重发会比对 tarball

从前，发布一个已经立在那儿的版本，是**凭名字**跳过的。现在它把 `dist.integrity` 与刚造出来的
归档比对，不符即拒为 `published module drift`。一次内容未变的重跑与从前一样通过；一次输入
已经动过的重跑会拒绝，而不是报告一个它没核实过的成功。

如果某条 lane 是**有意**在同一个版本下重发不同内容，那条 lane 本来就是错的，现在只是被告知了。

## 介质要等它那次发布的 seal 读得回来才投影

对于带二进制的产品，`plumb ship cargo|npm|oci|chart publish` 现在会在投影之前，从公网
authority 把发布 seal 读回来。**在 lane 里什么都不变**，因为投影的 job 本来就排在封印的
job 之后。在二进制发布尚未发出去时手工跑其中一个，现在会拒绝 —— 从前它凭一个本机 capsule
文件就往下走了。

## 封印的 inputs 多了 `oci` 对象

早先 Plumb 记下的 baseline 里没有它，所以这一版之后的第一次发布会把每个对象**重新计时一次**。
除此之外不产生任何后果。

## `plumb stable retract` 现在是 `plumb release retract`

点属于发布那一段，所以撤点的动词跟着搬过去。它现在**同时接受 exact 版本与 stable 版本**。

## stable 发布改为先声明、后运行

`plumb stable freeze` 不再打 stable 点。用 `plumb release stamp --version <stable>` 打；
在它立到线头之前，stable 派发会拒绝。exact 发布用同一个动词，取代 `git tag`。

## 附件宽度

逐介质的表被一个 `ceiling = 10` 取代，表里只留偏离它的。**原本被允许的，没有一样变成拒绝。**
