import sys
import logging
from datetime import datetime
import os
import time

import common
import config


def main(person):
    logFile = common.buildDCALogFileName(person)
    common.checkOrCreateFileName(logFile)
    logging.basicConfig(filename=logFile, level=logging.INFO)
    symbols = config.dca_table[person].keys()
    for symbol in symbols:
        txFile = common.buildTXFileName(person, symbol)
        common.checkOrCreateFileName(txFile)
        recordFile = buildRecordFileName(person, symbol)
        common.checkOrCreateFileName(recordFile)
        check(txFile, recordFile, person, symbol)
        time.sleep(5)
    return


def check(txLogFile, recordFile, person, symbol):
    openOrders = []
    closedOrders = []
    transactions = []
    keys = config.keys[person]
    with open(txLogFile, "r+") as txFD:
        transactions = txFD.read().splitlines()
    for tx in transactions:
        if tx == "":
            continue
        status, side, txID, price, volume, fees, tstamp = common.queryOrder(keys, tx)
        if status == "closed":
            closedOrders.append("{},{},{},{}".format(tstamp, price, volume, fees))
        elif status == "open":
            openOrders.append(tx)
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
