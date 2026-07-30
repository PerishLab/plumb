# Plumb v0.18.3

## 第一方依赖的实时稳定版本

Plumb 现在要求 perish.code 自有的每条直接发布依赖都跟随 registry 的实时
stable latest。该规则覆盖 `@perish` JSR scope 与 `perish` Cargo registry，
不再维护第二份 package-version 表。

Deno 声明保持不带版本，其精确 lock resolution 必须为当前稳定版本。Cargo
保留原生 requirement 语法，但直接依赖的精确 lock resolution 同样必须等于
stable latest。同一 workspace 内的 path 依赖仍由 release train 治理，不被
当作已发布 registry edge。

## 证据盲区会阻断

Doctor 以有界方式读取实时 registry metadata，并报告每项依赖的 ecosystem、
声明位置、requirement、resolution 与 latest。已知过期是 out of true；
manifest、lock 或 registry 证据缺失或格式错误是 blind，现在会让 doctor
返回非零。未知 repository shape 仍不阻断。

Plumb 只验证，绝不修改依赖声明或 lock。
