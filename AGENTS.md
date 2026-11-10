## Elemental

Follow CONTRIBUTING.md for information on 
how to contribute to this project.

## Compatibility

- Respect vanilla configuration
- Respect loader configuration
- Respect other launchers' configuration
- Avoid modifying existing files whenever possible

## Commands

use `./format` / `./format.bat` to format the codebase.

## Code Quality

Use Codacy MCP to fetch Complexity, Duplication, and file score.

Configure your Codacy token in the environment variable `CODACY_ACCOUNT_TOKEN` and use this command to run your mcp-server

```sh
npx -y @codacy/codacy-mcp
```

## Code Style

`Numquam ponenda est pluralitas sine necessitate”`

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

## Refactoring

- Combine loops, iterator chains, and helpers based on the operation; do not
  apply one control-flow style everywhere.
- Use loops for stateful traversal, early exits, fallible collection, and
  mutation of owned state.
- Use iterator chains for simple transformations and direct composition.
- Extract a helper only when it names a real behavior or removes duplication.
- Do not replace readable control flow with long `map`/`flatten`/`collect`
  chains solely to satisfy a complexity metric.
- Evaluate refactors by readability, ownership boundaries, and testability,
  not by CCN alone.

## Breaking Changes

- Dare to make breaking changes when they remove dead compatibility layers or
  simplify the design.
- Do not preserve an obsolete API without an active consumer.
- Always explicitly tell the user when a change is breaking, including the
  affected API and the required migration.
