import sys
import logging
from datetime import datetime
import os

import common
import config


def main(txLogFile, recordFile, person, symbol):
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
            closedOrders.append("{},{},{},{}".format(tstamp,price,volume,fees))
        elif status == "open":
            openOrders.append(tx)
        else:
            logging.warning("%s unknown status %s", tx, status)
    with open(recordFName, "a") as recFD:
        recFD.write("\n".join(closedOrders))
        recFD.write("\n")
    with open(txLogFile, "w") as txFD:
        for tx in openOrders:
            txFD.write(tx)
            txFD.write("\n")


def checkOrCreateRecordFile(txFName, symbol):
    dirname = os.path.dirname(txFName)
    recordFName = os.path.join(dirname, "{}.csv".format(symbol))
    if os.path.isfile(recordFName) is False:
        with open(recordFName, "w") as fd:
            fd.write("date,price,volume,fees\n")
    return recordFName


if __name__ == "__main__":
    person, symbol, logFName, txFName, _ = common.processInputArgs(sys.argv)
    common.checkOrCreateFileNames(logFName, txFName)
    recordFName = checkOrCreateRecordFile(txFName, symbol)
    logging.basicConfig(filename=logFName, level=logging.INFO)
    logging.info("###### checker on {} {}".format(symbol, datetime.now()))
    main(txFName, recordFName, person, symbol)
