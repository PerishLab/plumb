# Plumb v0.18.17

## 版本所属的发布产物

发布版本现在可以在 `docs/CHANGELOG/v<base-version>/artifacts/` 携带一个可选的
扁平产物目录。每个直接普通文件都会以自身文件名进入精确 prerelease 与 stable
seal。Plumb 保留并证明文件字节的交付，但不解释其用途、类型或内容。

该目录没有文件数量上限。通用发布安全仍会拒绝链接、嵌套条目、非 UTF-8 文件名，
以及与派生发布产物的冲突。

Prerelease 通过其 stable 基础版本解析该目录，因此同一提交会向候选与永久发布
身份提供完全相同的版本所属字节。
