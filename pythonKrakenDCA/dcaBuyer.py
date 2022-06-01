import time
import sys
import logging
from datetime import datetime, timedelta
from random import randint
import ta

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
        expense = config.dca_table[person][symbol]
        dcaBuy(txFile, kApi, expense, symbol)
        time.sleep(5)
    return


def dcaBuy(txLogFile, kApi, expense, symbol):
    interval = timedelta(days=1)
    volumeDecimals, priceDecimals = kraken.getPairDecimals(kApi=kApi, pair=symbol)
    (candles, lastPrice) = kraken.getLastCandles(kApi, symbol, interval)
    wPrice = getWeightedAveragePrice(candles, config.window.days)
    price = lastPrice
    if wPrice < lastPrice:
        price = wPrice
    volume = common.getVolume(expense, price)
    buyID = randint(0, common.MAX_RANGE - 1)
    txid = kraken.addOrder(
        kApi,
        symbol,
        "buy",
        volume,
        price=price,
        volumeDecimals=volumeDecimals, priceDecimals=priceDecimals,
        expiration=config.expiration,
        userref=buyID,
    )
    logging.info(
        "limit order vol %f, price %f, buyID %d, txID %s", volume, price, buyID, txid
    )
    if txid is None:
        logging.error("buy order failed %f, %f", price, volume)
        return
    with open(txLogFile, "a") as txFile:
        txFile.write(txid)
        txFile.write("\n")


def getWeightedAveragePrice(candles, window: int):
    wa = ta.volume.VolumeWeightedAveragePrice(
        high=candles["high"],
        low=candles["low"],
        close=candles["close"],
        volume=candles["volume"],
        window=window,
    )
    return wa.volume_weighted_average_price()[-1]


if __name__ == "__main__":
    args = sys.argv
    if len(args) < 2:
        print("missing input args <person>")
        sys.exit()
    person = args[1]
    main(person)
