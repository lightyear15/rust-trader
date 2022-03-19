
import time
import sys
import logging
from datetime import datetime, timedelta
from random import randint

import common
import config


def main(person):
    logFile = common.buildDCALogFileName(person)
    common.checkOrCreateFileName(logFile)
    logging.basicConfig(filename=logFile, level=logging.INFO)
    symbols = config.dca_table[person].keys()
    for symbol in symbols:
        logging.info("###### buyer for on {} {}".format(symbol, datetime.now()))
        txFile = common.buildTXFileName(person, symbol)
        common.checkOrCreateFileName(txFile)
        priceDec = common.priceDecimals[symbol]
        dcaBuy(txFile, person, symbol, priceDec)
        time.sleep(5)
    return


def dcaBuy(txLogFile, person, symbol, priceDecimals):
    ordersCount = len(open(txLogFile).readlines())
    if ordersCount >= config.max_order[person]:
        return
    interval = timedelta(minutes=60)
    (candles, lastPrice) = common.getLastCandles(symbol, interval)
    candles = candles[:int(config.window / interval)]
    wPrice = getWeightedAveragePrice(candles)
    price = lastPrice
    if wPrice < lastPrice:
        price = wPrice
    expense = config.dca_table[person][symbol]
    volume = common.getVolume(expense, price)
    buyID = randint(0, common.MAX_RANGE - 1)
    txid = common.addOrder(config.keys[person], symbol, "buy",
                                                        volume,
                                                        price=price,
                                                        price_decimals=priceDecimals,
                                                        expiration=config.expiration,
                                                        userref=buyID)
    logging.info("limit order vol %f, price %f, buyID %d, txID %s", volume, price, buyID, txid)
    if txid is None:
        logging.error("buy order failed %f, %f", price, volume)
        return
    with open(txLogFile, "a") as txFile:
        txFile.write(txid)
        txFile.write("\n")


def getWeightedAveragePrice(candles):
    total_volume = 0
    total_price = 0
    for candle in candles:
        avg = (candle["low"] + candle["high"]) / 2
        vol = candle["volume"]
        total_volume += vol
        total_price += avg * vol
    average = total_price / total_volume
    return average


if __name__ == "__main__":
    args = sys.argv
    if len(args) < 2:
        print("missing input args <person>")
        sys.exit()
    person = args[1]
    main(person)
