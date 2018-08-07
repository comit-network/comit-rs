# Description

This is a docker-compose setup for two Lightning Network nodes Alice and Bob
connected to the same btcd node.

# Setup

Start one LND node for Alice and a second one for Bob

```bash
# Init bitcoin network env variable:
$ export NETWORK="simnet"

# Run the "Alice" container and log into it:
$ docker-compose up

# Generate a new backward compatible nested p2sh address for Alice:
$ docker exec -i -t lnd-alice lncli newaddress np2wkh

# Recreate "btcd" node and set Alice's address as mining address:
$ MINING_ADDRESS=<alice_address> docker-compose up -d

# Generate 400 blocks (we need at least "100 >=" blocks because of coinbase
# block maturity and "300 ~=" in order to activate segwit):
$ docker-compose run btcctl generate 400

# Check that segwit is active:
$ docker-compose run btcctl getblockchaininfo | grep -A 1 segwit
    
    "segwit": {
      "status": "active",

```

Connect Bob's LND node to Alice
```bash
# Get the identity pubkey of "Bob" node:
$ docker exec -i -t lnd-bob lncli getinfo


{
    "identity_pubkey": "0318d0603dfa73b1e90cbdd91b17ca9d42aee5ca6969c0e573b0b8a8b844d4aca6",
    "alias": "0318d0603dfa73b1e90c",
    "num_pending_channels": 0,
    "num_active_channels": 0,
    "num_peers": 0,
    "block_height": 800,
    "block_hash": "75c9bce52aafefaab4b5bd201676a63979b6edfc07f1a9953f4d14b16ba8d842",
    "synced_to_chain": true,
    "testnet": false,
    "chains": [
        "bitcoin"
    ],
    "uris": [
    ],
    "best_header_timestamp": "1533604718",
    "version": "0.4.2-beta commit=7cf5ebe2650b6798182e10be198c7ffc1f1d6e19"
}

# Get the IP address of "Bob" node:
$ docker inspect lnd-bob | grep IPAddress

# Connect "Alice" to the "Bob" node:
$ docker exec -i -t lnd-alice lncli connect <bob_pubkey>@<bob_host>

# Check list of peers on "Alice" side:
$ docker exec -i -t lnd-alice lncli listpeers
{
    "peers": [
        {
            "pub_key": "0318d0603dfa73b1e90cbdd91b17ca9d42aee5ca6969c0e573b0b8a8b844d4aca6",
            "address": "172.28.0.4:9735",
            "bytes_sent": "7",
            "bytes_recv": "7",
            "sat_sent": "0",
            "sat_recv": "0",
            "inbound": false,
            "ping_time": "0"
        }
    ]
}

# Check list of peers on "Bob" side:
$ docker exec -i -t lnd-bob lncli listpeers
{
    "peers": [
        {
            "pub_key": "02c6854e99b936953139d1aea9ca9e44c8274eaa84746c4d7d26a4bf1d2966fd67",
            "address": "172.28.0.2:53016",
            "bytes_sent": "7",
            "bytes_recv": "7",
            "sat_sent": "0",
            "sat_recv": "0",
            "inbound": true,
            "ping_time": "0"
        }
    ]
}
```
