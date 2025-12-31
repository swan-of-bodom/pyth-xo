# pyth-keeper

Service that monitors Pyth Network price feeds and automatically updates on-chain oracles based on deviation thresholds and heartbeat intervals.

## Installation

```bash
cargo build --release
```

## Configuration

### 1. Create Configuration File

Copy the example configuration:

```bash
cp config.example.json config.json
```

Edit `config.json` with your settings:

```json
{
  "pyth_hermes_url": "https://hermes.pyth.network",
  "poll_interval_seconds": 30,
  "networks": [
    {
      "name": "Base",
      "chain_id": 8453,
      "rpc_url": "https://mainnet.base.org",
      "pyth_contract": "0x8250f4aF4B972684F7b336503E2D6dFeDeB1487a",
      "native_feed_id": "ff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace",
      "block_explorer": "https://basescan.org"
    }
  ],
  "feeds": [
    {
      "price_feed_id": "ff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace",
      "symbol": "ETH/USD",
      "deviation_threshold": 0.5,
      "heartbeat_seconds": 14400,
      "networks": ["Base"]
    }
  ]
}
```

### 2. Set Environment Variable

The private key is loaded from an environment variable (never stored in config):

```bash
export PRIVATE_KEY=your_private_key_here
```

Or create a `.env` file:

```
PRIVATE_KEY=your_private_key_here
```

## Usage

```bash
# Run with logging
RUST_LOG=info cargo run --release
```

## Pyth Price Feed IDs

Pyth Network price feed IDs: https://insights.pyth.network/price-feeds

Common feeds:
- ETH/USD: `ff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace`
- BTC/USD: `e62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43`
- USDC/USD: `eaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a`
- USDT/USD: `2b89b9dc8fdf9f34709a5b106b472f0f39bb6ca9ce04b0fd7f2e971688e2e53b`

## Pyth Contract Addresses

- **Ethereum**: `0x4305FB66699C3B2702D4d05CF36551390A4c69C6`
- **Base**: `0x8250f4aF4B972684F7b336503E2D6dFeDeB1487a`
- **Unichain Sepolia**: `0xA2aa501b19aff244D90cc15a4Cf739D2725B5729`

See full list: https://docs.pyth.network/price-feeds/contract-addresses/evm

## Update Logic

Each cycle:
1. Fetch all feed prices in 1 API request
2. For each feed, check if deviation >= threshold OR time >= heartbeat
3. Per network: batch all feeds that need updating into 1 transaction

## Resources

- [Pyth Network Documentation](https://docs.pyth.network/)
- [Pyth Price Feeds](https://pyth.network/developers/price-feed-ids)
- [Pyth EVM Integration](https://docs.pyth.network/price-feeds/use-real-time-data/evm)
- [Impermax Finance](https://www.impermax.finance/)

## License

MIT
