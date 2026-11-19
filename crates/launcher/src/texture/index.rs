use std::{fmt, path::PathBuf};

use elemental_core::auth::accounts::AccountId;
use serde::{Serialize, de::DeserializeOwned};

use crate::error::Error;

pub(crate) trait TextureId: Copy + Eq + fmt::Display + Send + Sync + 'static {
    fn from_u64(value: u64) -> Self;
}

pub(crate) trait TextureAsset:
    Clone + Serialize + DeserializeOwned + Send + Sync + 'static
{
    type Id: TextureId;

    fn id(&self) -> Self::Id;

    fn rename(&mut self, name: String);
}

pub(crate) trait TextureSelection:
    Clone + Serialize + DeserializeOwned + Send + Sync + 'static
{
    type Id: TextureId;

    fn account_id(&self) -> AccountId;

    fn texture_id(&self) -> Self::Id;
}

pub(crate) trait TextureIndex:
    Clone + Default + Serialize + DeserializeOwned + Send + Sync + 'static
{
    type Id: TextureId;
    type Asset: TextureAsset<Id = Self::Id>;
    type Selection: TextureSelection<Id = Self::Id>;

    fn parts(&self) -> (&u64, &Vec<Self::Asset>, &Vec<Self::Selection>);

    fn parts_mut(&mut self) -> (&mut u64, &mut Vec<Self::Asset>, &mut Vec<Self::Selection>);

    fn not_found(id: Self::Id) -> Error;

    fn file_not_found(id: Self::Id, path: PathBuf) -> Error;

    fn not_png(name: String) -> Error;

    fn id_exhausted() -> Error;

    fn next_id(&self) -> u64 {
        *self.parts().0
    }

    fn set_next_id(&mut self, next_id: u64) {
        *self.parts_mut().0 = next_id;
    }

    fn assets(&self) -> &[Self::Asset] {
        self.parts().1
    }

    fn assets_mut(&mut self) -> &mut Vec<Self::Asset> {
        self.parts_mut().1
    }

    fn remove(&mut self, id: Self::Id) {
        let (_, assets, selections) = self.parts_mut();
        assets.retain(|asset| asset.id() != id);
        selections.retain(|selection| selection.texture_id() != id);
    }

    fn selected_by(&self, account_id: AccountId) -> Option<&Self::Selection> {
        self.parts()
            .2
            .iter()
            .find(|selection| selection.account_id() == account_id)
    }

    fn select(&mut self, selection: Self::Selection) {
        let account_id = selection.account_id();
        self.clear_selection(account_id);
        self.parts_mut().2.push(selection);
    }

    fn clear_selection(&mut self, account_id: AccountId) {
        self.parts_mut()
            .2
            .retain(|selection| selection.account_id() != account_id);
    }

    fn rename(&mut self, id: Self::Id, name: String) {
        if let Some(asset) = self.parts_mut().1.iter_mut().find(|asset| asset.id() == id) {
            asset.rename(name);
        }
    }
}
