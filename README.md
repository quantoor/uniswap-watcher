# Uniswap Watcher

This application provides the following functionalities:

- It subscribes to swap events occurring on the WETH-USDC-500 pool on UniswapV3 on Ethereum mainnet.
- For every new event, it fetches the corresponding transaction data to get the tx fee in USDT and stores it in a database.
It also computes and logs the swap price from the log amounts.
- It runs a web server that exposes endpoints to query the tx fee in USDT for given transaction hashes.
  - If the tx hash exists in the database, the corresponding info is fetched from it and returned.
  - Otherwise, the tx fee is computed, stored in db and then returned.
- In order to store data in the database, a queue (channel) is shared between threads. Instead of blocking the thread by inserting the data
directly - which is potentially a time-consuming operation - the data is inserted in a queue. A separate thread is responsible for
consuming the data and inserting it in the database.

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
Get tx gas fee:
```
curl -X GET http://localhost:8080/tx_fee \
-H "Content-Type: application/json" \
-d '["0x465a5e24ebe4ad90d1a235455f14a12b4aba4b956893d4bf11d0d986ee42c4a7", "0x926484f31f9d99d24b0e984a98483f6459872fbcb7e0abd5f1ce704d70835cee"]'
```
Get swap price:
```
curl "http://localhost:8080/swap_price?tx_hash=0xe55abfa818e6237b794a41a99482ef7108ed7d6c89867ed9b443011c93d2fb77"
```

### System considerations
- Availability: this is achieved with a careful error handling that always keeps the application in a known state.
- Scalability: the amount of hardcoded values has been minimized to very specific cases, and the functions have been
designed to be as decoupled as possible and the code modular, in order to achieve a certain degree of abstraction which 
allows the application to be scalable. 
- Reliability: a set of automated tests has been implemented to decrease the likelihood of bugs and make sure that
the application behaves as expected.

### Decoding the swap price
Given the log of a swap event, the swap price can be computed from amount0 and amount1. 

For the WETH/USDC-500 pool in UniswapV3, amount0 represents the amount of USDC and amount1 the amount of WETH. 
The amounts need to be normalized by the number of decimals (6 for UDSC and 18 for WETH), and
then the price can be computed as `amount0/amount1`.

### References
- Zero to Production In Rust: An introduction to backend development in Rust
  by Luca Palmieri