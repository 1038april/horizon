//! Launcher-owned authentication operations. UI, CLI and a future game bridge receive only
//! public status; credentials stay behind SecretStore and the existing session-file handoff.
use super::*;
use std::fs::{File, OpenOptions};
use std::sync::MutexGuard;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Ready,
    NeedsRefresh,
    SignInRequired,
    Retry,
    OwnershipRequired,
}

/// Safe to show or serialize: contains no credential or secret-store contents.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Status {
    pub state: State,
    pub expires_at: Option<u64>,
    pub message: String,
}

const DAMAGED_SIGN_IN: &str = "Could not read the saved sign-in. Sign in again.";

impl Status {
    pub fn label(&self) -> &'static str {
        match self.state {
            State::Ready => "Ready for online play",
            State::NeedsRefresh => "Needs refresh",
            State::SignInRequired => "Sign in again",
            State::Retry => "Could not check sign-in",
            State::OwnershipRequired => "Minecraft account needs attention",
        }
    }

    fn failed(message: &str) -> Self {
        Self {
            state: if message == DAMAGED_SIGN_IN {
                State::SignInRequired
            } else {
                State::Retry
            },
            expires_at: None,
            message: message.into(),
        }
    }

    fn from_secrets(secrets: Option<&Secrets>) -> Self {
        let state = match secrets {
            None => State::SignInRequired,
            Some(s) if s.sign_in_required => State::SignInRequired,
            Some(s) if !s.owns => State::OwnershipRequired,
            Some(s) if s.minecraft_expires_at <= now() + RENEW_BEFORE => State::NeedsRefresh,
            Some(_) => State::Ready,
        };
        let mut status = Self {
            state,
            expires_at: secrets.map(|s| s.minecraft_expires_at),
            message: String::new(),
        };
        status.message = match state {
            State::SignInRequired => "Sign in again in your browser.".into(),
            State::OwnershipRequired => {
                "Check Minecraft: Java Edition ownership and your profile at minecraft.net.".into()
            }
            _ => status.label().into(),
        };
        status
    }
}

static OPERATIONS: Mutex<()> = Mutex::new(());

pub(super) struct Operation {
    _process: MutexGuard<'static, ()>,
    _file: Option<File>,
}

/// Serialize refresh with sign-in, sign-out and launch, before reading any credential.
/// The file is a lock only: it never contains tokens. Fail closed if locking fails.
pub(super) fn lock(store: &dyn SecretStore) -> Result<Operation, String> {
    let process = OPERATIONS
        .lock()
        .map_err(|_| "Account operation failed. Restart the launcher.")?;
    let file = store
        .lock_file()
        .map(|path| {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|_| "Could not create the account lock.")?;
            }
            let file = OpenOptions::new()
                .create(true)
                .truncate(false)
                .read(true)
                .write(true)
                .open(path)
                .map_err(|_| "Could not open the account lock.")?;
            file.lock()
                .map_err(|_| "Could not lock account operations.")?;
            Ok::<_, String>(file)
        })
        .transpose()?;
    Ok(Operation {
        _process: process,
        _file: file,
    })
}

fn read(store: &dyn SecretStore, uuid: &str) -> Result<Option<Secrets>, String> {
    store
        .get(uuid)
        .map_err(|_| "Could not read the keychain. Unlock it and retry.".to_string())?
        .map(|text| serde_json::from_str(&text).map_err(|_| DAMAGED_SIGN_IN.into()))
        .transpose()
}

/// Inspect local expiry and remembered failures without contacting authentication services.
/// Call off the UI thread: unlocking the keychain can require user interaction.
pub fn status(store: &dyn SecretStore, uuid: &str) -> Status {
    let result = (|| {
        let _operation = lock(store)?;
        Ok::<_, String>(Status::from_secrets(read(store, uuid)?.as_ref()))
    })();
    result.unwrap_or_else(|e| Status::failed(&e))
}

/// Force a renewal even when the access token is fresh. Does not select an account, change
/// the public account list, or return a token. A future IPC handler can call this same API,
/// then use write_session_file for an explicit game-session handoff.
pub fn refresh(who: &str) -> Result<Status, String> {
    let account = Accounts::load()
        .find(who)
        .cloned()
        .ok_or("That account isn't signed in.")?;
    Ok(refresh_with(
        &Endpoints::live(),
        client_id().as_deref(),
        store(),
        &account,
    ))
}

pub(super) fn refresh_with(
    ep: &Endpoints,
    client_id: Option<&str>,
    store: &dyn SecretStore,
    account: &Account,
) -> Status {
    let result = (|| {
        let _operation = lock(store)?;
        let Some(secrets) = read(store, &account.uuid)? else {
            return Ok(Status::from_secrets(None));
        };
        let before =
            serde_json::to_string(&secrets).map_err(|_| "Could not read the saved sign-in.")?;
        let (mut status, secrets) = match check_with(ep, client_id, account, secrets, true) {
            Check::Owns(_, s) => (Status::from_secrets(Some(&s)), s),
            Check::DoesNot(s) => (Status::from_secrets(Some(&s)), s),
            Check::NoAnswer(e, s) => {
                let mut status = Status::from_secrets(Some(&s));
                if !s.sign_in_required {
                    status.state = State::Retry;
                }
                status.message = e;
                (status, s)
            }
        };
        let after =
            serde_json::to_string(&secrets).map_err(|_| "Could not save the renewed sign-in.")?;
        if before != after && store.set(&account.uuid, &after).is_err() {
            status.state = State::Retry;
            status.message =
                "The renewed sign-in could not be saved in the keychain. Unlock it and retry."
                    .into();
        }
        Ok::<_, String>(status)
    })();
    result.unwrap_or_else(|e| Status::failed(&e))
}
