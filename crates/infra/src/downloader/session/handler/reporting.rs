use super::SessionHandler;
use crate::downloader::event::{DownloadEvent, SessionSnapshot};
use crate::downloader::report::DownloadReport;

impl SessionHandler {
    pub(crate) fn snapshot(&self) -> SessionSnapshot {
        self.lock_state().snapshot(self.name.clone())
    }

    pub(crate) fn build_report(&self) -> DownloadReport {
        self.lock_state()
            .report(self.name.clone(), self.created_at.elapsed())
    }

    pub(crate) fn emit_closed_report(&self) -> DownloadReport {
        let report = self.build_report();
        self.emit(DownloadEvent::SessionClosed {
            session_id: self.session_id,
            report: report.clone(),
        });
        report
    }
}
