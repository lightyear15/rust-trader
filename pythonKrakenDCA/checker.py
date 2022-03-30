import sys
import logging
from datetime import datetime
import os
import time

import common
import config
import kraken
import krakenex
from pykrakenapi import KrakenAPI


def main(person):
    logFile = common.buildDCALogFileName(person)
    common.checkOrCreateFileName(logFile)
    logging.basicConfig(filename=logFile, level=logging.INFO)
    api = krakenex.API(config.keys[person]["key"], config.keys[person]["secret"])
    kApi = KrakenAPI(api)
    symbols = config.dca_table[person].keys()
    for symbol in symbols:
        logging.info("###### checker for on {} {}".format(symbol, datetime.now()))
        txFile = common.buildTXFileName(person, symbol)
        common.checkOrCreateFileName(txFile)
        recordFile = buildRecordFileName(person, symbol)
        common.checkOrCreateFileName(recordFile)
        check(txFile, kApi, recordFile, symbol)
        time.sleep(5)
    return


def check(txLogFile: str, kApi: KrakenAPI, recordFile: str, symbol: str):
    openOrders = []
    closedOrders = []
    transactions = []
    with open(txLogFile, "r+") as txFD:
        transactions = txFD.read().splitlines()
    for tx in transactions:
        if tx == "":
            continue
        queryResult = kraken.queryOrder(kApi, tx)
        if queryResult is None:
            openOrders.append(tx)
            continue
        status, side, txID, price, volume, fees, tstamp = queryResult
        if status == "closed":
            closedOrders.append("{},{},{},{}".format(tstamp, price, volume, fees))
        elif status == "open":
            openOrders.append(tx)
        elif status == "expired":
            logging.info("order %s expired", tx)
        else:
            logging.warning("%s unknown status %s", tx, status)
    with open(recordFile, "a") as recFD:
        recFD.write("\n".join(closedOrders))
        recFD.write("\n")
    with open(txLogFile, "w") as txFD:
        for tx in openOrders:
            txFD.write(tx)
            txFD.write("\n")


def buildRecordFileName(person: str, symbol: str) -> str:
    basePath = os.environ["HOME"]
    recordFName = os.path.join(basePath, "krakenDCA/logs/{}/{}.csv".format(person, symbol))
    if os.path.isfile(recordFName) is False:
        with open(recordFName, "w") as fd:
            fd.write("date,price,volume,fees\n")
    return recordFName


if __name__ == "__main__":
    args = sys.argv
    if len(args) < 2:
        print("missing input args <person>")
        sys.exit()
    person = args[1]
    main(person)
