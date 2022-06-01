
from typing import Tuple, Optional
from datetime import timedelta, datetime
import logging
from pykrakenapi import KrakenAPI
import pandas
import krakenex

import config


def queryOrder(kApi: KrakenAPI, tx: str) -> Optional[Tuple[str, str, int, float, float, float, datetime]]:
    queryInfo = kApi.query_orders_info(txid=tx)
    tstamp = datetime.fromtimestamp(queryInfo.at[tx, "opentm"])
    status = queryInfo.at[tx, "status"]
    if status == "closed":
        tstamp = datetime.fromtimestamp(queryInfo.at[tx, "closetm"])
    return (status, queryInfo.at[tx, "descr_type"], queryInfo.at[tx, "userref"],
            float(queryInfo.at[tx, "price"]), float(queryInfo.at[tx, "vol"]),
            float(queryInfo.at[tx, "fee"]), tstamp,
            )


def getPairDecimals(kApi: KrakenAPI, pair: str) -> Tuple[int, int]:
    df = kApi.get_tradable_asset_pairs(pair=pair)
    volDecimals = df["lot_decimals"][0]
    priceDecimals = df["pair_decimals"][0]
    return volDecimals, priceDecimals


def addOrder(kApi: KrakenAPI, symbol, direction, volume, price=None,
             volumeDecimals=None, priceDecimals=None,
             expiration: timedelta = None, userref: int = None):
    vol_str = "{:.{prec}f}".format(volume, prec=volumeDecimals)
    price_str = None
    if price is not None:
        price_str = "{:.{prec}f}".format(price, prec=priceDecimals)
    return addRawOrder(kApi, symbol, direction, vol_str, price_str, expiration, userref)


def addRawOrder(kApi: KrakenAPI, symbol: str, direction: str, volume: str, price: str = None,
                expiration: timedelta = None, userref: int = None) -> str:
    ordertype = "limit"
    if price is None:
        ordertype = "market"
    expireSec = None
    timeInForce = "GTC"
    if expiration is not None:
        expireSec = "+{}".format(int(expiration.total_seconds()))
        timeInForce = "GTD"
    result = kApi.add_standard_order(ordertype=ordertype, type=direction, pair=symbol, userref=userref, volume=volume,
                                     price=price, expiretm=expireSec, validate=False, timeinforce=timeInForce,
                                     trigger=None)
    txids = result["txid"]
    if len(txids) != 1:
        logging.error("expecting only one txid, got %d", len(txids))
    return txids[0]


def getLastCandles(kApi: KrakenAPI, symbol: str, interval: timedelta = timedelta(minutes=1)) -> Tuple[pandas.DataFrame, float]:
    interval_minutes = int(interval.total_seconds() / 60.0)
    candles, _ = kApi.get_ohlc_data(pair=symbol, interval=interval_minutes, ascending=True)
    lastPrice = candles.iloc[-1].at["close"]
    candles = candles[:-1]
    return candles, lastPrice


if __name__ == "__main__":
    api = krakenex.API(config.keys["giulioTest"]["key"], config.keys["giulioTest"]["secret"])
    kApi = KrakenAPI(api)
    # candles, lastPrice = getLastCandles(kApi, symbol="xxbtzeur", interval=timedelta(days=1))
    # idx = candles.index
    # limit = datetime.now() - timedelta(days=8)
    # print(limit)
    # cnd = candles[idx >= limit]
    # wg = (cnd["close"] * cnd["volume"]).sum() / cnd["volume"].sum()
    # print(wg)
    # print(candles)
    # print(lastPrice)
    # info = queryOrder(kApi, tx="OAQK32-22OKH-AWZPIT")
    # print (info)
    # if info is not None:
        # status, side, txID, price, volume, fees, tstamp = info
        # print("{},{},{},{}".format(tstamp, price, volume, fees))
    # info = queryOrder(kApi, tx="OQARAP-7NYHB-ZFICMW")
    # print (info)
    # if info is not None:
        # status, side, txID, price, volume, fees, tstamp = info
        # print("{},{},{},{}".format(tstamp, price, volume, fees))
    # info = queryOrder(kApi, tx="OHJKHS-CFY4B-TIIKKB")
    # print (info)
    # if info is not None:
        # status, side, txID, price, volume, fees, tstamp = info
        # print("{},{},{},{}".format(tstamp, price, volume, fees))
    getPairDecimals(kApi=kApi, pair="XXBTZEUR")
