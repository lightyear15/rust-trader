import sys
import logging
from datetime import datetime, timedelta
import pandas
import common
import config
from ta import volume

MFI_THRE = 20

def main(txLogFile, person, symbol, price_decimals):
    orders_count = len(open(txLogFile).readlines())
    if orders_count >= config.max_order[person]:
        return
    interval = timedelta(days=1)
    (candles, lastPrice) = common.getLastCandles(symbol, interval)
    high, low, close, volume = buildSeriess(candles)
    mfi = getMFI(high,low,close,volume)
    lastMfi = mfi.mfi()[-1]
    if lastMfi >= mfi_THRE:
        logging.info("lastMfi %f @ %f, quitting", lastMfi, lastPrice)
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
    return volume.mfiIndicator(high=high, low=low, close=close, volume=volume)


if __name__ == "__main__":
    person, symbol, log_file, tx_file, decimals = common.processInputArgs(sys.argv)
    common.checkOrCreateFileNames(log_file, tx_file)
    logging.basicConfig(filename=log_file, level=logging.INFO)
    logging.info("###### mfibuyer for on {} {}".format(symbol, datetime.now()))
    main(tx_file, person, symbol, decimals)
