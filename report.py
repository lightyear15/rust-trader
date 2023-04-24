import sys
from datetime import datetime
from enum import Enum
import itertools
import psycopg2
import config
from currency_converter import CurrencyConverter


class Report(str, Enum):
    Last = "last"
    Binance = "binance"
    Kraken = "kraken"
    Monthly = "monthly"
    Coinly = "coin"
    Tax = "tax"


symbolMaps = {
        "xxbtzeur": "btc", "BTCBUSD": "btc",
        "WBTCBUSD": "wbtc", "wbtceur": "wbtc",
        "ETHBUSD": "eth", "xethzeur": "eth",
        "MKRBUSD": "mkr", "mkreur": "mkr",
        "BETHBUSD": "beth",
}

BuySellMatchDBQuery = """with b as (select * from transactions where side = 'Buy'),
s as (select * from transactions where side = 'Sell'),
i as (select b.exchange, b.symbol, b.tstamp as buyTstamp, b.price as buyPrice, b.volume as buyVolume, b.fees as buyFees,
s.tstamp as sellTstamp, s.price as sellPrice, s.volume as sellVolume, s.fees as sellFees
from b
left join s on b.id = s.reference and b.exchange = s.exchange and b.symbol = s.symbol and b.fees_asset = s.fees_asset
)
"""


def mapCoin(coin):
    if coin in symbolMaps.keys():
        return symbolMaps[coin]
    return coin


def lastReport(dbCursor):
    query = BuySellMatchDBQuery + """,
lastI as (select
i.exchange, i.symbol,
(case when sellTstamp is null then buyTstamp else sellTstamp end) as tstamp,
buyFees + coalesce(sellFees, 0.0) as fees,
(case when sellPrice is null then buyPrice else null end) as price,
buyVolume - sellVolume as volume,
sellPrice * sellVolume - buyPrice * buyVolume + sellPrice*(buyVolume - sellVolume) as profit,
sellTstamp - buyTstamp as elapsed
from i
)
select exchange, symbol, tstamp, price, profit, elapsed
from lastI
where extract(year from tstamp) = %s and extract(month from tstamp) = %s
order by tstamp desc
"""
    entries = []
    dbCursor.execute(query, (datetime.now().year, datetime.now().month))
    rows = dbCursor.fetchall()
    for row in rows:
        entries.append(
                {"exchange": row[0],
                    "symbol": row[1],
                    "tstamp": row[2],
                    "price": row[3],
                    "profit": row[4],
                    "elapsed": row[5],
                 })
    completed = filter(lambda entry: entry["profit"] is not None, entries)
    pending = filter(lambda entry: entry["profit"] is None, entries)
    print("pending trades")
    for e in pending:
        print(e["exchange"], "\t", e["symbol"], "\t", e["tstamp"], "\t", e["price"])
    print("last completed trades")
    cnt = 0
    for e in completed:
        print(e["exchange"], "\t", e["symbol"], "\t", e["tstamp"], "\t", e["profit"], "\t", e["elapsed"])
        cnt += 1
        if cnt == 5:
            return


