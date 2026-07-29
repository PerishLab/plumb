# 迁移到 v0.16.0

## 如果你从 skill 里读 standing

别再誊抄了。`plumb rule list --standing mechanized` 与 `--standing prose-only`
直接从与该二进制版本匹配的目录作答。把清单抄进散文，正是「声称」与「检查器
实际所做」发生漂移的路径。

注意 `clean` 的含义边界：它说的是本仓库没有产生 finding，**不是**说目录里每条
法都已机械化。整个目录的覆盖率由 `plumb doctor --json` 单独承载。

## 如果你在 `skills/` 下放散文

本仓库中它现在受 ectropy 扫描。别的仓库要照做，需要把 `skills/**/*.md` 加入
`scan.include`、把 `skills/*` 加入 `module.roots`；没有任何东西强制你这么做。

## 如果你的 lane 传的 `--version` 可能为空

无需动作。空白旗标现在会回退到仓库声明的版本，而不是带着错误解释拒绝。传入
真实版本的 lane 不受影响；完全没有声明版本的仓库仍会被拒绝——只是这次理由准确。

## 如果你有 `use-*.ts` 命名的 web hooks

按主题重命名——`use-health.ts` 改为 `health.ts`——并删掉你为该目录豁免单词法而
加的 ectropy boundary。导出的 hook 保持 React 拼写：`useHealth`。

没有任何东西强制你改名；现在的检查只拒绝非 `.ts` 或大写文件。但保留旧拼写的
仓库仍然需要那条 boundary，而本次发布正是要让这项豁免变得不必要。
