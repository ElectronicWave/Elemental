extern crate self as elemental_shared;

pub mod error;
pub mod loader;
pub mod migrator;
pub mod persistor;
pub mod profile;
pub mod scope;
pub mod store;
pub mod version;

#[cfg(test)]
mod shared_test {
    use crate::{
        error::Result,
        migrator::{Migrator, NoMigrator},
        persistor::{MemoryPersistor, json_persistor},
        profile::Profile,
        scope::Scope,
        version::Persistor,
    };
    use serde::{Deserialize, Serialize};

    /// A sample configuration struct for testing purposes.
    #[derive(Serialize, Deserialize, Clone)]
    pub struct Config {
        user: String,
    }

    impl Default for Config {
        fn default() -> Self {
            Config {
                user: "Player".to_owned(),
            }
        }
    }

    type ProfileConfig = Profile<Config>;

    #[tokio::test]
    async fn test_profile() {
        let loader = ProfileConfig::load(
            NoMigrator,
            json_persistor("foobar".to_string(), Scope::Dot),
            0,
        )
        .await
        .unwrap();

        println!(
            "{:?}",
            loader.get(|profile| profile.config.user.clone()).await
        ); // Player
        loader
            .set(|profile| profile.config.user = "NewPlayer".to_owned())
            .await
            .unwrap();
        println!(
            "{:?}",
            loader.get(|profile| profile.config.user.clone()).await
        ); // NewPlayer
    }

    #[tokio::test]
    async fn v1v2v3() -> Result<()> {
        #[derive(Serialize, Deserialize, Clone, Default)]
        struct V1 {
            name: String,
        }
        #[derive(Serialize, Deserialize, Clone, Default)]
        struct V2 {
            name2: String,
        }
        #[derive(Serialize, Deserialize, Clone, Default)]
        struct V3 {
            name3: String,
        }

        #[derive(Clone, Serialize, Deserialize)]
        #[serde(untagged)]
        enum Versions {
            V1(V1),
            V2(V2),
            V3(V3),
        }

        struct ProfileMigrator;

        impl Migrator<Profile<Versions>, Profile<V3>> for ProfileMigrator {
            fn migrate(&self, stored: Profile<Versions>) -> Result<Profile<V3>> {
                let Profile { config, version } = stored;
                let config = match config {
                    Versions::V1(config) => V3 { name3: config.name },
                    Versions::V2(config) => V3 {
                        name3: config.name2,
                    },
                    Versions::V3(config) => return Ok(Profile { config, version }),
                };

                Ok(Profile { config, version })
            }

            fn encode(&self, current: &Profile<V3>) -> Result<Profile<Versions>> {
                Ok(Profile {
                    config: Versions::V3(current.config.clone()),
                    version: current.version,
                })
            }
        }

        let persistor = MemoryPersistor::new(Some(Profile {
            config: Versions::V1(V1 {
                name: "Player".to_string(),
            }),
            version: 1,
        }));
        let saved = persistor.clone();

        let loader = Profile::<V3>::load(ProfileMigrator, persistor, 3).await?;
        assert_eq!(
            loader.get(|profile| profile.config.name3.clone()).await,
            "Player"
        );
        assert!(matches!(
            saved.load().await?,
            Some(Profile {
                config: Versions::V3(V3 { name3 }),
                version: 3,
            }) if name3 == "Player"
        ));

        Ok(())
    }
}
