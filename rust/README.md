# Deribit Rust SDK

Client for the Deribit WebSocket API, written in Rust.

```rust
let client = deribit::Client::public();
let client = deribit::Client::public_testnet();
let client = deribit::Client::private("your_client_id", "your_client_secret");

client.buy("BTC-PERPETUAL", deribit::Buy {
  amount: 1.0,
  price: 50000.0,
  type: "limit",
  post_only: true,
})
```

##  TODO
- [x] Auth
- [ ] Session mgmt
- [ ] Supporting
- [ ] Subscription mgmt
- [ ] Market data
- [ ] Trading
- [ ] Combo books
- [ ] Block trade
- [ ] Block RFQ
- [ ] Wallet
- [ ] Account mgmt

## Notes
- Doesn't support explicit sessions: [`fork_token`](https://docs.deribit.com/#public-fork_token), [scopes](https://docs.deribit.com/#access-scope)