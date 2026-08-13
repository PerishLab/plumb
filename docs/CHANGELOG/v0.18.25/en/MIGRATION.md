# Migration

Run `plumb policy --write` in repositories that have replaced their TSX web
surface with Svelte. Review the resulting Ectropy projection before recording
its document seals or landing unrelated changes.

Repositories that still contain TSX, or contain both TSX and Svelte, keep the
corresponding grants automatically.
