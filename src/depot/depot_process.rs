use std::borrow::Cow;
use std::ffi::OsString;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

use crate::depot::{DepotEvent, DownloadRequest, parse_output_line, parse_prompt};

#[derive(Clone, PartialEq)]
pub enum DepotUpdate {
    Event(DepotEvent),
    Exited { success: bool },
}

type Notify = Arc<dyn Fn() + Send + Sync>;

pub struct DepotProcess {
    updates: Receiver<DepotUpdate>,
    child: Arc<Mutex<Child>>,
    stdin: Mutex<Option<ChildStdin>>,
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
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        crate::platform::hide_console(&mut command);
        let mut child = command.spawn()?;
        let stdout = child.stdout.take().expect("stdout was piped");
        let stderr = child.stderr.take().expect("stderr was piped");
        let stdin = Mutex::new(child.stdin.take());
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
        Ok(Self {
            updates,
            child,
            stdin,
        })
    }

    pub fn take_updates(&self) -> Vec<DepotUpdate> {
        self.updates.try_iter().collect()
    }

    /// answers a password or steam guard prompt
    pub fn answer(&self, text: &str) -> io::Result<()> {
        let mut stdin = self.stdin.lock().unwrap_or_else(PoisonError::into_inner);
        let stdin = stdin
            .as_mut()
            .ok_or_else(|| io::Error::other("DepotDownloader has no input"))?;
        stdin.write_all(text.as_bytes())?;
        stdin.write_all(b"\n")?;
        stdin.flush()
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

// prompts end without a newline, so a partial line can be one
fn forward_lines(output: impl Read, sender: &Sender<DepotUpdate>, notify: &dyn Fn()) {
    let mut reader = BufReader::new(output);
    let mut pending = Vec::new();
    loop {
        let chunk = match reader.fill_buf() {
            Ok([]) | Err(_) => return,
            Ok(chunk) => chunk,
        };
        let read = chunk.len();
        pending.extend_from_slice(chunk);
        reader.consume(read);
        let mut events = Vec::new();
        while let Some(end) = pending.iter().position(|&byte| byte == b'\n') {
            let line: Vec<u8> = pending.drain(..=end).collect();
            events.extend(parse_output_line(&to_text(&line)));
        }
        let prompt = parse_prompt(&to_text(&pending));
        if let Some(prompt) = prompt {
            pending.clear();
            events.push(prompt);
        }
        for event in events {
            if sender.send(DepotUpdate::Event(event)).is_err() {
                return;
            }
            notify();
        }
    }
}

// windows pipes use the oem code page, qr blocks are 0xDB
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
