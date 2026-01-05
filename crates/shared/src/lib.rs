pub mod loader;
pub mod migrator;
pub mod persistor;
pub mod profile;
pub mod scope;
pub mod version;

#[cfg(test)]
mod shared_test {
    use crate::{migrator::NoMigrator, persistor::NoPersistor, profile::Profile};
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
        let loader = ProfileConfig::load(NoMigrator, NoPersistor, 0)
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
}
