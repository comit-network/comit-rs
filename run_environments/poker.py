#!/usr/bin/env python3

import argparse
import json
import os
import requests
import sys

HEADERS = {
    'Content-type': 'application/json',
}


def main():
    parser = argparse.ArgumentParser(description='Poke a swap service.')
    parser.add_argument("--btc-funded", help="Poke exchange_service as BTC HTLC has been funded.",
                        action="store_true")
    parser.add_argument("--eth-deployed", help="Poke trading_service as ETH HTLC has been deployed.",
                        action="store_true")
    parser.add_argument("--btc-redeem", help="Poke exchange_service as secret has been revealed.", action="store_true")
    parser.add_argument("-u", "--uid", help="The trade UID.")
    parser.add_argument("-t", "--txid", help="The Transaction ID of the BTC funding tx.")
    parser.add_argument("-v", "--vout", help="The vout of the BTC funding tx.")
    parser.add_argument("-c", "--contract", help="The address of the ETH HTLC.")
    parser.add_argument("-s", "--secret", help="The secret.")
    args = parser.parse_args()
    
    exchange_service_url = os.environ['BOB_COMIT_NODE_PID']
    if not exchange_service_url:
        print("BOB_COMIT_NODE_PID must be set.")
        sys.exit(1)

    trading_service_url = os.environ['ALICE_COMIT_NODE_PID']
    if not trading_service_url:
        print("ALICE_COMIT_NODE_PID must be set.")
        sys.exit(1)

    if args.btc_funded:
        btc_funded(exchange_service_url, args.uid, args.txid, args.vout)
    elif args.eth_deployed:
        eth_deployed(trading_service_url, args.uid, args.contract)
    elif args.btc_redeem:
        btc_redeem(exchange_service_url, args.uid, args.secret)


def btc_funded(exchange_service_url, trade_id, tx_id, vout):
    if not (exchange_service_url and trade_id and tx_id and vout):
        print("btc-funded action needs exchange_url, uid, txid and vout")
        sys.exit(2)

    data = {'transaction_id': tx_id, 'vout': int(vout)}
    url = '{ex_url}/trades/ETH-BTC/{trade_id}/buy-order-htlc-funded'.format(ex_url=exchange_service_url, trade_id=trade_id)

    response = requests.post(url, headers=HEADERS, data=json.dumps(data))
    print(response)


def eth_deployed(trading_service_url, trade_id, contract_address):
    if not (trading_service_url and trade_id and contract_address):
        print("eth-redeem needs trading_service_url, uid and contract_address")
        sys.exit(2)

    data = {'contract_address': contract_address}
    url = '{t_url}/cli/trades/ETH-BTC/{trade_id}/buy-order-contract-deployed'.format(t_url=trading_service_url, trade_id=trade_id)

    response = requests.post(url, headers=HEADERS, data=json.dumps(data))
    print(response)


def btc_redeem(exchange_service_url, trade_id, secret):
    if not (exchange_service_url and trade_id and secret):
        print("exchange_service_url, trade_id and secret are needed for btc_redeem")
        sys.exit(2)

    data = {'secret': secret}
    url = '{ex_url}/trades/ETH-BTC/{trade_id}/buy-order-secret-revealed'.format(ex_url=exchange_service_url, trade_id=trade_id)

    response = requests.post(url, headers=HEADERS, data=json.dumps(data))
    print(response)


if __name__ == '__main__':
    main()
