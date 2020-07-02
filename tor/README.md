Using cnd over Tor
==================

`cnd` may be used to do atomic swaps anonymously across the Tor network. In order to do so you will need to do the following:

0. Install Tor

Either use your package manager or see [here](https://2019.www.torproject.org/docs/tor-doc-unix).

1. Configure an onion service using the Tor run file.

You can use the exaple `./torrc` and run `tor` using `sudo /usr/bin/tor --defaults-torrc tor-service-defaults-torrc -f torrc --RunAsDaemon 0`

2. Set the onion address in the cnd config file.

Once you have run `tor` for the first time get the onion address from the `hostname` file (if you used the `tor` invocation above this will be in `/var/lib/tor/hidden_service/hostname`).
See `./cnd.toml` for an example config file.

3. Before starting cnd to do a swap you will need a running Tor instance. Currently we use the default SOCKS5 Tor port 9050.
