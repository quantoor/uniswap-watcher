# Uniswap Watcher

<!--
TODO 
decode price for every tx and store in db
full curl command to get tx
-->

This application provides the following functionalities:

- It subscribes to swap events occurring on the WETH/USDC-500 pool on UniswapV3 on Ethereum mainnet.
- For every new event, it fetches the corresponding transaction data to get the tx fee in USDT and the swap price, and stores them in a database.
- It runs a web server that exposes endpoints to query the tx fee in USDT and swap price for given transaction hashes.
  - If the tx hash exists in the database, the corresponding info is fetched from it and returned.
  - Otherwise, the tx fee and swap price are computed, stored in db and then returned.

### Setup
Download the repository and then cd into it:
```
git clone git@github.com:quantoor/uniswap-watcher.git
cd uniswap-watcher
```

### Run the tests
Make sure to have rust installed: https://doc.rust-lang.org/book/ch01-01-installation.html

To run the tests:
```
cargo test
```

### Run the application
Make sure to have docker-compose installed: https://docs.docker.com/compose/install/

To run the application:
```
docker-compose up
```
Check that the application is reachable:
```
curl http://127.0.0.1:8080
```

### Send requests
Get tx fee:
```
curl http://127.0.0.1:8080/tx_fee
```

### References
- Zero to Production In Rust: An introduction to backend development in Rust
by Luca Palmieri
