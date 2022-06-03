import sys
import logging
from datetime import datetime

import common
import config
import kraken
import krakenex
from pykrakenapi import KrakenAPI


def main(txLogFile, dbConn, person, symbol):
    openOrders = []
    transactions = []
    api = krakenex.API(config.keys[person]["key"], config.keys[person]["secret"])
    kApi = KrakenAPI(api)
    with open(txLogFile, "r+") as txFile:
        transactions = txFile.read().splitlines()
    for tx in transactions:
        status, side, txID, price, volume, fees, tstamp = kraken.queryOrder(kApi, tx)
        if status == "closed" and side == "sell":
            logging.info("%s sell closed order", tx)
            buyRef = txID - common.MAX_RANGE
            common.recordTransaction(dbConn, symbol, tstamp, side, price, volume, txID, fees, buyRef)
        elif status == "closed" and side == "buy":
            logging.info("%s buy closed order - %d", tx, txID)
            tpPrice, tpVolume = computeTakeProfit(person, price, volume)
            sellID = txID + common.MAX_RANGE
            volDecimals, priceDecimals = kraken.getPairDecimals(kApi=kApi, pair=symbol)
            tpTxID = kraken.addOrder(kApi, symbol, "sell", tpVolume, price=tpPrice,
                                     volumeDecimals=volDecimals, priceDecimals=priceDecimals,
                                     userref=sellID)
            common.recordTransaction(dbConn, symbol, tstamp, side, price, volume, txID, fees, 0)
            logging.info("taking profit -> %s", tpTxID)
            if tpTxID is not None:
                openOrders.append(tpTxID)
            else:
                logging.error("order failed %f @ %f", tpVolume, tpPrice)
                openOrders.append(tx)
        elif status == "open":
            # logging.info("%s open order", tx)
            openOrders.append(tx)
        else:
            logging.warning("%s unknown status %s", tx, status)
    with open(txLogFile, "w") as txFile:
        for tx in openOrders:
            txFile.write(tx)
            txFile.write("\n")
    common.closeDBConn(dbConn)


def computeTakeProfit(person, price, volume):
    factor = 1.0 + config.take_profit_factor
    takeProfitPrice = price * factor
    takeProfitVolume = price * volume / takeProfitPrice
    return takeProfitPrice, takeProfitVolume


if __name__ == "__main__":
    person, symbol, logFile, txFile = common.processInputArgs(sys.argv)
    common.checkOrCreateFileName(logFile)
    common.checkOrCreateFileName(txFile)
    db_conn = common.openDBConn(config.database)
    logging.basicConfig(filename=logFile, level=logging.INFO)
    logging.info("###### seller for {} on {}".format(symbol, datetime.now()))
    main(txFile, db_conn, person, symbol)
