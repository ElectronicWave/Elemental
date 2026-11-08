use elemental_driver::families::version_json::{
    BaseInstanceLayout, BaseRootLayout, VersionJsonInstanceLayout, VersionJsonRootLayout,
};
use elemental_infra::downloader::DownloadSession;

use crate::{
    error::Result,
    launcher::Launcher,
    reporter::{NoopReporter, ProgressReporter},
    request::RuntimeOptions,
    result::PreparedInstance,
    spec::DriverSpec,
};

pub struct InstancePreparation<'a, L = BaseRootLayout, VL = BaseInstanceLayout>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    launcher: &'a Launcher<L, VL>,
    session: DownloadSession,
    instance_name: String,
    driver: DriverSpec,
    runtime_options: RuntimeOptions,
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
        runtime_options: &RuntimeOptions,
    ) -> Self {
        Self {
            launcher,
            session,
            instance_name,
            driver,
            runtime_options: runtime_options.clone(),
        }
    }

    pub fn session(&self) -> &DownloadSession {
        &self.session
    }

    pub async fn prepare(self) -> Result<PreparedInstance<L, VL>> {
        self.prepare_with_reporter(&NoopReporter).await
    }

    pub async fn prepare_with_reporter<R>(self, reporter: &R) -> Result<PreparedInstance<L, VL>>
    where
        R: ProgressReporter,
    {
        let Self {
            launcher,
            session,
            instance_name,
            driver,
            runtime_options,
        } = self;

        let mut progress_receiver = session.progress_receiver();
        let mut progress_open = true;
        reporter.report(&session.progress());

        let prepared = {
            let preparation =
                launcher.prepare_in_session(&session, instance_name, driver, &runtime_options);
            tokio::pin!(preparation);

            loop {
                tokio::select! {
                    result = &mut preparation => break result,
                    changed = progress_receiver.changed(), if progress_open => {
                        if changed.is_err() {
                            progress_open = false;
                        } else {
                            let progress = progress_receiver.borrow_and_update().clone();
                            reporter.report(&progress);
                        }
                    }
                }
            }
        };

        reporter.report(&session.progress());
        let report = session.finish().await;
        let prepared = prepared?;
        let report = report?;
        report.success()?;
        Ok(prepared)
    }
}
