//! Passwords live only in Secret Service, scoped to the exact URL/method/account.
//! `secret-tool` is invoked without a shell; secret bytes travel through stdin,
//! never process arguments, config files, error messages or application logs.
use super::{ConnectionConfig, LoginMethod};
use std::{
    io::Write,
    process::{Command, Stdio},
    sync::Arc,
};

pub(super) trait CredentialStore: Send + Sync {
    fn lookup(&self, config: &ConnectionConfig) -> Result<Option<String>, ()>;
    fn store(&self, config: &ConnectionConfig, password: &str) -> Result<(), ()>;
    fn clear(&self, config: &ConnectionConfig) -> Result<(), ()>;
}
pub(super) struct SystemKeyring;

fn attributes(config: &ConnectionConfig) -> Vec<String> {
    vec![
        "application".into(),
        "cafe.avery.Delfin.Seerr".into(),
        "server".into(),
        config.url().into(),
        "account".into(),
        config.username.as_deref().unwrap_or("").into(),
        "method".into(),
        match config.login_method {
            LoginMethod::Local => "local",
            LoginMethod::Jellyfin => "jellyfin",
            LoginMethod::JellyfinQuickConnect => "quick_connect",
        }
        .into(),
    ]
}
fn command(action: &str, config: &ConnectionConfig) -> Command {
    let mut cmd = Command::new("/usr/bin/timeout");
    cmd.args(["--kill-after=1s", "15s", "/usr/bin/secret-tool", action]);
    if action == "store" {
        cmd.args(["--label", "Delfin — Seerr sign-in"]);
    }
    cmd.args(attributes(config));
    cmd.stderr(Stdio::null());
    cmd
}
impl CredentialStore for SystemKeyring {
    fn lookup(&self, config: &ConnectionConfig) -> Result<Option<String>, ()> {
        let output = command("lookup", config)
            .stdin(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .map_err(|_| ())?;
        if output.status.code() == Some(1) && output.stderr.is_empty() {
            return Ok(None);
        }
        if !output.status.success() || output.stdout.len() > 16 * 1024 {
            return Err(());
        }
        // With piped stdout, secret-tool returns the exact bytes, no added
        // newline. Preserve whitespace, including a password's trailing newline.
        let secret = String::from_utf8(output.stdout).map_err(|_| ())?;
        Ok((!secret.is_empty()).then_some(secret))
    }
    fn store(&self, config: &ConnectionConfig, password: &str) -> Result<(), ()> {
        if password.is_empty() || password.len() > 16 * 1024 {
            return Err(());
        }
        let mut child = command("store", config)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .map_err(|_| ())?;
        let written = child
            .stdin
            .take()
            .ok_or(())
            .and_then(|mut stdin| stdin.write_all(password.as_bytes()).map_err(|_| ()));
        let status = child.wait().map_err(|_| ())?;
        written?;
        if status.success() {
            Ok(())
        } else {
            Err(())
        }
    }
    fn clear(&self, config: &ConnectionConfig) -> Result<(), ()> {
        let output = command("clear", config)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .map_err(|_| ())?;
        // Distinguish a missing item from a keyring/permission error. Never log stderr.
        if output.status.success() || (output.status.code() == Some(1) && output.stderr.is_empty())
        {
            Ok(())
        } else {
            Err(())
        }
    }
}

pub(super) async fn lookup(
    store: Arc<dyn CredentialStore>,
    config: ConnectionConfig,
) -> Result<Option<String>, ()> {
    tokio::task::spawn_blocking(move || store.lookup(&config))
        .await
        .map_err(|_| ())?
}
pub(super) async fn save(
    store: Arc<dyn CredentialStore>,
    config: ConnectionConfig,
    password: String,
    account: Arc<std::sync::atomic::AtomicU64>,
    expected: u64,
) -> Result<(), ()> {
    // The blocking task outlives cancellation of its awaiting GTK/service task.
    // Perform revocation cleanup here too, so closing the app during a pending
    // save cannot undo an explicit logout/account change.
    tokio::task::spawn_blocking(move || {
        use std::sync::atomic::Ordering;
        if account.load(Ordering::Relaxed) != expected {
            return Err(());
        }
        let result = store.store(&config, &password);
        if result.is_ok() && account.load(Ordering::Relaxed) != expected {
            store.clear(&config)?;
            return Err(());
        }
        result
    })
    .await
    .map_err(|_| ())?
}
pub(super) async fn clear(
    store: Arc<dyn CredentialStore>,
    config: ConnectionConfig,
) -> Result<(), ()> {
    tokio::task::spawn_blocking(move || store.clear(&config))
        .await
        .map_err(|_| ())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires an unlocked system keyring; creates and removes only a unique dummy credential"]
    fn system_keyring_roundtrip_preserves_exact_secret_bytes() {
        let config = ConnectionConfig::new(
            &format!(
                "https://keyring-self-test.invalid/{}/",
                uuid::Uuid::new_v4()
            ),
            LoginMethod::Local,
            Some("temporary-self-test".into()),
        )
        .unwrap();
        struct Cleanup(ConnectionConfig);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = SystemKeyring.clear(&self.0);
            }
        }
        let _cleanup = Cleanup(config.clone());
        for password in [
            "dummy password",
            " dummy password with trailing whitespace \n",
        ] {
            SystemKeyring.store(&config, password).unwrap();
            assert_eq!(
                SystemKeyring.lookup(&config).unwrap().as_deref(),
                Some(password)
            );
        }
        SystemKeyring.clear(&config).unwrap();
        assert!(SystemKeyring.lookup(&config).unwrap().is_none());
    }
    #[test]
    fn keyring_scope_separates_exact_servers_methods_and_accounts() {
        let config = ConnectionConfig::new(
            "https://media/seerr",
            LoginMethod::Local,
            Some("name with spaces; $(not-a-command)".into()),
        )
        .unwrap();
        let args = attributes(&config);
        assert_eq!(args[3], "https://media/seerr/");
        assert_eq!(args[5], "name with spaces; $(not-a-command)");
        assert_eq!(args[7], "local");
        let other = ConnectionConfig::new(
            "https://media:5055/seerr",
            LoginMethod::Jellyfin,
            config.username.clone(),
        )
        .unwrap();
        assert_ne!(attributes(&config), attributes(&other));
        let cmd = command("store", &config);
        let args: Vec<_> = cmd.get_args().map(|v| v.to_string_lossy()).collect();
        assert_eq!(cmd.get_program(), "/usr/bin/timeout");
        assert!(args.iter().any(|arg| arg == "15s"));
        // Only metadata is passed in argv; store() passes the password via stdin.
        assert!(!args.iter().any(|arg| arg.contains("password=")));
    }
}
