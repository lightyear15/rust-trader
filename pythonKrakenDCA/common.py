
import sys
import os
import psycopg2
from datetime import datetime
import config
from typing import List, Optional

MAX_RANGE = 8388608
priceDecimals = {
    "xxbtzeur": 1,
    "xethzeur": 2,
    "atomeur": 2,
    "adaeur": 3
    }


def processInputArgs(args: List[str]):
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


def buildTXFileName(person: str, symbol: str) -> str:
    basePath = os.environ["HOME"]
    txFile = os.path.join(basePath, "krakenDCA/logs/{}/{}_txs.txt".format(person, symbol))
    return txFile


def buildFileNames(person: str, symbol: str):
    basePath = os.environ["HOME"]
    logFile = os.path.join(basePath, "krakenDCA/logs/{}/{}.log".format(person, symbol))
    txFile = os.path.join(basePath, "krakenDCA/logs/{}/{}_txs.txt".format(person, symbol))
    return logFile, txFile


def buildDCALogFileName(person: str) -> str:
    basePath = os.environ["HOME"]
    logFile = os.path.join(basePath, "krakenDCA/logs/{}/dca.log".format(person))
    return logFile


def checkOrCreateFileName(fname):
    if os.path.isfile(fname) is False:
        open(fname, 'a').close()



def getVolume(max_order: float, buy_price: float) -> float:
    return max_order / buy_price


def openDBConn(connData):
    if connData is None:
        return None
    con = psycopg2.connect(database=connData["database"], user=connData["username"],
                           password=connData["password"], host=connData["hostname"], port=connData["port"])
    return con


def closeDBConn(db_conn):
    if db_conn is None:
        return
    db_conn.commit()
    db_conn.close()


def recordTransaction(db_conn, symbol: str, tstamp: datetime, side: str, price: float, volume: float,
                      opId: int, fees: float, reference: int):
    if db_conn is None:
        return
    cursor = db_conn.cursor()
    cursor.execute("INSERT INTO transactions (exchange, symbol, tstamp, side, price, volume, id, fees, fees_asset, reference) VALUES('kraken', %s, %s, %s, %s, %s, %s, %s, 'EUR', %s)", (symbol, tstamp, side.capitalize(), price, volume, opId, fees, reference))
    db_conn.commit()
    cursor.close()
