import sys
import logging
from datetime import datetime, timedelta
import pandas
import common
import config
import ta
from random import randint

MFI_THRE = 30


def main(txLogFile, person, symbol, price_decimals):
    orders_count = len(open(txLogFile).readlines())
    if orders_count >= config.max_order[person]:
        return
    interval = timedelta(days=1)
    (candles, lastPrice) = common.getLastCandles(symbol, interval)
    high, low, close, volume = buildSeriess(candles)
    mfi = getMFI(high, low, close, volume)
    bb = getBB(close)
    lastMfi = mfi.money_flow_index()[-1]
    bbLowIndicator = bb.bollinger_lband_indicator()[-1]
    if lastMfi >= MFI_THRE:
        logging.info("lastMfi %f @ %f, quitting", lastMfi, lastPrice)
        return
    if bbLowIndicator == 0:
        logging.info("bbLowIndicator %d @ %f, quitting", bbLowIndicator, lastPrice)
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


def buildSeriess(candls):
    index = [pandas.Timestamp(c["tstamp"], unit="s") for c in candls]
    high = pandas.Series(data=[c["high"] for c in candls], index=index).sort_index()
    low = pandas.Series(data=[c["low"] for c in candls], index=index).sort_index()
    close = pandas.Series(data=[c["close"] for c in candls], index=index).sort_index()
    volume = pandas.Series(data=[c["volume"] for c in candls], index=index).sort_index()
    return high, low, close, volume


def getMFI(high, low, close, volume):
    return ta.volume.MFIIndicator(high=high, low=low, close=close, volume=volume)


def getBB(close):
    return ta.volatility.BollingerBands(close=close)


if __name__ == "__main__":
    person, symbol, logFile, txFile, decimals = common.processInputArgs(sys.argv)
    common.checkOrCreateFileName(logFile)
    common.checkOrCreateFileName(txFile)
    logging.basicConfig(filename=logFile, level=logging.INFO)
    logging.info("###### bbmfibuyer for on {} {}".format(symbol, datetime.now()))
    main(txFile, person, symbol, decimals)
