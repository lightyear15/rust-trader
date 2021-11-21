

import sys
import logging
from datetime import datetime

import common
import config


def main(txLogFile, db_conn, person, symbol, price_decimals):
    open_orders = []
    transactions = []
    keys = config.keys[person]
    with open(txLogFile, "r+") as txFile:
        transactions = txFile.read().splitlines()
    for tx in transactions:
        # logging.info("processing order # %s #", tx)
        if tx == "":
            continue
        status, ttype, price, vol, txID = common.queryOrder(keys, tx)
        # a take profit order closed
        if status == "closed" and ttype == "sell":
            logging.info("%s sell closed order", tx)
            buyRef = txID - common.MAX_RANGE
            common.recordTransaction(db_conn)
        elif status == "closed" and ttype == "buy":
            logging.info("%s buy closed order", tx)
            common.recordTransaction(db_conn)
            tp_price, tp_volume = computeTakeProfit(person, price, vol)
            sellID = txID + common.MAX_RANGE
            tp_txid = common.addOrder(keys, symbol, "sell", tp_volume, price=tp_price,
                                      price_decimals=price_decimals, userref=sellID)
            logging.info("taking profit -> %s", tp_txid)
            if tp_txid is not None:
                open_orders.append(tp_txid)
            else:
                logging.error("order failed %f @ %f", tp_volume, tp_price)
                open_orders.append(tx)
        elif status == "open":
            # logging.info("%s open order", tx)
            open_orders.append(tx)
        else:
            logging.warning("%s unknown status %s", tx, status)
    with open(txLogFile, "w") as txFile:
        for tx in open_orders:
            txFile.write(tx)
            txFile.write("\n")
    common.closeDBConn(db_conn)


def computeTakeProfit(person, price, volume):
    takeProfitPrice = price * (1.0 + config.take_profit_factor[person])
    takeProfitVolume = price * volume / takeProfitPrice
    return takeProfitPrice, takeProfitVolume


if __name__ == "__main__":
    person, symbol, log_file, tx_file, decimals = common.processInputArgs(sys.argv)
    common.checkOrCreateFileNames(log_file, tx_file)
    db_conn = common.openDBConn(config.database[person])
    logging.basicConfig(filename=log_file, level=logging.INFO)
    logging.info("###### seller on {} {}".format(symbol, datetime.now()))
    main(tx_file, db_conn, person, symbol, decimals)
