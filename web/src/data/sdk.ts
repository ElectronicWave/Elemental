export interface DriverFamily {
  readonly id: string;
  readonly name: string;
  readonly familyLabel: string;
  readonly drivers: readonly string[];
  readonly sampleGameVersion: string;
  readonly sampleLoaderVersion: string | null;
  readonly javaMajor: number;
  readonly summary: string;
  readonly cliCommand: string;
  readonly rustSpec: string;
  readonly pipelineNotes: {
    readonly metadata: string;
    readonly download: string;
    readonly launch: string;
  };
}

export interface WorkspaceCrate {
  readonly atomicNumber: string;
  readonly symbol: string;
  readonly name: string;
  readonly modulePath: string | null;
  readonly feature: string | null;
  readonly layer: 'facade' | 'orchestration' | 'domain' | 'foundation' | 'binary';
  readonly purpose: string;
  readonly readmeUrl: string;
}

export interface CargoFeature {
  readonly id: string;
  readonly category: 'module' | 'forwarded' | 'tls';
  readonly description: string;
  readonly implies?: readonly string[];
  readonly defaultEnabled: boolean;
}

export const SDK_META = {
  name: 'Elemental',
  packageName: 'elemental-kit',
  version: '0.3.0',
  rustEdition: '2024',
  minRustVersion: '1.85',
  license: 'AGPL-3.0-only',
  repositoryUrl: 'https://github.com/ElectronicWave/Elemental',
  docsRsUrl: 'https://docs.rs/elemental-kit',
  issuesUrl: 'https://github.com/ElectronicWave/Elemental/issues',
} as const;

export const DRIVER_FAMILIES: readonly DriverFamily[] = [
  {
    id: 'vanilla',
    name: 'Vanilla',
    familyLabel: 'version_json',
    drivers: ['Vanilla'],
    sampleGameVersion: '1.20.1',
    sampleLoaderVersion: null,
    javaMajor: 17,
    summary: 'Mojang piston manifest resolution, asset indexing, and native library extraction.',
    cliCommand: 'cargo run -p launchpad -- vanilla 1.20.1 --instance-name MyVanilla-1.20.1',
    rustSpec: 'DriverSpec::vanilla("1.20.1".into())',
    pipelineNotes: {
      metadata: 'Fetch Mojang piston manifest and persist merged version JSON',
      download: 'Batch client jar, rule-filtered libraries, and asset objects',
      launch: 'Evaluate JVM/game rules and emit LaunchCommand with cwd',
    },
  },
  {
    id: 'fabric',
    name: 'Fabric Ecosystem',
    familyLabel: 'version_json · profiled',
    drivers: ['Fabric', 'Legacy Fabric', 'Babric', 'Quilt'],
    sampleGameVersion: '1.20.1',
    sampleLoaderVersion: '0.16.10',
    javaMajor: 17,
    summary: 'Layered loader profile merging over vanilla metadata with Maven coordinate resolution.',
    cliCommand: 'cargo run -p launchpad -- fabric 1.20.1 0.16.10 --instance-name MyFabric-1.20.1',
    rustSpec: 'DriverSpec::from_descriptor(FABRIC_DRIVER, "1.20.1".into(), Some("0.16.10".into()))?',
    pipelineNotes: {
      metadata: 'Merge Fabric/Quilt meta profile onto vanilla piston metadata',
      download: 'Resolve intermediary mappings, loader jars, and vanilla assets',
      launch: 'Compose KnotClient main class and classpath overrides',
    },
  },
  {
    id: 'installer',
    name: 'Installer-Driven',
    familyLabel: 'installer · processor',
    drivers: ['Forge', 'NeoForge', 'CleanroomMC', 'Outlands'],
    sampleGameVersion: '1.20.1',
    sampleLoaderVersion: '47.3.1',
    javaMajor: 17,
    summary: 'Installer profile parsing, processor pipeline execution, and module-path filtering.',
    cliCommand: 'cargo run -p launchpad -- forge 1.20.1 47.3.1',
    rustSpec: 'DriverSpec::from_descriptor(FORGE_DRIVER, "1.20.1".into(), Some("47.3.1".into()))?',
    pipelineNotes: {
      metadata: 'Extract install_profile.json and version.json from installer archive',
      download: 'Fetch processor libraries and run JVM patch processors',
      launch: 'Collect module path entries before classpath deduplication',
    },
  },
  {
    id: 'legacy',
    name: 'Profiled Legacy',
    familyLabel: 'version_json · legacy',
    drivers: ['Rift', 'LiteLoader'],
    sampleGameVersion: '1.12.2',
    sampleLoaderVersion: '1.12.2-SNAPSHOT',
    javaMajor: 8,
    summary: 'Dedicated catalog and manifest adapters for classic tweak-class and profiled loaders.',
    cliCommand: 'cargo run -p launchpad -- liteloader 1.12.2 1.12.2-SNAPSHOT',
    rustSpec: 'DriverSpec::from_descriptor(LITELOADER_DRIVER, "1.12.2".into(), Some("1.12.2-SNAPSHOT".into()))?',
    pipelineNotes: {
      metadata: 'Resolve legacy manifest tweak classes and parent version JSON',
      download: 'Download snapshot/release loader artifacts and legacy assets',
      launch: 'Build legacy tweakClass arguments with Java 8 runtime resolution',
    },
  },
];

