### ▶️ Steps to Build

```
cargo build --release -p node-cli
```

```
rm -rf /tmp/solochain
```

```
./target/xxxxx key insert --base-path /tmp/solochain --chain dev --scheme Sr25519 --key-type btc! --suri //Alice
```

```
./target/xxxxx --base-path /tmp/solochain --dev
```

### ✅ Steps to Use

1. **Add Alice to the Whitelist**
   Call the `add_whitelist` extrinsic to add the account `Alice` to the whitelist.

2. **Default Supported Symbols**
   The `SupportedSymbols` storage is initialized with the following default trading pairs:
   `"BTC/USDT"`, `"ETH/USDT"`, `"SOL/USDT"`, `"DOGE/USDT"`
   You can dynamically add new symbols using the `add_symbol` extrinsic.

3. **Querying Chain State**
   Price data can be queried on-chain using the base token symbol (e.g., `"BTC"`, `"ETH"`). The latest price and timestamp will be returned.

4. **Customizing Price Sources**
   The available price sources are configured via Cargo feature flags:

   ```toml
   [features]
   # default = ["std", "cryptocompare", "okx", "binance", "coinbase", "bybit"]
   default = ["std", "cryptocompare", "okx"]
   ```

   Currently supported sources include:
   `Binance`, `OKX`, `Coinbase`, `Bybit`, `Cryptocompare`, etc.

5. **Price Weights**
   Each source has an associated weight (`weight`) as defined in code.
   For example:

   - Binance: 90
   - OKX: 80
     These weights are used for calculating weighted average prices.

---

### 🌐 Important Notes on Network Access

- Even if you can successfully fetch data using `curl`, this **does not guarantee** that the offchain worker (OCW) will be able to do so.
- OCWs run in a **restricted environment** (either WASM sandbox or native thread) within Substrate. Their HTTP networking capabilities are limited and may not support external HTTPS services directly.

---

### 🔄 Solution: Proxy Server

To resolve this, you need to run a **local proxy server** that fetches data from the target API and serves it over a simple HTTP endpoint.

For example, you can run a Node.js proxy server. If the following works:

```bash
curl http://localhost:3000/binance
```

Then your OCW will also be able to access the data successfully.

#### 📦 Install Dependencies

```bash
npm install express node-fetch@2 axios https-proxy-agent
```

You can refer to the provided `proxy.example.js` file for an implementation, or build your own using your preferred framework or language.
