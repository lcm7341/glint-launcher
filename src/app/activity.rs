use super::Notice;

#[derive(Default)]
pub(super) struct Activity {
    notices: Vec<Notice>,
    log: Vec<String>,
    log_added: usize,
}

impl Activity {
    pub(super) fn notice(&mut self, notice: Notice) {
        self.notices.push(notice);
    }
    pub(super) fn extend(&mut self, notices: impl IntoIterator<Item = Notice>) {
        self.notices.extend(notices);
    }
    pub(super) fn push_log(&mut self, line: String) {
        if self.log.len() == 2000 {
            self.log.remove(0);
        }
        self.log.push(line);
        self.log_added += 1;
    }
    pub(super) fn take_notices(&mut self) -> Vec<Notice> {
        let notices = std::mem::take(&mut self.notices);
        for notice in &notices {
            self.push_log(notice.text.clone());
        }
        notices
    }
    pub(super) fn log(&self) -> &[String] {
        &self.log
    }
    pub(super) fn log_added(&self) -> usize {
        self.log_added
    }
}
