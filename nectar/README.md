# nectar

`nectar help` to see the available commands.

Check `./sample-config.toml` if you want to customize the config.

## Security advisory

-   The seed file is used to generate the Bitcoin and Ethereum wallets.
-   Bitcoin funds are held in a new wallet generated in the bitcoind instance; keep your bitcoind instance secure.
-   If the seed file is lost, then any funds present in Nectar's Ethereum wallet are lost.
-   If the seed file is lost, Bitcoin funds can be recovered from the bitcoind instance.
-   The bitcoind wallet is **not** password protected.
-   If the database files are lost, then it is not possible to resume or abort ongoing swaps.
-   If the database files are lost, once funds are recovered, a new seed should be generated to avoid reuse of the Bitcoin transient keys used in the HTLCs. 
