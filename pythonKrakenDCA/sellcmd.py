import sys
from datetime import datetime

import common
import config

def main(person: str, coin: str, volume: str, price: str, reference: int):
    log_file, tx_file = common.buildFileNames(person, coin)
    sellID = reference + common.MAX_RANGE
    tp_txid = common.addRawOrder(keys, coin, "sell", volume, price=price, userref=sellID)
    if tp_txid is None:
        print("sending order has failed... exiting")
        return
    print("order sent successfully")
    with open(txLogFile, "a") as txFile:
            txFile.write(tp_txid)
            txFile.write("\n")
            print("order", tp_txid, "appended to ", tx_file)


if __name__ == "__main__":
    if len(sys.argv) < 6:
        print("missing argument(s) - usage is")
        print("./sellcmd.py person coin volume price reference")
        sys.exit(1)
    person = sys.argv[1]
    coin = sys.argv[2]
    volume = float(sys.argv[3])
    price = float(sys.argv[4])
    reference = int(sys.argv[5])
    main(person, coin, volume, price, reference)
