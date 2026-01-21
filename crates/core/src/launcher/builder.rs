use super::model::LaunchEnvs;
use crate::{auth::authorizer::Authorizer, runtime::distribution::Distribution};

pub struct LaunchBuilder<A: Authorizer> {
    pub authorizer: A,
    pub runtime: Distribution,
    inner: LaunchEnvs,
}

impl<A: Authorizer> LaunchBuilder<A> {
    pub fn new(authorizer: A, runtime: Distribution) -> Self {
        Self {
            authorizer,
            runtime,
            inner: LaunchEnvs::default(),
        }
    }

    pub fn set_quick_play_path(
        mut self,
        quick_play_path: Option<String>,
        quick_play_multiplayer: Option<String>,
        quick_play_singleplayer: Option<String>,
        quick_play_realms: Option<String>,
    ) -> Self {
        self.inner.quick_play_path = quick_play_path;
        self.inner.quick_play_multiplayer = quick_play_multiplayer;
        self.inner.quick_play_singleplayer = quick_play_singleplayer;
        self.inner.quick_play_realms = quick_play_realms;
        self
    }

    pub fn set_username(mut self, username: String) -> Self {
        self.inner.auth_player_name = username;
        self
    }

    pub fn set_resolution(mut self, width: String, height: String) -> Self {
        self.inner.resolution_width = width;
        self.inner.resolution_height = height;
        self
    }

    pub fn set_client_id(mut self, client_id: String) -> Self {
        self.inner.clientid = client_id;
        self
    }

    pub fn set_launcher(mut self, name: String, version: String) -> Self {
        self.inner.launcher_name = name;
        self.inner.launcher_version = version;
        self
    }

    pub fn build(self) -> LaunchEnvs {
        self.inner
    }
}
