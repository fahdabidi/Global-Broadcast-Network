use std::{env, thread, time::Duration};

#[derive(Clone, Copy, Debug)]
pub enum DeploymentRole {
    Publisher,
    ExitBridge,
    HostCreator,
    CreatorClient,
}

impl DeploymentRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Publisher => "publisher",
            Self::ExitBridge => "exit-bridge",
            Self::HostCreator => "host-creator",
            Self::CreatorClient => "creator-client",
        }
    }

    fn default_node_id(self) -> &'static str {
        match self {
            Self::Publisher => "publisher-authority",
            Self::ExitBridge => "exit-bridge",
            Self::HostCreator => "host-creator",
            Self::CreatorClient => "creator",
        }
    }
}

#[derive(Debug)]
pub struct DeploymentConfig {
    pub role: DeploymentRole,
    pub node_id: String,
    pub stack_name: String,
    pub publisher_url: String,
    pub publisher_bind_addr: String,
    pub udp_punch_port: u16,
    pub batch_window_ms: u64,
}

impl DeploymentConfig {
    pub fn from_env(role: DeploymentRole) -> Result<Self, String> {
        Ok(Self {
            role,
            node_id: env_or("GBN_BRIDGE_NODE_ID", role.default_node_id()),
            stack_name: env_or("GBN_BRIDGE_STACK_NAME", "gbn-bridge-phase2-local"),
            publisher_url: env_or("GBN_BRIDGE_PUBLISHER_URL", "http://127.0.0.1:8080"),
            publisher_bind_addr: env_or("GBN_BRIDGE_PUBLISHER_BIND_ADDR", "0.0.0.0:8080"),
            udp_punch_port: parse_env_u16("GBN_BRIDGE_PUNCH_PORT", 443)?,
            batch_window_ms: parse_env_u64("GBN_BRIDGE_BATCH_WINDOW_MS", 500)?,
        })
    }

    pub fn summary(&self) -> String {
        format!(
            "role={} node_id={} stack={} publisher_url={} publisher_bind_addr={} udp_punch_port={} batch_window_ms={}",
            self.role.as_str(),
            self.node_id,
            self.stack_name,
            self.publisher_url,
            self.publisher_bind_addr,
            self.udp_punch_port,
            self.batch_window_ms
        )
    }
}

pub fn run_deployment_entrypoint(role: DeploymentRole) {
    let config = match DeploymentConfig::from_env(role) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("configuration error: {err}");
            std::process::exit(2);
        }
    };

    let args: Vec<String> = env::args().collect();
    let serve = args.iter().any(|arg| arg == "--serve")
        || env::var("GBN_BRIDGE_RUN_MODE")
            .map(|mode| mode == "serve")
            .unwrap_or(false);

    println!("{}", config.summary());

    if !serve {
        println!("configuration check complete; use --serve to keep the deployment process alive");
        return;
    }

    println!(
        "{} deployment placeholder running; network protocol service wiring remains V2-local until the AWS runtime validation cut",
        config.role.as_str()
    );

    loop {
        thread::sleep(Duration::from_secs(60));
    }
}

fn env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

fn parse_env_u16(key: &str, default: u16) -> Result<u16, String> {
    match env::var(key) {
        Ok(value) => value
            .parse::<u16>()
            .map_err(|_| format!("{key} must be a valid u16, got {value:?}")),
        Err(_) => Ok(default),
    }
}

fn parse_env_u64(key: &str, default: u64) -> Result<u64, String> {
    match env::var(key) {
        Ok(value) => value
            .parse::<u64>()
            .map_err(|_| format!("{key} must be a valid u64, got {value:?}")),
        Err(_) => Ok(default),
    }
}
