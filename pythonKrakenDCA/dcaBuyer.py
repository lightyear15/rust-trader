
import sys
import logging
from datetime import datetime, timedelta
from random import randint

import common
import config


def main(txLogFile, person, symbol, price_decimals):
    orders_count = len(open(txLogFile).readlines())
    if orders_count >= config.max_order[person]:
        return
    interval = timedelta(minutes=60)
    (candles, last) = common.getLastCandles(symbol, interval)
    candles = candles[:int(config.window / interval)]
    price = getWeightedAveragePrice(candles)
    expense = config.dca_table[person][symbol]
    volume = common.getVolume(expense, price)
    buyID = randint(0, common.MAX_RANGE - 1)
    txid = common.addOrder(config.keys[person], symbol, "buy",
                                                        volume,
                                                        price=price,
                                                        price_decimals=price_decimals,
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
    person, symbol, log_file, tx_file, decimals = common.processInputArgs(sys.argv)
    common.checkOrCreateFileNames(log_file, tx_file)
    logging.basicConfig(filename=log_file, level=logging.INFO)
    logging.info("###### buyer for on {} {}".format(symbol, datetime.now()))
    main(tx_file, person, symbol, decimals)
