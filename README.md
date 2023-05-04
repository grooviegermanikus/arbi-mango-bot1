
# Local Development
Start mango-feeds _service-mango-orderbook_ on port 8080:
```
 # note: need to create test-config.toml based on example-config.toml
 cargo run --bin service-mango-orderbook service-mango-orderbook/conf/test-config.toml
```

Start TypeScript client for reference (need to adjust __marketId__):
```
# change marketId in code
npx ts-node ./ts/client/scripts/orderbook.ts
```

Sample output from TypeScript client:
```
update {
  market: 'Fgh9JSZ2qfSjCw9RPJ85W2xbihsp2muLvfRztzoVR7f1',
  side: 'ask',
  update: [
    [ 1884.56, 0 ],
    [ 1885.88, 0 ],
    [ 1887.01, 0 ],
    [ 1888.9, 0 ],
    [ 1884.46, 0.026000000000000002 ],
    [ 1885.78, 0.2605 ],
    [ 1886.91, 0.5209 ],
    [ 1888.79, 4.2516 ]
  ],
  slot: 191936563,
  write_version: 695365571888
}
```

# What's missing
* WebSocket reconnect handling for mango-feeds service