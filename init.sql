-- Connect to the database
\c postgres_db;

-- Create a sample table
CREATE TABLE fees
(
    tx_hash   TEXT             NOT NULL,
    PRIMARY KEY (tx_hash),
    fee_eth   DOUBLE PRECISION NOT NULL,
    fee_usdt  DOUBLE PRECISION NOT NULL
);