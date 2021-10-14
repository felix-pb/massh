use crate::Auth;
use anyhow::Result;
use ssh2::Session;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Output of a command executed over SSH.
pub struct CommandOutput {
    /// Exit status
    pub exit_status: i32,
    /// Standard error
    pub stderr: Vec<u8>,
    /// Standard output
    pub stdout: Vec<u8>,
}

/// SSH client to run commands on a single host.
///
/// ## Public API Overview
///
/// Construct a new `SshClient`:
/// - [`SshClient::from`]
/// - [`SshClient::try_from`]
///
/// Configure this `SshClient`:
/// - [`SshClient::set_auth_agent`]
/// - [`SshClient::set_auth_password`]
/// - [`SshClient::set_auth_pubkey`]
/// - [`SshClient::set_timeout`]
///
/// Inspect this `SshClient`:
/// - [`SshClient::get_addr`]
/// - [`SshClient::get_auth`]
/// - [`SshClient::get_timeout`]
/// - [`SshClient::get_user`]
/// - [`SshClient::is_connected`]
///
/// Run commands with this `SshClient`:
/// - [`SshClient::execute`]
/// - [`SshClient::scp_download`]
/// - [`SshClient::scp_upload`]
///
/// There are also methods to manage the internal authenticated session of this `SshClient`:
/// - [`SshClient::connect`]
/// - [`SshClient::disconnect`]
///
/// However, it's typically not necessary to call them because they are invoked lazily when needed.
///
/// ## Example
///
/// ```no_run
/// use massh::SshClient;
/// use std::net::Ipv4Addr;
///
/// // Construct a new SSH client for "username@localhost:22".
/// let mut ssh = SshClient::from("username", (Ipv4Addr::LOCALHOST, 22));
///
/// // Configure the client.
/// ssh.set_auth_password("top-secret").set_timeout(5000);
///
/// // Download a file.
/// ssh.scp_download("remote.txt", "local.txt").unwrap();
///
/// // Upload a file.
/// ssh.scp_upload("local.txt", "remote-copy.txt").unwrap();
///
/// // Run a command and print its output.
/// let output = ssh.execute("cat remote-copy.txt").unwrap();
///
/// println!("status: {}", output.exit_status);
/// println!("stdout: {}", String::from_utf8(output.stdout).unwrap());
/// println!("stderr: {}", String::from_utf8(output.stderr).unwrap());
/// ```
pub struct SshClient {
    addr: SocketAddr,
    auth: Auth,
    session: Option<Session>,
    timeout: u64,
    user: String,
}

impl SshClient {
    /// Constructs a new `SshClient` for the specified host's username and address.
    ///
    /// By default, the client uses agent authentication and has no timeout.
    ///
    /// ## Example
    /// ```no_run
    /// use massh::SshClient;
    /// use std::net::Ipv4Addr;
    ///
    /// let ssh = SshClient::from("username", (Ipv4Addr::LOCALHOST, 22));
    /// ```
    pub fn from(user: impl Into<String>, addr: impl Into<SocketAddr>) -> Self {
        Self {
            addr: addr.into(),
            auth: Auth::Agent,
            session: None,
            timeout: 0,
            user: user.into(),
        }
    }

    /// Attempts to construct a new `SshClient` for the specified host's username and address.
    ///
    /// Unlike [`SshClient::from`], it can resolve a hostname to an address.
    /// However, it's fallible and therefore returns a `Result`.
    ///
    /// By default, the client uses agent authentication and has no timeout.
    ///
    /// ## Example
    /// ```no_run
    /// use massh::SshClient;
    ///
    /// let ssh1 = SshClient::try_from("username", "127.0.0.1:22").unwrap();
    /// let ssh2 = SshClient::try_from("username", "localhost:22").unwrap();
    /// let ssh3 = SshClient::try_from("ec2-user", "xyz.compute.amazonaws.com:22").unwrap();
    /// ```
    pub fn try_from(user: impl Into<String>, addr: impl ToSocketAddrs) -> Result<Self> {
        if let Some(addr) = addr.to_socket_addrs()?.next() {
            Ok(Self {
                addr,
                auth: Auth::Agent,
                session: None,
                timeout: 0,
                user: user.into(),
            })
        } else {
            Err(anyhow::anyhow!("Socket address conversion failed"))
        }
    }

    /// Configures this `SshClient` to perform agent authentication using
    /// the first public key found in an SSH agent.
    ///
    /// This is the default.
    ///
    /// ## Example
    /// ```no_run
    /// let mut ssh = SshClient::from("username", (Ipv4Addr::LOCALHOST, 22));
    ///
    /// // Note that this does nothing since it's the default.
    /// ssh.set_auth_agent();
    /// ```
    pub fn set_auth_agent(&mut self) -> &mut Self {
        self.auth = Auth::Agent;
        self
    }

