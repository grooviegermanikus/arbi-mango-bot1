use env_logger::Env;

mod services;
mod mango;
mod coordinator;

#[tokio::main]
async fn main() {
    // env_logger::Builder::from_env(Env::default().default_filter_or("debug,reqwest=info")).init();
    env_logger::Builder::from_env(Env::default().default_filter_or("arbi_mango_bot1=info")).init();

    coordinator::run_coordinator_service().await;

    // rpc_slot().await;
}