export const WORKSPACE_CRATES: readonly WorkspaceCrate[] = [
  {
    atomicNumber: '01',
    symbol: 'Kt',
    name: 'elemental-kit',
    modulePath: 'elemental::*',
    feature: 'full',
    layer: 'facade',
    purpose: 'Re-export facade with selective feature flags and a prelude for end-to-end or modular adoption.',
    readmeUrl: 'https://github.com/ElectronicWave/Elemental/blob/main/README.md#the-elemental-kit-facade',
  },
  {
    atomicNumber: '02',
    symbol: 'Lc',
    name: 'elemental-launcher',
    modulePath: 'elemental::launcher',
    feature: 'launcher',
    layer: 'orchestration',
    purpose: 'High-level Launcher facade, instance inspection, AccountStore, skin/cape APIs, and custom driver registry.',
    readmeUrl: 'https://github.com/ElectronicWave/Elemental/blob/main/crates/launcher/README.md',
  },
  {
    atomicNumber: '03',
    symbol: 'Dr',
    name: 'elemental-driver',
    modulePath: 'elemental::driver',
    feature: 'driver',
    layer: 'domain',
    purpose: 'Distribution catalogs, Send + Sync Driver trait, version_json/installer families, and 11 built-in drivers.',
    readmeUrl: 'https://github.com/ElectronicWave/Elemental/blob/main/crates/driver/README.md',
  },
  {
    atomicNumber: '04',
    symbol: 'Cr',
    name: 'elemental-core',
    modulePath: 'elemental::core',
    feature: 'core',
    layer: 'domain',
    purpose: 'Typed Storage/Layout, Java RuntimeResolver, unified Authorizer trait (Offline/Microsoft/Yggdrasil), and LaunchCommand.',
    readmeUrl: 'https://github.com/ElectronicWave/Elemental/blob/main/crates/core/README.md',
  },
  {
    atomicNumber: '05',
    symbol: 'In',
    name: 'elemental-infra',
    modulePath: 'elemental::infra',
    feature: 'infra',
    layer: 'foundation',
    purpose: 'Session-based async downloader with hardlink cache materialization, retry/rate policies, and JAR utilities.',
    readmeUrl: 'https://github.com/ElectronicWave/Elemental/blob/main/crates/infra/README.md',
  },
  {
    atomicNumber: '06',
    symbol: 'Sh',
    name: 'elemental-shared',
    modulePath: 'elemental::shared',
    feature: 'shared',
    layer: 'foundation',
    purpose: 'Version-aware persistence, typed Migrator chains, and JSON/TOML/in-memory persistors.',
    readmeUrl: 'https://github.com/ElectronicWave/Elemental/blob/main/crates/shared/README.md',
  },
  {
    atomicNumber: '07',
    symbol: 'Ob',
    name: 'elemental-object',
    modulePath: 'elemental::object',
    feature: 'object',
    layer: 'foundation',
    purpose: 'Type-indexed global and task-local object pools with structured async shutdown callbacks.',
    readmeUrl: 'https://github.com/ElectronicWave/Elemental/blob/main/crates/object/README.md',
  },
  {
    atomicNumber: '08',
    symbol: 'Sc',
    name: 'elemental-schema',
    modulePath: 'elemental::schema',
    feature: 'schema',
    layer: 'foundation',
    purpose: 'Strongly typed Mojang, Fabric, Quilt, Forge, and Maven manifest schemas.',
    readmeUrl: 'https://github.com/ElectronicWave/Elemental/blob/main/crates/schema/README.md',
  },
  {
    atomicNumber: '09',
    symbol: 'Lp',
    name: 'launchpad',
    modulePath: null,
    feature: null,
    layer: 'binary',
    purpose: 'Reference CLI launcher with interactive wizard, non-interactive flags, and --local-only execution.',
    readmeUrl: 'https://github.com/ElectronicWave/Elemental/blob/main/crates/launchpad/README.md',
  },
];

