use crate::SshAuth;
use anyhow::Result;
use serde::{Deserialize, Deserializer};
use std::net::{IpAddr, ToSocketAddrs};

/// Configuration for a `MasshClient` target host.
#[derive(Deserialize)]
#[serde(from = "MasshHostConfigEnum")]
pub struct MasshHostConfig {
    /// IP address, either IPv4 or IPv6.
    pub addr: IpAddr,
    /// Optional authentication method to override the default.
    pub auth: Option<SshAuth>,
    /// Optional port number to override the default.
    pub port: Option<u16>,
    /// Optional username to override the default.
    pub user: Option<String>,
}

/// Configuration for a `MasshClient`.
#[derive(Deserialize)]
pub struct MasshConfig {
    /// Default authentication method for all configured hosts.
    pub default_auth: SshAuth,
    /// Default port number for all configured hosts.
    pub default_port: u16,
    /// Default username for all configured hosts.
    pub default_user: String,
    /// Number of threads in the internal thread pool.
    ///
    /// A value of zero signifies 1 thread per configured host.
    pub threads: u64,
    /// Timeout, in milliseconds, for blocking functions.
    ///
    /// A value of zero signifies no timeout.
    pub timeout: u64,
    /// List of configured hosts.
    ///
    /// Internally, every host is uniquely identified by the tuple (username, ip_address, port).
    /// Duplicates are discarded.
    pub hosts: Vec<MasshHostConfig>,
}

impl MasshConfig {
    /// Attempts to construct a new `MasshConfig` from a JSON string.
    ///
    /// ## Simple Example
    ///
    /// ```json
    /// {
    ///   "default_auth": "agent",
    ///   "default_port": 22,
    ///   "default_user": "username",
    ///   "threads": 0,
    ///   "timeout": 0,
    ///   "hosts": [
    ///     "1.1.1.1",
    ///     "2.2.2.2",
    ///     "3.3.3.3"
    ///   ]
    /// }
    /// ```
    ///
    /// ## Complex Example
    ///
    /// ```json
    /// {
    ///   "default_auth": {
    ///     "pubkey": "/home/username/.ssh/id_rsa"
    ///   },
    ///   "default_port": 22,
    ///   "default_user": "username",
    ///   "threads": 2,
    ///   "timeout": 5000,
    ///   "hosts": [
    ///     "1.1.1.1",
    ///     "other-user-1@2.2.2.2",
    ///     "other-user-2@3.3.3.3:20022",
    ///     {
    ///       "addr": "4.4.4.4"
    ///     },
    ///     {
    ///       "addr": "5.5.5.5",
    ///       "auth": "agent",
    ///       "port": null,
    ///       "user": null
    ///     },
    ///     {
    ///       "addr": "6.6.6.6",
    ///       "auth": {
    ///         "password": "special-password"
    ///       },
    ///       "user": "other-user-3"
    ///     }
    ///   ]
    /// }
    /// ```
    ///
    /// ## Usage
    ///
    /// ```no_run
    /// use massh::MasshConfig;
    ///
    /// let json = std::fs::read_to_string("massh.json").unwrap();
    /// let config = MasshConfig::from_json(&json).unwrap();
    /// ```
    pub fn from_json(json: &str) -> Result<Self> {
        let config: MasshConfig = serde_json::from_str(json)?;
        Ok(config)
    }

    /// Attempts to construct a new `MasshConfig` from a YAML string.
    ///
    /// ## Simple Example
    ///
    /// ```yaml
    /// ---
    /// default_auth: agent
    /// default_port: 22
    /// default_user: username
    /// threads: 0
    /// timeout: 0
    /// hosts:
    ///   - 1.1.1.1
    ///   - 2.2.2.2
    ///   - 3.3.3.3
    /// ```
    ///
    /// ## Complex Example
    ///
    /// ```yaml
    /// ---
    /// default_auth:
    ///   pubkey: /home/username/.ssh/id_rsa
    /// default_port: 22
    /// default_user: username
    /// threads: 2
    /// timeout: 5000
    /// hosts:
    ///   - 1.1.1.1
    ///   - other-user-1@2.2.2.2
    ///   - other-user-2@3.3.3.3:20022
    ///   - addr: 4.4.4.4
    ///   - addr: 5.5.5.5
    ///     auth: agent
    ///     port: ~
    ///     user: ~
    ///   - addr: 6.6.6.6
    ///     auth:
    ///       password: special-password
    ///     user: other-user-3
    /// ```
    ///
    /// ## Usage
    ///
    /// ```no_run
    /// use massh::MasshConfig;
    ///
    /// let yaml = std::fs::read_to_string("massh.yaml").unwrap();
    /// let config = MasshConfig::from_yaml(&yaml).unwrap();
    /// ```
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        let config: MasshConfig = serde_yaml::from_str(yaml)?;
        Ok(config)
    }
}

// The rest of this file consists of private items to help deserialize
// a `MasshHostConfig` struct from either a map or a string.

#[derive(Deserialize)]
struct InnerMasshHostConfig {
    addr: IpAddr,
    auth: Option<SshAuth>,
    port: Option<u16>,
    user: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum MasshHostConfigEnum {
    FromMap(InnerMasshHostConfig),
    #[serde(deserialize_with = "deserialize_host_from_str")]
    FromStr(InnerMasshHostConfig),
}

impl From<MasshHostConfigEnum> for MasshHostConfig {
    fn from(e: MasshHostConfigEnum) -> MasshHostConfig {
        let inner = match e {
            MasshHostConfigEnum::FromMap(inner) => inner,
            MasshHostConfigEnum::FromStr(inner) => inner,
        };
        MasshHostConfig {
            addr: inner.addr,
            auth: inner.auth,
            port: inner.port,
            user: inner.user,
        }
    }
}

fn deserialize_host_from_str<'de, D>(deserializer: D) -> Result<InnerMasshHostConfig>
where
    D: Deserializer<'de>,
{
    let e = anyhow::anyhow!("String deserialization failed");
    let value = match String::deserialize(deserializer) {
        Ok(value) => value,
        Err(_) => return Err(e),
    };

    let (user, value) = match value.split_once('@') {
        Some((left, right)) => (Some(left.to_owned()), right.to_owned()),
        None => (None, value),
    };

    let (mut addrs, no_port) = if let Ok(addrs) = value.to_socket_addrs() {
        (addrs, false)
    } else if let Ok(addrs) = format!("{}:22", value).to_socket_addrs() {
        (addrs, true)
    } else {
        return Err(e);
    };

    let socket = match addrs.next() {
        Some(socket) => socket,
        None => return Err(e),
    };

    let addr = socket.ip();
    let port = if no_port { None } else { Some(socket.port()) };

    Ok(InnerMasshHostConfig {
        addr,
        auth: None,
        port,
        user,
    })
}
