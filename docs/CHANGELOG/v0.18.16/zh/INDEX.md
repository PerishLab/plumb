# Plumb v0.18.16

## 退役词典载入首个词条

过渡性的退役词典此前只发布了机制，词条集合为空，因此
`vocabulary.retired-term-absent` 没有主体，扫描在读取任何仓库之前就返回。
词典现在携带一个编码原子，该法条对每个受管仓库正式生效。

当该词出现在受管仓库的 tracked 路径名，或某个 tracked 文件的当前工作树字节中
时，仓库即为 out of true。唯一豁免是 `docs/CHANGELOG`。匹配大小写不敏感且不设
词边界，因此一句声明"本仓库不使用该退役产品"的散文同样携带该词，与其他命中
一视同仁。

## Doctor fixture 现在是仓库

启用词典让 vocabulary 路径第一次真正执行，并暴露出测试 fixture 建模的是"不是
Git 仓库的受管仓库"。Doctor 对这些 fixture 报 blind 是正确的：它的证据是
tracked 路径，而非仓库无法提供这种证据。所有运行 Doctor 的 fixture 现在都是
仓库。法条本身未作改动。