    /// Configures this `SshClient` to perform basic password authentication.
    ///
    /// ## Example
    /// ```no_run
    /// let mut ssh = SshClient::from("username", (Ipv4Addr::LOCALHOST, 22));
    ///
    /// ssh.set_auth_password("top-secret");
    /// ```
    pub fn set_auth_password(&mut self, password: impl Into<String>) -> &mut Self {
        self.auth = Auth::Password(password.into());
        self
    }

    /// Configures this `SshClient` to perform public key authentication using
    /// a PEM encoded private key file stored on disk.
    ///
    /// ## Example
    /// ```no_run
    /// let mut ssh = SshClient::from("username", (Ipv4Addr::LOCALHOST, 22));
    ///
    /// ssh.set_auth_pubkey("/home/username/.ssh/id_rsa");
    /// ```
    pub fn set_auth_pubkey(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.auth = Auth::Pubkey(path.into());
        self
    }

    /// Configures this `SshClient` to use a timeout, in milliseconds, for blocking functions.
    ///
    /// A timeout of zero signifies no timeout. This is the default.
    ///
    /// ## Example
    /// ```no_run
    /// let mut ssh = SshClient::from("username", (Ipv4Addr::LOCALHOST, 22));
    ///
    /// // Set a timeout of 5 seconds.
    /// ssh.set_timeout(5000);
    /// ```
    pub fn set_timeout(&mut self, timeout_ms: u64) -> &mut Self {
        self.timeout = timeout_ms;
        self
    }

    /// Returns the address of this `SshClient`'s configured host.
    pub fn get_addr(&self) -> SocketAddr {
        self.addr
    }

    /// Returns the authentication method of this `SshClient`'s configured host.
    pub fn get_auth(&self) -> &Auth {
        &self.auth
    }

    /// Returns the timeout, in milliseconds, of this `SshClient`'s configured host.
    ///
    /// A timeout of zero signifies no timeout.
    pub fn get_timeout(&self) -> u64 {
        self.timeout
    }

    /// Returns the username of this `SshClient`'s configured host.
    pub fn get_user(&self) -> &str {
        &self.user
    }

    /// Returns whether this `SshClient` has established an authenticated session
    /// with the configured host.
    pub fn is_connected(&self) -> bool {
        self.session.is_some()
    }

    /// Attempts to execute a command on the configured host.
    ///
    /// Note that this method implicitly calls [`SshClient::connect`] if no session was
    /// established prior. Otherwise, it reuses the cached session.
    ///
    /// If successful, it returns a [`CommandOutput`] containing the exit status, standard output,
    /// and standard error of the command.
    ///
    /// ## Example
    /// ```no_run
    /// let mut ssh = SshClient::from("username", (Ipv4Addr::LOCALHOST, 22));
    ///
    /// let output = ssh.execute("echo $PATH").unwrap();
    ///
    /// println!("status: {}", output.exit_status);
    /// println!("stdout: {}", String::from_utf8(output.stdout).unwrap());
    /// println!("stderr: {}", String::from_utf8(output.stderr).unwrap());
    /// ```
    pub fn execute(&mut self, command: &str) -> Result<CommandOutput> {
        // Establish authenticated SSH session.
        if self.session.is_none() {
            self.connect()?;
        }
        let session = self.session.as_ref().unwrap();

        // Open channel and stderr stream.
        let mut channel = session.channel_session()?;
        let mut stderr_stream = channel.stderr();

        // Execute command.
        channel.exec(command)?;

        // Read stdout into buffer.
        let mut stdout = Vec::new();
        channel.read_to_end(&mut stdout)?;

        // Read stderr into buffer.
        let mut stderr = Vec::new();
        stderr_stream.read_to_end(&mut stderr)?;

        // Close channel and retrieve exit status.
        channel.wait_close()?;
        let exit_status = channel.exit_status()?;

        // Return successfully.
        Ok(CommandOutput {
            exit_status,
            stdout,
            stderr,
        })
    }

    /// Attempts to download a file from the configured host.
    ///
    /// Note that this method implicitly calls [`SshClient::connect`] if no session was
    /// established prior. Otherwise, it reuses the cached session.
    ///
    /// ## Example
    /// ```no_run
    /// let mut ssh = SshClient::from("username", (Ipv4Addr::LOCALHOST, 22));
    ///
    /// if ssh.scp_download("remote.txt", "local.txt").is_ok() {
    ///     println!("download worked!");
    /// }
    /// ```
    pub fn scp_download<P: AsRef<Path>>(&mut self, remote_path: P, local_path: P) -> Result<()> {
        // Establish authenticated SSH session.
        if self.session.is_none() {
            self.connect()?;
        }
        let session = self.session.as_ref().unwrap();

        // Open channel.
        let (mut channel, _) = session.scp_recv(remote_path.as_ref())?;

        // Read remote file into buffer.
        let mut buffer = Vec::new();
        channel.read_to_end(&mut buffer)?;

        // Write buffer to local file.
        std::fs::write(local_path, &buffer)?;

        // Close channel.
        channel.send_eof()?;
        channel.wait_eof()?;
        channel.close()?;
        channel.wait_close()?;

        // Return successfully.
        Ok(())
    }

