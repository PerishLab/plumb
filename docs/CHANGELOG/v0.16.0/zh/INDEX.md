# Plumb v0.16.0

本版本将 canonical stable 明确为全域共识锚点。所有 non-stable channel
仍然只是精确、不可变的验证候选，只能安装到显式隔离的席位。

Plumb 现在完整拥有 binary 发布机制：Cargo 发现与版本固化、target 构建、
归档、skill、Debian 包、Cargo 附件、manager 生成、密封存储、验证、
激活、smoke 与 stable tag。`plumb release` 的运行值统一通过类型化环境
字段注入。

Plumb skill 也新增了一个偏好：高度重复的 Plumb-shaped 机制是所有权信号；
当闭包足够清晰时，应主动反馈并评估是否由 substrate 吸收。
