use crate::{MasshConfig, SshAuth, SshClient, SshOutput};
use anyhow::Result;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use threadpool::ThreadPool;

/// Unique string identifier (`username@ip_address:port`) for a `MasshClient` target host.
pub type MasshHost = String;

/// Receiving half of a `std::sync::mpsc::channel` which receives exactly 1 message per host.
pub type MasshReceiver<T> = Receiver<(MasshHost, Result<T>)>;

/// SSH client to run commands on multiple hosts in parallel.
///
/// ## Public API Overview
///
/// Construct a new `MasshClient`:
/// - [`MasshClient::from`]
///
/// Run commands with this `MasshClient`:
/// - [`MasshClient::execute`]
/// - [`MasshClient::scp_download`]
/// - [`MasshClient::scp_upload`]
///
/// ## Example
///
/// ```no_run
/// use massh::{MasshConfig, MasshClient};
///
/// // Construct a new `MasshClient` from a YAML configuration file.
/// let yaml = std::fs::read_to_string("massh.yaml").unwrap();
/// let config = MasshConfig::from_json(&yaml).unwrap();
/// let massh = MasshClient::from(&config);
///
/// // Run a command on all the configured host.
/// let rx = massh.execute("docker ps");
///
/// // Receive the result of the command for each host and print its output.
/// while let Ok((host, result)) = rx.recv() {
///     let output = result.unwrap();
///     println!("host: {}", host);
///     println!("status: {}", output.exit_status);
///     println!("stdout: {}", String::from_utf8(output.stdout).unwrap());
///     println!("stderr: {}", String::from_utf8(output.stderr).unwrap());
/// }
/// ```
pub struct MasshClient {
    clients: HashMap<MasshHost, Arc<Mutex<SshClient>>>,
    pool: Option<ThreadPool>,
}

impl MasshClient {
    /// Constructs a new `MasshClient` from the specified configuration file.
    ///
    /// See [`MasshConfig`] for more details.
    ///
    /// ## Example
    /// ```no_run
    /// use massh::{MasshConfig, MasshClient};
    ///
    /// let yaml = std::fs::read_to_string("massh.yaml").unwrap();
    /// let config = MasshConfig::from_json(&yaml).unwrap();
    /// let massh = MasshClient::from(&config);
    /// ```
    pub fn from(config: &MasshConfig) -> Self {
        // Configure the internal SSH clients.
        let mut clients = HashMap::new();
        config.hosts.iter().for_each(|host| {
            let addr = host.addr;
            let auth = match &host.auth {
                Some(auth) => auth,
                None => &config.default_auth,
            };
            let port = match host.port {
                Some(port) => port,
                None => config.default_port,
            };
            let user = match &host.user {
                Some(user) => user,
                None => &config.default_user,
            };

            let mut ssh = SshClient::from(user, (addr, port));
            match auth {
                SshAuth::Agent => ssh.set_auth_agent(),
                SshAuth::Password(password) => ssh.set_auth_password(password),
                SshAuth::Pubkey(path) => ssh.set_auth_pubkey(path),
            };
            ssh.set_timeout(config.timeout);

            let host = format!("{}@{}", ssh.get_user(), ssh.get_addr());
            clients.insert(host, Arc::new(Mutex::new(ssh)));
        });

        // Configure the internal thread pool if specified.
        let pool = if config.threads == 0 {
            None
        } else {
            Some(ThreadPool::new(config.threads as usize))
        };

        MasshClient { clients, pool }
    }

