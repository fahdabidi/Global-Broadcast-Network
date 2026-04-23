use gbn_bridge_cli::{run_deployment_entrypoint, DeploymentRole};

fn main() {
    run_deployment_entrypoint(DeploymentRole::Publisher);
}
