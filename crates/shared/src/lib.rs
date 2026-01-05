pub mod loader;
pub mod migrator;
pub mod persistor;
pub mod profile;
pub mod version;

#[cfg(test)]
mod shared_test {
    use super::profile::Profile;
    use crate::{migrator::NoMigrator, persistor::NoPersistor};
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

        println!("{:?}", loader.cloned().await.config.user);
    }
}
