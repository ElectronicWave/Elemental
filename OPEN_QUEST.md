# Open Questions

- `crates/driver/src/drivers/*/source.rs`
  `HttpSource<E>` removed the repeated `client + endpoints + with_client` source shell, but endpoint-side duplication is still visible across `Origin + Endpoints + official()/new()/rewrite_upstream()` patterns. If more loaders are added, a small shared endpoint substrate may finally be worth it.

- `crates/driver/src/inspect.rs`
  Identity matching, metadata probing, and installed-instance detection now live together on purpose, but the file is becoming the de facto inspection hub. If a facade layer lands next, this module is the most likely candidate for a public-facing inspection substrate.
