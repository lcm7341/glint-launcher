use std::borrow::Cow;
use std::ffi::OsString;
use std::io::{self, BufRead, BufReader, Read};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

use crate::depot::{DepotEvent, DownloadRequest, parse_output_line};

#[derive(Clone, PartialEq)]
pub enum DepotUpdate {
    Event(DepotEvent),
    Exited { success: bool },
}

type Notify = Arc<dyn Fn() + Send + Sync>;

// stdin is closed so a password prompt doesn't hang
pub struct DepotProcess {
    updates: Receiver<DepotUpdate>,
    child: Arc<Mutex<Child>>,
}

impl DepotProcess {
    pub fn start(
        program: &Path,
        request: &DownloadRequest,
        notify: impl Fn() + Send + Sync + 'static,
    ) -> io::Result<Self> {
        Self::spawn(program, request.to_arguments(), notify)
    }

    pub fn spawn(
        program: &Path,
        arguments: Vec<OsString>,
        notify: impl Fn() + Send + Sync + 'static,
    ) -> io::Result<Self> {
        let mut command = Command::new(program);
        command
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        hide_console(&mut command);
        let mut child = command.spawn()?;
        let stdout = child.stdout.take().expect("stdout was piped");
        let stderr = child.stderr.take().expect("stderr was piped");
        let child = Arc::new(Mutex::new(child));
        let notify: Notify = Arc::new(notify);
        let (sender, updates) = mpsc::channel();

        let stderr_reader = {
            let sender = sender.clone();
            let notify = Arc::clone(&notify);
            thread::spawn(move || forward_lines(stderr, &sender, &*notify))
        };
        let waited = Arc::clone(&child);
        thread::spawn(move || {
            forward_lines(stdout, &sender, &*notify);
            let _ = stderr_reader.join();
            let success = waited
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .wait()
                .is_ok_and(|status| status.success());
            let _ = sender.send(DepotUpdate::Exited { success });
            notify();
        });
        Ok(Self { updates, child })
    }

    pub fn take_updates(&self) -> Vec<DepotUpdate> {
        self.updates.try_iter().collect()
    }

    pub fn cancel(&self) {
        let _ = self
            .child
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .kill();
    }
}

impl Drop for DepotProcess {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[cfg(windows)]
fn hide_console(command: &mut Command) {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_console(_command: &mut Command) {}

fn forward_lines(output: impl Read, sender: &Sender<DepotUpdate>, notify: &dyn Fn()) {
    let mut reader = BufReader::new(output);
    let mut line = Vec::new();
    loop {
        line.clear();
        match reader.read_until(b'\n', &mut line) {
            Ok(0) | Err(_) => return,
            Ok(_) => {
                if let Some(event) = parse_output_line(&to_text(&line)) {
                    if sender.send(DepotUpdate::Event(event)).is_err() {
                        return;
                    }
                    notify();
                }
            }
        }
    }
}

// piped output on windows is in the oem code page, qr blocks come through as 0xDB
fn to_text(line: &[u8]) -> Cow<'_, str> {
    if let Ok(text) = str::from_utf8(line) {
        return Cow::Borrowed(text);
    }
    let text = line
        .iter()
        .map(|&byte| match byte {
            0xDB => '█',
            byte if byte.is_ascii() => char::from(byte),
            _ => char::REPLACEMENT_CHARACTER,
        })
        .collect();
    Cow::Owned(text)
}
