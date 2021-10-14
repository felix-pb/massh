use anyhow::Result;
use serde::{Deserialize, Deserializer};
use std::net::{IpAddr, ToSocketAddrs};
use std::path::PathBuf;

/// Authentication method.
#[derive(Deserialize)]
pub enum Auth {
    /// Agent authentication using the first public key found in an SSH agent.
    #[serde(rename = "agent")]
    Agent,
    /// Basic password authentication.
    #[serde(rename = "password")]
    Password(String),
    /// Public key authentication using a PEM encoded private key file stored on disk.
    #[serde(rename = "pubkey")]
    Pubkey(PathBuf),
}

/// One of the configured hosts in a `ConfigFile`.
#[derive(Deserialize)]
#[serde(from = "ConfigHostEnum")]
pub struct ConfigHost {
    /// IP address.
    pub addr: IpAddr,
    /// Optional authentication method to override the default.
    pub auth: Option<Auth>,
    /// Optional port number to override the default.
    pub port: Option<u16>,
    /// Optional username to override the default.
    pub user: Option<String>,
}

/// Configuration file to build a `MasshClient`.
#[derive(Deserialize)]
pub struct ConfigFile {
    /// Default authentication method for the configured hosts.
    pub default_auth: Auth,
    /// Default port number for the configured hosts.
    pub default_port: u16,
    /// Default username for the configured hosts.
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
    /// Every host is uniquely identified by its username, IP address and port number.
    /// Duplicates are discarded.
    pub hosts: Vec<ConfigHost>,
}

impl ConfigFile {
    /// Attempts to construct a new `ConfigFile` from a JSON string.
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
    /// use massh::ConfigFile;
    ///
    /// let json = std::fs::read_to_string("massh.json").unwrap();
    /// let config = ConfigFile::from_json(&json).unwrap();
    /// ```
    pub fn from_json(json: &str) -> Result<Self> {
        Err(anyhow::anyhow!(json.to_owned()))
    }

    /// Attempts to construct a new `ConfigFile` from a YAML string.
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
    /// use massh::ConfigFile;
    ///
    /// let yaml = std::fs::read_to_string("massh.yaml").unwrap();
    /// let config = ConfigFile::from_json(&yaml).unwrap();
    /// ```
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        Err(anyhow::anyhow!(yaml.to_owned()))
    }
}

// The rest of this file consists of private items to help deserialize
// a `ConfigHost` struct from either a map or a string.

#[derive(Deserialize)]
struct InnerConfigHost {
    addr: IpAddr,
    auth: Option<Auth>,
    port: Option<u16>,
    user: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ConfigHostEnum {
    FromMap(InnerConfigHost),
    #[serde(deserialize_with = "deserialize_host_from_str")]
    FromStr(InnerConfigHost),
}

impl From<ConfigHostEnum> for ConfigHost {
    fn from(e: ConfigHostEnum) -> ConfigHost {
        let inner = match e {
            ConfigHostEnum::FromMap(inner) => inner,
            ConfigHostEnum::FromStr(inner) => inner,
        };
        ConfigHost {
            addr: inner.addr,
            auth: inner.auth,
            port: inner.port,
            user: inner.user,
        }
    }
}

fn deserialize_host_from_str<'de, D>(deserializer: D) -> Result<InnerConfigHost>
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

    Ok(InnerConfigHost {
        addr,
        auth: None,
        port,
        user,
    })
}
