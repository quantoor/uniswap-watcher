use anyhow::Result;
use ethers::types::TransactionReceipt;
use ethers::{
    contract::{abigen, Contract, LogMeta},
    core::types::ValueOrArray,
    providers::{Http, Middleware, Provider, StreamExt, Ws},
    types::{transaction, Transaction, H256, U256},
    utils::format_units,
};
use std::{error::Error, str::FromStr, sync::Arc};
use uniswap_watcher::{compute_gas_fee_eth, POOL_ADDRESS, RPC_URL_HTTP, RPC_URL_WS};

abigen!(
    AggregatorInterface,
    r#"[
        event Swap(address indexed sender, address indexed recipient, int256 amount0, int256 amount1, uint160 sqrtPriceX96, uint128 liquidity, int24 tick)
    ]"#,
);

#[tokio::main]
async fn main() -> Result<()> {
    let client = Provider::<Http>::try_from(RPC_URL_HTTP).unwrap();

    let ws_client = Provider::<Ws>::connect(RPC_URL_WS).await.unwrap();
    let ws_client = Arc::new(ws_client);

    let event = Contract::event_of_type::<SwapFilter>(ws_client)
        .address(ValueOrArray::Array(vec![POOL_ADDRESS.parse()?]));

    let mut stream = event.subscribe_with_meta().await?;
    loop {
        while let Some(Ok((log, meta))) = stream.next().await {
            let log: SwapFilter = log;
            let meta: LogMeta = meta;
            // println!("{log:?}");
            // println!("{meta:?}");
            // SwapFilter { sender: 0x0b8a49d816cc709b6eadb09498030ae3416b66dc, recipient: 0x5777d92f208679db4b9778590fa3cab3ac9e2168, amount_0: -204560386743, amount_1: 59650147611384101454, sqrt_price_x96: 1352811659334603508045816700931506, liquidity: 10626721212312255156, tick: 194917 }
            // LogMeta { address: 0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640, block_number: 19334649, block_hash: 0x810a72a1a782e3d5c73ef8b4f3ba83bc0ebf15603e3c01bcc6b7156acc332279, transaction_hash: 0x8705731e8d54ca8de0daf4d7a09398e308a7c7fb06f8f26122b95fcb8d2d51c7, transaction_index: 102, log_index: 34 }

            let tx = client
                .get_transaction_receipt(meta.transaction_hash)
                .await
                .unwrap() // todo handle this
                .unwrap(); // todo handle this

            // println!("{tx:?}");

            _ = compute_gas_fee_eth(&tx).await;
            //Transaction { hash: 0xdcf71f94263712cd6471ed62f63b35c5f0b7da79c3cde2ed79ede08905046cef, nonce: 49, block_hash: Some(0xe5480c13daf5ece2059fd7ebb1a1c1f93e1e6c196d65af22e6f6870c2cf5b3a7), block_number: Some(19334841), transaction_index: Some(139), from: 0xdb1aeb6982734f29c8875cd09d835334e20839c1, to: Some(0xce16f69375520ab01377ce7b88f5ba8c48f8d666), value: 25000141853047277933, gas_price: Some(87798903854), gas: 420636, input: Bytes(0x846a1bc6...), v: 0, r: 9178740..., s: 573577789..., transaction_type: Some(2), access_list: Some(AccessList([])), max_priority_fee_per_gas: Some(44264641), max_fee_per_gas: Some(129859474761), chain_id: Some(1), other: OtherFields { inner: {"yParity": String("0x0")} } }
        }
    }

    Ok(())
}