export const CARGO_FEATURES: readonly CargoFeature[] = [
  { id: 'launcher', category: 'module', description: 'High-level launcher facade & instance lifecycle', defaultEnabled: true },
  { id: 'driver', category: 'module', description: 'Loader catalogs, families, and built-in drivers', defaultEnabled: false },
  { id: 'core', category: 'module', description: 'Storage, runtime discovery, auth, launch primitives', defaultEnabled: false },
  { id: 'infra', category: 'module', description: 'Download sessions, hardlink cache, JAR tools', defaultEnabled: false },
  { id: 'shared', category: 'module', description: 'Versioned persistence and migrations', defaultEnabled: false },
  { id: 'object', category: 'module', description: 'Type-indexed global and task-local object pools', defaultEnabled: false },
  { id: 'schema', category: 'module', description: 'Mojang and modloader manifest schemas', defaultEnabled: false },

  { id: 'shared-json', category: 'forwarded', description: 'JSON file persistor (enables shared)', implies: ['shared'], defaultEnabled: false },
  { id: 'shared-toml', category: 'forwarded', description: 'TOML file persistor (enables shared)', implies: ['shared'], defaultEnabled: false },
  { id: 'object-notify', category: 'forwarded', description: 'Async notifications for object pools (enables object)', implies: ['object'], defaultEnabled: false },
  { id: 'yggdrasil-auth', category: 'forwarded', description: 'Yggdrasil + authlib-injector launch injection', implies: ['core', 'launcher'], defaultEnabled: false },
  { id: 'offline-server', category: 'forwarded', description: 'Local offline Yggdrasil server for development', implies: ['yggdrasil-auth'], defaultEnabled: false },

  { id: 'tls-aws-lc', category: 'tls', description: 'Default: Rustls with aws-lc-rs', defaultEnabled: true },
  { id: 'tls-native', category: 'tls', description: 'Platform TLS (schannel / system store)', defaultEnabled: false },
  { id: 'tls-native-vendored', category: 'tls', description: 'Platform TLS with vendored OpenSSL', defaultEnabled: false },
  { id: 'tls-rustls-no-provider', category: 'tls', description: 'Rustls without bundled crypto provider', defaultEnabled: false },
];

export const LIFECYCLE_STAGES = [
  {
    index: '01',
    id: 'catalog',
    label: 'Catalog & Spec',
    api: 'DriverSpec::from_descriptor(...)',
    detail: 'Resolve game and loader version metadata from Mojang, Fabric, Forge, or custom registries.',
  },
  {
    index: '02',
    id: 'inspect',
    label: 'Inspect & Reuse',
    api: 'launcher.inspect_instance(name).await?',
    detail: 'Probe local storage layout; reuse verified metadata without redundant filesystem scans.',
  },
  {
    index: '03',
    id: 'download',
    label: 'Download Session',
    api: 'launcher.prepare_with_session(session, ...).await?',
    detail: 'Stream missing artifacts, libraries, and assets via hardlink-cached worker pools.',
  },
  {
    index: '04',
    id: 'natives',
    label: 'Extract Natives',
    api: 'ensure_natives_extracted(&version, &metadata).await?',
    detail: 'Extract only missing platform binaries (.dll/.so/.dylib) while preserving other launchers’ layouts.',
  },
  {
    index: '05',
    id: 'runtime',
    label: 'Resolve Runtime',
    api: 'runtime_resolver.resolve(major_version).await?',
    detail: 'Match required Java major version across custom paths, JAVA_HOME, PATH, and registry providers.',
  },
  {
    index: '06',
    id: 'launch',
    label: 'Build & Spawn',
    api: 'launcher.build_launch_command(...).await? → launch()',
    detail: 'Evaluate OS/arch/feature rules, substitute auth & path variables, and spawn the JVM process.',
  },
] as const;
