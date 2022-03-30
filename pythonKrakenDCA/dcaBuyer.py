
import time
import sys
import logging
from datetime import datetime, timedelta
from random import randint

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
        logging.info("###### buyer for on {} {}".format(symbol, datetime.now()))
        txFile = common.buildTXFileName(person, symbol)
        common.checkOrCreateFileName(txFile)
        priceDec = common.priceDecimals[symbol]
        expense = config.dca_table[person][symbol]
        dcaBuy(txFile, kApi, expense, symbol, priceDec)
        time.sleep(5)
    return


def dcaBuy(txLogFile, kApi, expense, symbol, priceDecimals):
    interval = timedelta(days=1)
    (candles, lastPrice) = kraken.getLastCandles(kApi, symbol, interval)
    wPrice = getWeightedAveragePrice(candles, config.window)
    price = lastPrice
    if wPrice < lastPrice:
        price = wPrice
    volume = common.getVolume(expense, price)
    buyID = randint(0, common.MAX_RANGE - 1)
    txid = kraken.addOrder(kApi, symbol, "buy", volume, price=price, price_decimals=priceDecimals,
                           expiration=config.expiration, userref=buyID)
    logging.info("limit order vol %f, price %f, buyID %d, txID %s", volume, price, buyID, txid)
    if txid is None:
        logging.error("buy order failed %f, %f", price, volume)
        return
    with open(txLogFile, "a") as txFile:
        txFile.write(txid)
        txFile.write("\n")


def getWeightedAveragePrice(candles, window):
    limit = datetime.now() - window
    idx = candles.index
    cnd = candles[idx >= limit]
    wg = (cnd["close"] * cnd["volume"]).sum() / cnd["volume"].sum()
    return wg


if __name__ == "__main__":
    args = sys.argv
    if len(args) < 2:
        print("missing input args <person>")
        sys.exit()
    person = args[1]
    main(person)
