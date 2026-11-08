use elemental_driver::families::version_json::{
    BaseInstanceLayout, BaseRootLayout, VersionJsonInstanceLayout, VersionJsonRootLayout,
};
use elemental_infra::downloader::DownloadSession;

use crate::{error::Result, launcher::Launcher, result::PreparedInstance, spec::DriverSpec};

pub struct InstancePreparation<'a, L = BaseRootLayout, VL = BaseInstanceLayout>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    launcher: &'a Launcher<L, VL>,
    session: DownloadSession,
    instance_name: String,
    driver: DriverSpec,
}

impl<'a, L, VL> InstancePreparation<'a, L, VL>
where
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone + Send,
{
    pub(crate) fn new(
        launcher: &'a Launcher<L, VL>,
        session: DownloadSession,
        instance_name: String,
        driver: DriverSpec,
    ) -> Self {
        Self {
            launcher,
            session,
            instance_name,
            driver,
        }
    }

    pub fn session(&self) -> &DownloadSession {
        &self.session
    }

    pub async fn prepare(self) -> Result<PreparedInstance<L, VL>> {
        let Self {
            launcher,
            session,
            instance_name,
            driver,
        } = self;
        let prepared = launcher
            .prepare_in_session(&session, instance_name, driver)
            .await;
        let report = session.finish().await;
        let prepared = prepared?;
        let report = report?;
        report.success()?;
        Ok(prepared)
    }
}
