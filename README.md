# chainstate

Chainstate is a CLI utility to examine the health of EVM-compatible nodes via JSON-RPC api

## Usage
```
USAGE:
    chainstate [FLAGS] [OPTIONS]

FLAGS:
        --endpoints    Return working endpoint (tag may be applied to restrict the list)
    -h, --help         Prints help information
    -s, --server       Whether to start HTTP API server
    -V, --version      Prints version information

OPTIONS:
    -a, --addr <addr>                      In case of server, TCP address to be listened [env: LISTEN=] [default:0.0.0.0:8000]
        --network <network>                Check single network address (internally used tags: nosync, nogaps)
    -n, --networks-file <networks-file>    Optional - plain text file, containing the list of RPC addresses to be
                                           checked. Tag may be appled to restrict the list [env: NETWORKS_FILE=./networks.txt]  [default: ]
    -t, --tag <tag>                        Filter chains by tag [default: ]
```

### Check state of single RPC node
```
$ chainstate --network http://localhost:4444/
Dec 20 10:10:16.685  INFO chain 31, block 2451166
```

### Check state of multiple RPC nodes

To manage multiple nodes, please create plain text file to contain the list of JSON+RPC nodes
(path can be preserved in `NETWORKS_FILE` environment variable). Each lines means separate JSON+RPC node. 
Optional rows starting with `#` can contain the list of comma-separated tags which can be helpful in filtering the list

Example of such file:
```
# arbitrum
https://arbitrum.xdaichain.com/
# rsk, nosync, nogaps
https://public-node.rsk.co
# rsk, nosync, nogaps
https://mainnet.sovryn.app/rpc
# rsk, nosync, nogaps, testnet
https://public-node.testnet.rsk.co
# avalanche, nosync, nogaps
https://api.avax.network/ext/bc/C/rpc
```

Special tags can be included
- `nosync` - means `eth_syncing` to check status of the sync
- `nogaps` - means there will be no request `parity_chainStatus` to check gaps in the blocks

Check state of all networks:
```
chainstate -n networks.txt
```

Check state of all networks filtered by tag:
```
chainstate -n networks.txt -t rsk
```

Check state of all networks not including tag `testnet`:
```
chainstate -n networks.txt -t rsk,-testnet
```

### Healthy node selection

To get working JSON+RPC endpoint URLs in plain text format (one URL - one line), 
the same principle as getting multiple nodes state is applied, with `--endpoints` flag in addition.

```
chainstate -n networks.txt -t rsk,-testnet --endpoints
```

## License

MIT