    /// Attempts to upload a file to the configured host.
    ///
    /// Note that this method implicitly calls [`SshClient::connect`] if no session was
    /// established prior. Otherwise, it reuses the cached session.
    ///
    /// ## Example
    /// ```no_run
    /// let mut ssh = SshClient::from("username", (Ipv4Addr::LOCALHOST, 22));
    ///
    /// if ssh.scp_upload("local.txt", "remote.txt").is_ok() {
    ///     println!("upload worked!");
    /// }
    /// ```
    pub fn scp_upload<P: AsRef<Path>>(&mut self, local_path: P, remote_path: P) -> Result<()> {
        // Establish authenticated SSH session.
        if self.session.is_none() {
            self.connect()?;
        }
        let session = self.session.as_ref().unwrap();

        // Read local file into buffer.
        let buffer = std::fs::read(local_path)?;
        let size = buffer.len() as u64;

        // Open channel.
        let mut channel = session.scp_send(remote_path.as_ref(), 0o644, size, None)?;

        // Write buffer to remote file.
        channel.write_all(&buffer)?;

        // Close channel.
        channel.send_eof()?;
        channel.wait_eof()?;
        channel.close()?;
        channel.wait_close()?;

        // Return successfully.
        Ok(())
    }

    /// Attempts to establish an authenticated session between this `SshClient`
    /// and the configured host.
    ///
    /// If successful, the session is cached internally by the client and is reused when
    /// running multiple commands with [`SshClient::execute`], [`SshClient::scp_download`],
    /// or [`SshClient::scp_upload`].
    ///
    /// Note that it's not strictly necessary to call this method because the 3 methods
    /// mentioned above will invoke it lazily if no session was established prior.
    ///
    /// Finally, if the first session succeeds but the second session fails,
    /// the first session will remain cached internally by the client. If the second
    /// session succeeds, it replaces the first session (which is dropped).
    ///
    /// ## Example
    /// ```no_run
    /// let mut ssh = SshClient::from("username", (Ipv4Addr::LOCALHOST, 22));
    ///
    /// if ssh.connect().is_ok() {
    ///     println!("agent authentication worked!");
    /// }
    ///
    /// if ssh.set_auth_password("top-secret").connect().is_ok() {
    ///     println!("password authentication also worked!");
    /// }
    /// ```
    pub fn connect(&mut self) -> Result<&mut Self> {
        // Initialize new SSH session.
        let mut session = Session::new()?;

        // Open a TCP connection to the configured host and attach it to the SSH session.
        let tcp_stream = if self.timeout == 0 {
            // If timeout is zero, don't set a timeout.
            TcpStream::connect(&self.addr)?
        } else {
            // If timeout is non-zero, set a timeout on both the SSH session and the TCP stream.
            session.set_timeout(self.timeout as u32);
            TcpStream::connect_timeout(&self.addr, Duration::from_millis(self.timeout))?
        };
        session.set_tcp_stream(tcp_stream);

        // Perform SSH handshake.
        session.handshake()?;

        // Perform SSH authentication based on selected method.
        match &self.auth {
            Auth::Agent => session.userauth_agent(&self.user)?,
            Auth::Password(password) => session.userauth_password(&self.user, password)?,
            Auth::Pubkey(path) => session.userauth_pubkey_file(&self.user, None, path, None)?,
        }

        // Confirm that the session is authenticated.
        if !session.authenticated() {
            return Err(anyhow::anyhow!("Authentication failed"));
        }

        // Cache authenticated session and return successfully.
        self.session = Some(session);
        Ok(self)
    }

    /// Drops the authenticated session between this `SshClient` and the configured host,
    /// or does nothing if no session was established prior.
    ///
    /// Note that it's not strictly necessary to call this method because it is invoked
    /// implicitly when the client itself is dropped.
    ///
    /// ## Example
    /// ```no_run
    /// let mut ssh = SshClient::from("username", (Ipv4Addr::LOCALHOST, 22));
    ///
    /// ssh.connect().unwrap();
    /// // Do some stuff...
    /// ssh.disconnect();
    /// ```
    pub fn disconnect(&mut self) -> &mut Self {
        self.session = None;
        self
    }
}
