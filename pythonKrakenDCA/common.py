
import sys
import requests
import hashlib
import hmac
import base64
import urllib.parse
import time
import os
import logging
import psycopg2
from datetime import timedelta
import config

MAX_RANGE = 8388608

def sign(keys, data, urlpath):
    postdata = urllib.parse.urlencode(data)
    encoded = (str(data['nonce']) + postdata).encode()
    message = urlpath.encode() + hashlib.sha256(encoded).digest()
    signature = hmac.new(base64.b64decode(keys["secret"]), message, hashlib.sha512)
    sigdigest = base64.b64encode(signature.digest())
    return sigdigest.decode()


def processInputArgs(args: [str]):
    if len(args) < 3:
        print("missing input args <person> <symbol> (decimals)")
        sys.exit()
    person = args[1]
    symbol = args[2]
    log_file, tx_file = buildFileNames(person, symbol)
    decimals = 2
    if len(args) > 3:
        decimals = int(args[3])
    return person, symbol, log_file, tx_file, decimals


def buildFileNames(person: str, symbol: str):
    base_path = os.environ["HOME"]
    log_file = os.path.join(base_path, "krakenDCA/logs/{}/{}.log".format(person, symbol))
    tx_file = os.path.join(base_path, "krakenDCA/logs/{}/{}_txs.txt".format(person, symbol))
    return log_file, tx_file


def checkOrCreateFileNames(log, tx):
    if os.path.isfile(log) is False:
        open(log, 'a').close()
    if os.path.isfile(tx) is False:
        open(tx, 'a').close()
    return


def nonce() -> int:
    return int(1000*time.time())


def queryOrder(keys, tx):
    urlpath = "/0/private/QueryOrders"
    data = {"txid": tx, "nonce": nonce()}
    headers = {"API-Key": keys["key"], "API-Sign": sign(keys, data, urlpath)}
    # logging.info("querying order %s", tx)
    order = requests.post(config.api_endpoint + urlpath, data=data, headers=headers)
    if order.status_code != 200:
        logging.error("got an error on AddOrder")
        return None
    order_json = order.json()
    # print("queryOrder", tx, order_json)
    if order_json["error"] != []:
        logging.error("queryOrder, response error %s", order_json["error"][0])
        return None
    status = order_json["result"][tx]["status"]
    ttype = order_json["result"][tx]["descr"]["type"]
    vol = float(order_json["result"][tx]["vol_exec"])
    price = float(order_json["result"][tx]["price"])
    ref = int(order_json["result"][tx]["userref"])
    return status, ttype, price, vol, ref


def addOrder(keys, symbol, direction, volume, price=None,
             price_decimals=None, expiration: timedelta = None, userref: int = None):
    vol_str = "{:.8f}".format(volume)
    urlpath = "/0/private/AddOrder"
    data = {
            "nonce": nonce(),
            "pair": symbol,
            "type": direction,
            "volume": vol_str,
            # "validate": "true",
            }
    if price is None:
        data["ordertype"] = "market"
    else:
        price_str = "{:.{prec}f}".format(price, prec=price_decimals)
        data["ordertype"] = "limit"
        data["price"] = price_str
    if expiration is not None:
        data["expiretm"] = "+{}".format(int(expiration.total_seconds()))
    if userref is not None:
        data["userref"] = userref
    headers = {"API-Key": keys["key"], "API-Sign": sign(keys, data, urlpath)}
    # logging.info("sending order, direction %s, vol %s, price %s", direction, vol_str, price_str)
    order = requests.post(config.api_endpoint + urlpath, data=data, headers=headers)
    if order.status_code != 200:
        logging.error("got an error on AddOrder")
        return None
    order_json = order.json()
    # print("addOrder", direction, order_json)
    if order_json["error"] != []:
        logging.error("addOrder, response error %s", order_json["error"][0])
        return None
    txids = order_json["result"]["txid"]
    if txids is None:
        return None
    if len(txids) != 1:
        logging.error("expecting only one txid, got %d", len(txids))
    return txids[0]


def getLastCandles(symbol: str, interval: timedelta):
    minutes = int(interval.total_seconds() / 60.0)  # in minutes
    ohlc = requests.get("{}/0/public/OHLC?pair={}&interval={}".format(config.api_endpoint, symbol, minutes))
    if ohlc.status_code != 200:
        logging.error("got an error from ohlc")
        return None, None
    ohlc_json = ohlc.json()
    if ohlc_json["error"] != []:
        logging.error("addOrder, response error %s", ohlc_json["error"][0])
        return None, None
    candles = [{
            "tstamp": int(cdl[0]),
            "open": float(cdl[1]),
            "high": float(cdl[2]),
            "low": float(cdl[3]),
            "close": float(cdl[4]),
            "volume": float(cdl[6])
            } for cdl in ohlc_json["result"][symbol.upper()]]
    candles.sort(key=lambda x: x["tstamp"], reverse=True)
    return (candles[1:], candles[0]["close"])


def getVolume(max_order: float, buy_price: float) -> float:
    return max_order / buy_price


def openDBConn(connData):
    if connData is None:
        return None
    con = psycopg2.connect(database=connData["database"], user=connData["user"],
                           password=connData["password"], host=connData["host"], port=connData["port"])
    return con


def closeDBConn(db_conn):
    if db_conn is None:
        return
    db_conn.commit()
    db_conn.close()


def recordTransaction(db_conn):
    if db_conn is None:
        return
