
import sys
import logging
from datetime import datetime, timedelta
import pandas
import common
import config
from ta import momentum

RSI_THRE = 40

def main(txLogFile, person, symbol, price_decimals):
    orders_count = len(open(txLogFile).readlines())
    if orders_count >= config.max_order[person]:
        return
    interval = timedelta(days=1)
    (candles, lastPrice) = common.getLastCandles(symbol, interval)
    series = buildSeries(candles)
    rsi = getRSI(series)
    lastRsi = rsi.rsi()[-1]
    secondLastRsi = rsi.rsi()[-2]
    if lastRsi >= RSI_THRE:
        logging.info("lastRsi %f @ %f, quitting", lastRsi, lastPrice)
        return
    if secondLastRsi >= RSI_THRE:
        logging.info("(lastRsi %f) - secondLastRsi %f @ %f, quitting", lastRsi, secondLastRsi, lastPrice)
        return
    if lastRsi < secondLastRsi:
        logging.info("lastRsi < secondLastRsi %f < %f @ %f, quitting", lastRsi, secondLastRsi, lastPrice)
        return
    volume = common.getVolume(config.euros[person], lastPrice)
    buyID = randint(0, common.MAX_RANGE - 1)
    txid = common.addOrder(config.keys[person], symbol, "buy",
                           volume,
                           price=lastPrice,
                           price_decimals=price_decimals,
                           expiration=config.expiration,
                           userref=buyID)
    logging.info("limit order vol %f, price %f, txID %s", volume, lastPrice, txid)
    if txid is None:
        logging.error("buy order failed %f, %f", lastPrice, volume)
        return
    with open(txLogFile, "a") as txFile:
        txFile.write(txid)
        txFile.write("\n")


def buildSeries(candls):
    return pandas.Series(data=[c["close"] for c in candls],
                         index=[pandas.Timestamp(c["tstamp"], unit="s") for c in candls]).sort_index()


def getRSI(candles):
    return momentum.RSIIndicator(close=candles)


if __name__ == "__main__":
    person, symbol, log_file, tx_file, decimals = common.processInputArgs(sys.argv)
    common.checkOrCreateFileNames(log_file, tx_file)
    logging.basicConfig(filename=log_file, level=logging.INFO)
    logging.info("###### rsibuyer for on {} {}".format(symbol, datetime.now()))
    main(tx_file, person, symbol, decimals)
