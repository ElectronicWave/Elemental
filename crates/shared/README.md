# Elemental Shared Library

A lightweight, extensible Rust library for managing application profiles and persistent configurations with built-in versioning and migration support.

## Features

✨ **Key Capabilities**
- **Profile Management**: Load, modify, and persist user configurations
- **Version Control**: Track and migrate data across schema versions
- **Flexible Persistence**: JSON, TOML, or custom serialization formats
- **Cross-Platform**: Support for standard system directories (Documents, Config, Home, etc.)
- **Thread-Safe**: Built on `Arc<RwLock<T>>` for safe concurrent access
- **Async/Await**: Non-blocking I/O operations throughout
- **Zero Configuration**: Smart defaults for common scenarios

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
elemental-shared = { path = "crates/shared" }
```

### Optional Features

```toml
[dependencies]
elemental-shared = { path = "crates/shared", features = ["json", "toml"] }
```

- `json` (default): JSON serialization support
- `toml`: TOML serialization support

## Quick Start

### Basic Profile Loading

```rust
use elemental_shared::{
    profile::Profile,
    persistor::json_persistor,
    migrator::NoMigrator,
    scope::Scope,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct GameConfig {
    pub player_name: String,
    pub level: u32,
    pub score: u64,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            player_name: "Player".to_string(),
            level: 1,
            score: 0,
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load profile from disk
    let loader = Profile::load(
        NoMigrator,
        json_persistor("game_save".to_string(), Scope::Document),
        1,  // schema version
    )
    .await?;

    // Read configuration
    let player_name = loader.get(|profile| {
        profile.config.player_name.clone()
    }).await;
    println!("Welcome, {}!", player_name);

    // Modify and save configuration
    loader.set(|profile| {
        profile.config.level += 1;
        profile.config.score += 100;
    }).await?;

    Ok(())
}
```

## Core Concepts

### 1. Profiles

A `Profile<C>` contains:
- `name`: Human-readable profile identifier
- `config`: Your custom configuration type (generic `C`)
- `version`: Schema version for migrations

```rust
#[derive(Serialize, Deserialize)]
pub struct Profile<C> {
    pub name: String,
    pub config: C,
    pub version: usize,
}
```

### 2. Loaders

`Loader<M, V, P>` provides thread-safe access to your profile:

```rust
// Immutable read
let value = loader.get(|profile| profile.config.score).await;

// Mutable modification (auto-saves)
loader.set(|profile| {
    profile.config.score += 50;
}).await?;

// Clone entire profile
let profile_copy = loader.cloned().await;
```

### 3. Storage Scopes

Choose where files are saved:

```rust
use elemental_shared::scope::Scope;

// Standard locations
Scope::Document        // User's Documents folder
Scope::Home            // User's home directory (~)
Scope::Config          // System config directory (~/.config)
Scope::ConfigLocal     // Local config directory
Scope::Dot             // Current working directory

// Custom path
Scope::Custom(PathBuf::from("/path/to/storage"))
```

Files are always stored in a `.elemental/` subdirectory:
```
~/.config/.elemental/my_app.json
~/Documents/.elemental/game_save.json
./.elemental/config.toml
```

### 4. Serialization Formats

#### JSON (default)
```rust
use elemental_shared::persistor::json_persistor;

let persistor = json_persistor("config".to_string(), Scope::Config);
```

#### TOML (requires `toml` feature)
```rust
use elemental_shared::persistor::toml_persistor;

let persistor = toml_persistor("settings".to_string(), Scope::Home);
```

#### Custom Format

For custom serialization formats, create a helper function:

```rust
use elemental_shared::{
    persistor::{CustomPersistor, AsyncFileStringIO},
    scope::Scope,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use anyhow::Result;

fn my_custom_persistor<V: Serialize + DeserializeOwned>(
    id: String,
    scope: Scope,
) -> CustomPersistor<
    V,
    String,
    AsyncFileStringIO,
    impl Fn(&V) -> Result<String>,
    impl Fn(&String) -> Result<V>,
> {
    CustomPersistor::new(
        |v: &V| serde_json::to_string(v).map_err(Into::into),     // serializer
        |s: &String| serde_json::from_str(s).map_err(Into::into), // deserializer
        id,
        scope,
        Some("json".to_string()),
    )
}

// Usage
let persistor = my_custom_persistor::<MyData>("my_config".to_string(), Scope::Config);
```

## Advanced Usage

### Version Migration

Implement custom migration logic to handle schema evolution:

```rust
use elemental_shared::version::{Migrator, VersionControlled};
use anyhow::Result;

#[derive(Serialize, Deserialize)]
pub struct MyConfig {
    pub player_name: String,
    pub level: u32,
}

impl VersionControlled for Profile<MyConfig> {
    fn version(&self) -> usize {
        self.version
    }
}

pub struct MyMigrator;

impl Migrator<Profile<MyConfig>> for MyMigrator {
    fn migrate(&self, mut profile: Profile<MyConfig>, target_version: usize) -> Result<()> {
        if profile.version < 2 && target_version >= 2 {
            // Migration from v1 to v2: add default values
            // Perform transformations here
        }
        profile.version = target_version;
        Ok(profile)
    }
}

// Usage
let loader = Profile::load(
    MyMigrator,
    json_persistor("config".to_string(), Scope::Config),
    2,  // target version
).await?;
```

### No Persistence (Testing)

Use `NoPersistor` for testing without file I/O:

```rust
use elemental_shared::persistor::NoPersistor;

// Data loads with default values and saves are discarded
let loader = Profile::load(
    NoMigrator,
    NoPersistor,
    1,
).await?;
```

### Direct Loader Usage

For more control, use `Loader` directly:

```rust
use elemental_shared::loader::Loader;

let loader = Loader::load(
    your_migrator,
    your_persistor,
    current_version,
).await?;

// Access the raw inner value
let value = loader.cloned().await;
```

## API Reference

### Profile

```rust
impl<C: Default> Profile<C> {
    pub async fn load<M, P>(
        migrator: M,
        persistor: P,
        version: usize,
    ) -> Result<ProfileLoader<M, C, P>>
    where
        M: Migrator<Profile<C>>,
        P: Persistor<Profile<C>>;
}
```

### Loader

```rust
pub struct Loader<M, V, P> { /* ... */ }

impl<M, V, P> Loader<M, V, P> {
    pub async fn load(
        migrator: M,
        persistor: P,
        loader_version: usize,
    ) -> Result<Self>;

    pub async fn get<T>(&self, f: impl FnOnce(&V) -> T) -> T;

    pub async fn set(&self, f: impl FnOnce(&mut V)) -> Result<()>;

    pub async fn cloned(&self) -> V where V: Clone;
}
```

### Persistors

```rust
// JSON (requires 'json' feature)
pub fn json_persistor<V>(id: String, scope: Scope) -> CustomPersistor<...>;

// TOML (requires 'toml' feature)
pub fn toml_persistor<V>(id: String, scope: Scope) -> CustomPersistor<...>;

// No-op persistor (testing)
pub struct NoPersistor;
```

### Scope

```rust
pub enum Scope {
    Document,
    Home,
    Config,
    ConfigLocal,
    Dot,
    Custom(PathBuf),
}

impl Scope {
    pub fn path(&self) -> Option<PathBuf>;
    pub async fn get_full_path(&self, id: &str, suffix: Option<String>) -> Result<PathBuf>;
}
```

### Traits

```rust
pub trait VersionControlled: Default {
    fn version(&self) -> usize;
    fn is_up_to_date(&self, latest_version: usize) -> bool {
        self.version() >= latest_version
    }
}

pub trait Migrator<V: VersionControlled> {
    fn migrate(&self, value: V, target_version: usize) -> Result<V>;
}

pub trait Persistor<V: VersionControlled> {
    async fn load(&self) -> Result<Option<V>>;
    async fn save(&self, value: &V) -> Result<()>;
}
```

## Examples

### Example: Game Save System

```rust
use elemental_shared::{profile::Profile, persistor::json_persistor, scope::Scope};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct GameState {
    pub position_x: f32,
    pub position_y: f32,
    pub inventory: Vec<String>,
    pub health: i32,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load game save
    let game = Profile::load(
        NoMigrator,
        json_persistor("save_slot_1".to_string(), Scope::Document),
        1,
    ).await?;

    // Read current state
    let pos = game.get(|p| (p.config.position_x, p.config.position_y)).await;
    println!("Player at: {:?}", pos);

    // Update after action
    game.set(|p| {
        p.config.position_x += 5.0;
        p.config.inventory.push("Sword".to_string());
        p.config.health -= 10;
    }).await?;

    println!("Game saved!");
    Ok(())
}
```

### Example: Application Settings

```rust
use elemental_shared::{profile::Profile, persistor::toml_persistor, scope::Scope};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct AppSettings {
    pub theme: String,
    pub language: String,
    pub auto_save: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = Profile::load(
        NoMigrator,
        toml_persistor("app_config".to_string(), Scope::Config),
        1,
    ).await?;

    // Apply settings
    let theme = settings.get(|p| p.config.theme.clone()).await;

    // Update settings
    settings.set(|p| {
        p.config.theme = "dark".to_string();
        p.config.auto_save = true;
    }).await?;

    Ok(())
}
```

## Performance Considerations

- **Lock Contention**: `RwLock` allows concurrent readers; minimize time in `set()` closures
- **File I/O**: Operations are async but file operations may block; consider using a dedicated runtime executor for I/O-heavy workloads
- **Serialization**: Large configurations may slow down save/load; consider streaming or partial updates

## Common Pitfalls

### ❌ Long-Running Operations in Closures

```rust
// BAD: Holds write lock during network request
loader.set(|profile| {
    let data = std::net::TcpStream::connect("...").unwrap(); // Blocks!
    profile.config.data = data;
}).await?;
```

### ✅ Proper Pattern

```rust
// GOOD: Fetch data, then update
let data = std::net::TcpStream::connect("...").await?;
loader.set(|profile| {
    profile.config.data = data;
}).await?;
```

### ❌ Ignoring Version Mismatches

Always check that your schema version matches your data structure.

### ✅ Proper Pattern

```rust
// Define version as constant, use consistently
const APP_VERSION: usize = 2;

Profile::load(
    MyMigrator,
    json_persistor("config".to_string(), Scope::Config),
    APP_VERSION,
).await?
```

## Testing

The library includes built-in tests demonstrating:
- Loading profiles with default values
- Modifying and persisting data
- Round-trip serialization verification

Run tests with:
```bash
cargo test -p elemental-shared
```

## Contributing

When extending this library:

1. **Implement new Migrators**: Extend the trait for custom upgrade paths
2. **Add Persistor implementations**: Support new storage backends
3. **Enhance Scope options**: Add standard directory conventions for new platforms
4. **Write comprehensive tests**: Use `#[tokio::test]` for async tests

## License

See the main repository LICENSE file.

## Related Crates

- `serde`: Serialization framework
- `tokio`: Async runtime
- `dirs`: Cross-platform directory resolution
- `anyhow`: Error handling

## FAQ

**Q: Can I use this without async/await?**
A: The API requires `async`, but you can use `tokio::task::block_in_place()` to run async code in synchronous contexts.

**Q: Can I share a Loader across threads?**
A: Yes! `Loader<M, V, P>` is `Send + Sync` when all generic parameters are, so you can wrap it in `Arc` and share freely.

**Q: What happens if the file is corrupted?**
A: The `load()` method returns an error; you can handle this by falling back to defaults.

**Q: Can I have multiple profiles?**
A: Yes! Create multiple `Loader` instances with different IDs or scopes.

**Q: Is data encrypted?**
A: No. Implement encryption in a custom `Persistor` if needed.

---

**For detailed architectural information, see [DESIGN.md](DESIGN.md)**