def monthlyReport(dbCursor):
    query = BuySellMatchDBQuery + """select
extract(year from sellTstamp) as year, extract(month from sellTstamp) as month, exchange,
sum(buyFees + sellFees) as fees,
sum(sellPrice * sellVolume - buyPrice * buyVolume + sellPrice*(buyVolume - sellVolume)) as profit,
count(*) as counter,
avg(sellTstamp - buyTstamp) as elapsed
from i
where sellTstamp is not null
group by year, month, exchange
order by year, month, exchange
"""
    entries = []
    dbCursor.execute(query)
    rows = dbCursor.fetchall()
    for row in rows:
        entries.append(
                {"date": "{}-{:02d}".format(int(row[0]), int(row[1])),
                    "exchange": row[2],
                    "fees": row[3],
                    "profit": row[4],
                    "counter": row[5],
                    "elapsed": row[6],
                 })
    grouped = []
    for k, group in itertools.groupby(entries, key=lambda e: e["date"]):
        groupedElement = {"date": k}
        for g in group:
            groupedElement[g["exchange"]] = {"profit": g["profit"], "elapsed": g["elapsed"], "counter": g["counter"], "fees": g["fees"]}
        grouped.append(groupedElement)

    exchanges = ["binance", "kraken"]
    total = dict.fromkeys(exchanges, (0.0, 0.0, 0))
    for entry in grouped:
        print(entry["date"])
        for exchange in exchanges:
            if exchange not in entry:
                continue
            data = entry[exchange]
            resume = "\t {}:\t{:.2f}\t{:.4f}\t{}\t{}".format(exchange, data["profit"], data["fees"], data["counter"], data["elapsed"])
            (profit, fees, counter) = total[exchange]
            total[exchange] = (profit + float(data["profit"]), fees + float(data["fees"]), counter + int(data["counter"]))
            print(resume)
    print("total")
    for exchange in exchanges:
        (profit, fees, counter) = total[exchange]
        resume = "\t {}:\t{:.2f}\t{:.4f}\t{}".format(exchange, profit, fees, counter)
        print(resume)


def coinlyReport(dbCursor):
    query = BuySellMatchDBQuery + """select symbol,
sum(buyVolume - sellVolume) as volume,
count(*) as counter
from i
where sellTstamp is not null
group by symbol
"""
    dbCursor.execute(query)
    rows = dbCursor.fetchall()
    for row in rows:
        print("{}: \t{:.8f} {}".format(row[0], row[1], row[2]))
    return()


def taxReport(dbCursor):
    c = CurrencyConverter(fallback_on_missing_rate=True)
    year = datetime.today().year - 1
    query = """
select symbol, side, tstamp, price, volume
from transactions
where tstamp between '%(year)s-01-01' and '%(year)s-12-31'
    """
    dbCursor.execute(query, {"year": year})
    rows = dbCursor.fetchall()
    operations = {}
    defaultEntry = {"buyVolume": 0.0, "sellVolume": 0.0, "buyPrice": 0.0, "sellPrice": 0.0}
    for row in rows:
        symbol = row[0]
        side = row[1]
        tstamp = datetime.fromisoformat(str(row[2]))
        price = row[3]
        volume = row[4]
        coin = mapCoin(row[0])

        if coin not in operations.keys():
            operations[coin] = defaultEntry
        op = operations[coin].copy()
        if symbol.endswith("USD"):
            price = c.convert(price, "USD", "EUR", date=tstamp.date())
        if side == "Sell":
            totVolume = op["sellVolume"]
            totPrice = op["sellPrice"]
            newVolume = totVolume + volume
            newPrice = (totPrice * totVolume + price * volume) / newVolume
            op["sellVolume"] = newVolume
            op["sellPrice"] = newPrice
        if side == "Buy":
            totVolume = op["buyVolume"]
            totPrice = op["buyPrice"]
            newVolume = totVolume + volume
            newPrice = (totPrice * totVolume + price * volume) / newVolume
            op["buyVolume"] = newVolume
            op["buyPrice"] = newPrice
        operations[coin] = op
    for (key, op) in operations.items():
        sellValue = op["sellVolume"] * op["sellPrice"]
        buyValue = op["buyVolume"] * op["buyPrice"]
        # print(key, op)
        print(key, "buyValue", buyValue, "sellValue", sellValue, "dif", sellValue - buyValue)
    return()


reportFunction = {
        Report.Last: lastReport,
        Report.Monthly: monthlyReport,
        Report.Coinly: coinlyReport,
        Report.Tax: taxReport,
}


def main(cmd: Report):
    connData = config.DBData
    dbConn = psycopg2.connect(database=connData["database"],
                              user=connData["username"],
                              password=connData["password"],
                              host=connData["hostname"],
                              port=connData["port"])
    with dbConn.cursor() as crs:
        reportFunction[cmd](crs)
    dbConn.close()
    return


if __name__ == "__main__":
    comm = Report.Last
    if len(sys.argv) > 1:
        comm = Report(sys.argv[1])
    main(comm)
