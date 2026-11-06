## Elemental

Follow CONTRIBUTING.md for information on how to contribute to this project.

## Commands

use `./format` / `./format.bat` to format the codebase.

## Code Quality

Use Codacy MCP to fetch Complexity, Duplication, and file score.

Configure your Codacy token in the environment variable `CODACY_ACCOUNT_TOKEN` and use this command to run your mcp-server

```sh
npx -y @codacy/codacy-mcp
```

## Code Style

Prefer to use default trait functions instead of batch of little functions.

Good:

```rust
pub trait MyTrait {
    // Do something forced to be customized
    fn foo(&self);
    fn bar(&self) {
        // call foo() and do something else
    }
}

impl MyTrait for MyStruct {
    fn foo(&self) {
        // Implementation
    }
}
```

Bad:

```rust
fn foo(...) {
}

pub trait MyTrait {
    fn bar(&self);
}

impl MyTrait for MyStruct {
    fn bar(&self) {
        foo()
    }
}
```