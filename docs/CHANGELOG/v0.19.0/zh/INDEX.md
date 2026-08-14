# Plumb v0.19.0

## ref 承载发布

发布 lane 不再接受 channel 与 version 两个输入，改为读 `github.ref`：
`refs/heads/release/vX.Y.Z` 携带 stable 版本，`refs/tags/vX.Y.Z-<channel>.N`
携带 exact 版本，其余一律拒绝。channel 再由 `plumb release channel` 派生，
这条规则只住在那一处。

`plumb ship binary dispatch` 只剩 `--version`。它派生 channel、派生承载它的 ref，
不发送任何身份输入。`--channel` 与 `--ref` 一并移除——exact 发布绑定任意分支
正是 tag 锚点取消掉的自由。

两个 caller 仍由操作者派发。身份从敲出来的字符串变成选出来的 ref，
且没有任何发布跟随推送而发生，因此不会有东西被意外发布。

这同时补掉了 lane 内部的第二个源：resolve 冻结了 version 与 commit，
而 build 与 coordinate 各自重读 channel 输入；现在每个 job 都读冻结后的
resolve 输出。

## 缺失的分支被创建而不是被拒绝

`plumb stable prepare` 在新建发布线时以 "The target couldn't be found." 失败。
存在性探测按已退休的 vendored client 的错误措辞判定分支缺失，而 Runseal
的措辞不含其中任何一个。分支与保护规则现在都按读取结果选择：读得到就编辑，
读不到就创建，真正的失败由需要它的那个操作自己上抛。没有文本跨越产品边界。

## 这一版尚未定下的 `ship`

`ship` 在这里只带 `binary` 一个 adaptor。adaptor 集合在本版中尚未枚举，
`site` 仍以自己的命令应答，两者都在此版之后才落地。请把搬走的九个动词
读作变更的全部，而不是一个已成形对象的形状。

`release` 仍保留 `activate`、`inspect` 与 `registry`。它们或横跨两个对象、
或独自站着，都是过渡态而非定局，会在认领它们的那个 adaptor 被吸收时移动。
迁移这一版搬走的，不要围绕这一版没搬的接线。
