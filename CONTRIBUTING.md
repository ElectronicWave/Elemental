# Elemental Contribution Guideline

Thank you for considering contributing to Elemental!

Please try to follow the advice below before coding.

## API Design

Funciton should be clear and easy to use.

## Code Style

Avoid making `panic!`, use `std::io::Result<T>` or just log and ignore.

Use `cargo format` to sort your code.

## Dependencies

Use `cargo-sort` to sort the `Cargo.toml` once you make changes to it.

```sh
cargo install cargo-sort
cargo sort
```