    /// Attempts to execute a command on the configured hosts.
    ///
    /// It returns a [`MasshReceiver`] that receives exactly 1 message per host.
    /// Each message contains the result of the operation.
    ///
    /// ## Example
    /// ```no_run
    /// let massh = MasshClient::from(&config);
    ///
    /// let rx = massh.execute("echo $PATH");
    ///
    /// while let Ok((host, result)) = rx.recv() {
    ///     println!("Command succeeded on {}? {}", host, result.is_ok());
    /// }
    /// ```
    pub fn execute(&self, command: impl Into<String>) -> MasshReceiver<SshOutput> {
        let command = command.into();

        // Create a multi-producer, single-consumer channel.
        let (tx, rx) = std::sync::mpsc::channel();

        // For each configured host...
        self.clients.iter().for_each(|(host, client)| {
            // Prepare a task closure responsible for sending the result of the operation.
            let (client, host, tx) = (client.clone(), host.clone(), tx.clone());
            let command = command.clone();
            let task_closure = move || {
                let mut client = client.lock();
                let result = client.execute(&command);
                let _ = tx.send((host, result));
            };

            // Execute the task closure in the thread pool or spawn it in its own thread.
            if let Some(pool) = &self.pool {
                pool.execute(task_closure)
            } else {
                std::thread::spawn(task_closure);
            }
        });

        // Return the receiving half of the channel.
        rx
    }

    /// Attempts to download a file from the configured hosts.
    ///
    /// It returns a [`MasshReceiver`] that receives exactly 1 message per host.
    /// Each message contains the result of the operation.
    ///
    /// Note that the downloaded file names are of the form "user@ip-address:port".
    ///
    /// ## Example
    /// ```no_run
    /// let massh = MasshClient::from(&config);
    ///
    /// let rx = massh.scp_download("remote.txt", "local_dir");
    ///
    /// while let Ok((host, result)) = rx.recv() {
    ///     println!("Download succeeded on {}? {}", host, result.is_ok());
    /// }
    /// ```
    pub fn scp_download<P>(&self, remote_path: P, local_path: P) -> MasshReceiver<()>
    where
        P: Into<PathBuf>,
    {
        let (remote_path, local_path) = (remote_path.into(), local_path.into());

        // Create a multi-producer, single-consumer channel.
        let (tx, rx) = std::sync::mpsc::channel();

        // For each configured host...
        self.clients.iter().for_each(|(host, client)| {
            // Prepare a task closure responsible for sending the result of the operation.
            let (client, host, tx) = (client.clone(), host.clone(), tx.clone());
            let (remote_path, mut local_path) = (remote_path.clone(), local_path.clone());
            let task_closure = move || {
                let mut client = client.lock();
                local_path.push(&host);
                let result = client.scp_download(remote_path, local_path);
                let _ = tx.send((host, result));
            };

            // Execute the task closure in the thread pool or spawn it in its own thread.
            if let Some(pool) = &self.pool {
                pool.execute(task_closure)
            } else {
                std::thread::spawn(task_closure);
            }
        });

        // Return the receiving half of the channel.
        rx
    }

    /// Attempts to upload a file to the configured hosts.
    ///
    /// It returns a [`MasshReceiver`] that receives exactly 1 message per host.
    /// Each message contains the result of the operation.
    ///
    /// ## Example
    /// ```no_run
    /// let massh = MasshClient::from(&config);
    ///
    /// let rx = massh.scp_upload("local.txt", "remote.txt");
    ///
    /// while let Ok((host, result)) = rx.recv() {
    ///     println!("Upload succeeded on {}? {}", host, result.is_ok());
    /// }
    /// ```
    pub fn scp_upload<P>(&self, local_path: P, remote_path: P) -> MasshReceiver<()>
    where
        P: Into<PathBuf>,
    {
        let (local_path, remote_path) = (local_path.into(), remote_path.into());

        // Create a multi-producer, single-consumer channel.
        let (tx, rx) = std::sync::mpsc::channel();

        // For each configured host...
        self.clients.iter().for_each(|(host, client)| {
            // Prepare a task closure responsible for sending the result of the operation.
            let (client, host, tx) = (client.clone(), host.clone(), tx.clone());
            let (local_path, remote_path) = (local_path.clone(), remote_path.clone());
            let task_closure = move || {
                let mut client = client.lock();
                let result = client.scp_upload(local_path, remote_path);
                let _ = tx.send((host, result));
            };

            // Execute the task closure in the thread pool or spawn it in its own thread.
            if let Some(pool) = &self.pool {
                pool.execute(task_closure)
            } else {
                std::thread::spawn(task_closure);
            }
        });

        // Return the receiving half of the channel.
        rx
    }
}
