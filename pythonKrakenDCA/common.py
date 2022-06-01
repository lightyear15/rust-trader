
import sys
import os
import psycopg2
from datetime import datetime
import pandas
import config
from typing import List


MAX_RANGE = 8388608
priceDecimals = {
    "xxbtzeur": 1,
    "xethzeur": 2,
    "atomeur": 2,
    "adaeur": 3,
    "lunaeur": 2,
    "mkreur": 1,
    }


def dateParser(date: str) -> datetime:
    return pandas.to_datetime(date, format="%Y-%m-%d %H:%M:%S.%f")


def processInputArgs(args: List[str]):
    if len(args) < 3:
        print("missing input args <person> <symbol>")
        sys.exit()
    person = args[1]
    symbol = args[2]
    logFile = buildTradeLogFileName(person, symbol)
    txFile = buildTXFileName(person, symbol)
    return person, symbol, logFile, txFile


def buildBaseFolderName(person: str) -> str:
    homeFolder = os.environ["HOME"]
    baseFolder = os.path.join(homeFolder, "krakenDCA/logs/{}".format(person))
    return baseFolder


def buildTXFileName(person: str, symbol: str) -> str:
    baseFolder = buildBaseFolderName(person)
    fileName = "{}_txs.txt".format(symbol)
    txFile = os.path.join(baseFolder, fileName)
    return txFile


def buildTradeLogFileName(person: str, symbol: str):
    baseFolder = buildBaseFolderName(person)
    logFileName = "trade.log"
    logFile = os.path.join(baseFolder, logFileName)
    return logFile


def buildDCALogFileName(person: str) -> str:
    baseFolder = buildBaseFolderName(person)
    logFile = os.path.join(baseFolder, "dca.log")
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
