use log::*;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{clock::DEFAULT_MS_PER_SLOT, commitment_config::CommitmentConfig, hash::Hash};
use std::{
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::{
    spawn,
    time::{sleep, timeout},
};

const RETRY: Duration = Duration::from_millis(5 * DEFAULT_MS_PER_SLOT);
const TIMEOUT: Duration = Duration::from_secs(10);

async fn poll_loop(client: Arc<RpcClient>, blockhash: Arc<RwLock<Hash>>) {
    loop {
        match timeout(TIMEOUT, client.get_latest_blockhash()).await {
            Ok(Ok(new_blockhash)) => {
                let mut shared_blockhash = blockhash.write().unwrap();
                if new_blockhash != *shared_blockhash {
                    debug!("blockhash update {:?}", new_blockhash);
                    *shared_blockhash = new_blockhash;
                }
            }
            Ok(Err(e)) => {
                error!("error reading blockhash err={e:?}. sleep for {TIMEOUT:?}");
                sleep(TIMEOUT - RETRY).await;
            }
            Err(e) => {
                error!("timeout reading blockhash err={e:?}");
            }
        }

        // retry every few slots
        sleep(RETRY).await;
    }
}

pub async fn start_blockhash_service(rpc_url: String) -> Arc<RwLock<Hash>> {
    let rpc_client = Arc::new(RpcClient::new_with_commitment(
        rpc_url,
        CommitmentConfig::confirmed(),
    ));

    // get the first blockhash
    let blockhash = Arc::new(RwLock::new(
        rpc_client
            .get_latest_blockhash()
            .await
            .expect("fetch initial blockhash"),
    ));

    // launch task
    let _join_hdl = {
        // create a thread-local reference to blockhash
        let blockhash_c = blockhash.clone();
        spawn(async move { poll_loop(rpc_client, blockhash_c).await })
    };

    blockhash
}
