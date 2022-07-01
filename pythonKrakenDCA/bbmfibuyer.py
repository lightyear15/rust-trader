import sys
import logging
from datetime import datetime, timedelta
import pandas
import ta
from random import randint
import time

import common
import config
import kraken
import krakenex
from pykrakenapi import KrakenAPI

MFI_THRE = 35


def main(txLogFile, person, symbol):
    orders_count = len(open(txLogFile).readlines())
    if orders_count >= config.max_order:
        return
    api = krakenex.API(config.keys[person]["key"], config.keys[person]["secret"])
    kApi = KrakenAPI(api)
    interval = timedelta(days=1)
    (candles, lastPrice) = kraken.getLastCandles(kApi, symbol, interval)
    mfi = ta.volume.MFIIndicator(high=candles["high"], low=candles["low"], close=candles["close"], volume=candles["volume"])
    bb = ta.volatility.BollingerBands(close=candles[ "close" ])
    # print(candles["close"][-10:], bb.bollinger_lband_indicator()[-10:], mfi.money_flow_index()[-10:])
    mfiIndicator = mfi.money_flow_index()[-2:-1]
    if any(map(lambda x: x >= MFI_THRE, mfiIndicator)):
        logging.info("mfiIndicator %f @ %f, quitting", mfiIndicator, lastPrice)
        return
    bbLowIndicator = bb.bollinger_lband_indicator()[-1]
    if bbLowIndicator == 0:
        logging.info("bbLowIndicator %d @ %f, quitting", bbLowIndicator, lastPrice)
        return
    volume = common.getVolume(config.EUR, lastPrice)
    buyID = randint(0, common.MAX_RANGE - 1)
    volDecimals, priceDecimals = kraken.getPairDecimals(kApi=kApi, pair=symbol)
    time.sleep(5)
    txid = kraken.addOrder(kApi, symbol, "buy",
                           volume,
                           price=lastPrice,
                           volumeDecimals=volDecimals, priceDecimals=priceDecimals,
                           expiration=config.expiration,
                           userref=buyID)
    logging.info("limit order vol %f, price %f, txID %s", volume, lastPrice, txid)
    if txid is None:
        logging.error("buy order failed %f, %f", lastPrice, volume)
        return
    with open(txLogFile, "a") as txFile:
        txFile.write(txid)
        txFile.write("\n")


if __name__ == "__main__":
    person, symbol, logFile, txFile = common.processInputArgs(sys.argv)
    common.checkOrCreateFileName(logFile)
    common.checkOrCreateFileName(txFile)
    logging.basicConfig(filename=logFile, level=logging.INFO)
    logging.info("###### bbmfibuyer for {} on {}".format(symbol, datetime.now()))
    main(txFile, person, symbol)
