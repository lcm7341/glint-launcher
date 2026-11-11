use crate::depot::{CodeKind, DepotEvent};

pub fn parse_output_line(line: &str) -> Option<DepotEvent> {
    let line = line
        .trim_end_matches(['\r', '\n'])
        .trim_start_matches("\u{1b}[?1h\u{1b}=");
    if line.trim().is_empty() {
        return None;
    }
    if let Some(row) = qr_row(line) {
        return Some(DepotEvent::QrCodeRow(row));
    }
    let text = line.trim();
    if text == "Use the Steam Mobile App to sign in with this QR code:" {
        return Some(DepotEvent::QrCodeFollows);
    }
    if let Some(username) = qr_signed_in(text) {
        return Some(DepotEvent::SignedInAs(username));
    }
    if let Some(prompt) = parse_prompt(text) {
        return Some(prompt);
    }
    if text.starts_with("The previous 2-factor auth code you have provided is incorrect") {
        return Some(DepotEvent::WrongCode);
    }
    if let Some(reason) = text
        .strip_prefix("Failed to authenticate with Steam: ")
        .or_else(|| text.strip_prefix("Unable to login to Steam3: "))
    {
        return Some(DepotEvent::SignInFailed(reason.to_owned()));
    }
    if text.starts_with("STEAM GUARD! Use the Steam Mobile App to confirm") {
        return Some(DepotEvent::ApproveInMobileApp);
    }
    if text.starts_with("Got ") && text.ends_with(" licenses for account!") {
        return Some(DepotEvent::SignedIn);
    }
    if let Some(event) = manifest_found(text) {
        return Some(event);
    }
    if let Some(event) = progress(line) {
        return Some(event);
    }
    if let Some(event) = finished(text) {
        return Some(event);
    }
    if is_failure(text) {
        return Some(DepotEvent::Failed(text.to_owned()));
    }
    if text.starts_with("Encountered error downloading depot manifest") {
        return Some(DepotEvent::Retrying(text.to_owned()));
    }
    Some(DepotEvent::Other(text.to_owned()))
}

/// a prompt waiting for input, which ends without a newline
pub fn parse_prompt(text: &str) -> Option<DepotEvent> {
    let text = text.trim();
    if !text.ends_with(':') {
        return None;
    }
    if text.starts_with("Enter account password for") {
        return Some(DepotEvent::PasswordRequested);
    }
    let text = text.strip_prefix("STEAM GUARD! ").unwrap_or(text);
    if text.starts_with("Please enter your 2-factor auth code")
        || text.starts_with("Please enter your 2 factor auth code")
    {
        return Some(DepotEvent::CodeRequested(CodeKind::App));
    }
    if let Some(email) = text.strip_prefix("Please enter the auth code sent to the email at ") {
        let email = email.trim_end_matches(':').trim().to_owned();
        return Some(DepotEvent::CodeRequested(CodeKind::Email(Some(email))));
    }
    if text.starts_with("Please enter the authentication code sent to your email") {
        return Some(DepotEvent::CodeRequested(CodeKind::Email(None)));
    }
    None
}

fn qr_row(line: &str) -> Option<Vec<bool>> {
    if !line.contains('█') || !line.chars().all(|letter| matches!(letter, '█' | ' ')) {
        return None;
    }
    let letters: Vec<char> = line.chars().collect();
    Some(letters.chunks(2).map(|module| module[0] == '█').collect())
}

fn qr_signed_in(text: &str) -> Option<String> {
    let rest = text.strip_prefix("Success! Next time you can login with -username ")?;
    let (username, _) = rest.split_once(" -remember-password")?;
    Some(username.to_owned())
}

fn manifest_found(text: &str) -> Option<DepotEvent> {
    let rest = text.strip_prefix("Manifest ")?;
    let (id, date) = rest.split_once(" (")?;
    let date = date.strip_suffix(')')?;
    Some(DepotEvent::ManifestFound {
        manifest: id.parse().ok()?,
        date: date.to_owned(),
    })
}

fn progress(line: &str) -> Option<DepotEvent> {
    let (percent, file) = line.trim_start().split_once("% ")?;
    let percent: f32 = percent.parse().ok()?;
    (0.0..=100.0)
        .contains(&percent)
        .then(|| DepotEvent::Progress {
            percent,
            file: file.to_owned(),
        })
}

fn finished(text: &str) -> Option<DepotEvent> {
    let rest = text.strip_prefix("Total downloaded: ")?;
    let (bytes, rest) = rest.split_once(" bytes (")?;
    let (uncompressed, _) = rest.split_once(" bytes uncompressed)")?;
    Some(DepotEvent::Finished {
        bytes: bytes.parse().ok()?,
        uncompressed_bytes: uncompressed.parse().ok()?,
    })
}

fn is_failure(text: &str) -> bool {
    text.starts_with("Unable to download")
        || text.ends_with("Aborting.")
        || text.ends_with("was not completely downloaded.")
        || text == "The operation was canceled."
}
