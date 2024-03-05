# Uniswap Watcher

This application provides the following functionalities:

- It subscribes to swap events occurring on the WETH/USDC-500 pool on UniswapV3 on Ethereum mainnet.
- For every new event, it fetches the corresponding transaction data to get the tx fee in USDT and stores it in a database.
It also computes and logs the swap price from the log amounts.
- It runs a web server that exposes endpoints to query the tx fee in USDT for given transaction hashes.
  - If the tx hash exists in the database, the corresponding info is fetched from it and returned.
  - Otherwise, the tx fee is computed, stored in db and then returned.

### Setup
Download the repository and then cd into it:
```
git clone git@github.com:quantoor/uniswap-watcher.git
cd uniswap-watcher
```
All following commands should be executed inside the root folder `/uniswap-watcher`

### Run the tests
Make sure to have rust installed: https://doc.rust-lang.org/book/ch01-01-installation.html

Make sure that local port 5432 is not being used, then start an instance of Postgres in Docker with:
```
./scripts/init_db.sh
```
To run the tests:
```
cargo test
```

### Run the application
Make sure to have docker-compose installed: https://docs.docker.com/compose/install/

Make sure that local port 5432 is not being used (if you executed `init_db.sh` in the previous step,
you might need to stop the Postgres service in Docker). To run the application:
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
curl -X GET http://localhost:8080/tx_fee \
-H "Content-Type: application/json" \
-d '["0x465a5e24ebe4ad90d1a235455f14a12b4aba4b956893d4bf11d0d986ee42c4a7", "0x926484f31f9d99d24b0e984a98483f6459872fbcb7e0abd5f1ce704d70835cee"]'
```

### References
- Zero to Production In Rust: An introduction to backend development in Rust
by Luca Palmieri
