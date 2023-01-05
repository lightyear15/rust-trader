import sys
from datetime import datetime
from enum import Enum
import itertools
import psycopg2
import config


class Report(str, Enum):
    Last = "last"
    Binance = "binance"
    Kraken = "kraken"
    Monthly = "monthly"


BuySellMatchDBQuery = """with b as (select * from transactions where side = 'Buy'),
s as (select * from transactions where side = 'Sell'),
i as (select b.exchange, b.symbol, b.tstamp as buyTstamp, b.price as buyPrice, b.volume as buyVolume, b.fees as buyFees,
s.tstamp as sellTstamp, s.price as sellPrice, s.volume as sellVolume, s.fees as sellFees
from b
left join s on b.id = s.reference and b.exchange = s.exchange and b.symbol = s.symbol and b.fees_asset = s.fees_asset
)
"""


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
            groupedElement[g["exchange"]] = {"profit": g["profit"], "elapsed": g["elapsed"], "counter": g["counter"]}
        grouped.append(groupedElement)
    for g in grouped:
        print(g["date"])
        if "binance" in g:
            print("\t", "binance:", g["binance"]["profit"], "\t", g["binance"]["counter"], "\t", g["binance"]["elapsed"])
        if "kraken" in g:
            print("\t", "kraken:", g["kraken"]["profit"], "\t", g["kraken"]["counter"], "\t", g["kraken"]["elapsed"])

        # print(, int(e["month"])), "\t", e["exchange"], "\t", e["profit"])


reportFunction = {
        Report.Last: lastReport,
        Report.Monthly: monthlyReport,
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


