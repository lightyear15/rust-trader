import sys
import os
import pandas
from datetime import datetime
import time
import string
import smtplib
import ssl
from email.message import EmailMessage
from email.policy import EmailPolicy

import config
import common
import kraken
import krakenex
from pykrakenapi import KrakenAPI


months = ["Gennaio", "Febbraio", "Marzo", "Aprile", "Maggio", "Giugno", "Luglio", "Agosto", "Settembre", "Ottobre", "Novembre", "Dicembre"]
hrSymbols = {"xxbtzeur": "btc", "xethzeur": "eth", "atomeur": "atom", "adaeur": "ada"}
reportDecimals = {"xxbtzeur": 8, "xethzeur": 4, "atomeur": 4, "adaeur": 4}
emailSubject = "cripto-resoconto del mese"
messageTemplate = """ciao {person},
anche questo {month} é passato.
Vediamo come é andata questo mese:
{perSymbolMonthlyResume}

In totale quindi hai accumulato:
{perSymbolTotalResume}

Ci si sente il prossimo mese,
   il tuo criptomacinino di fiducia
"""

monthlyResumeTemplate = "{symbol}: hai comprato {volume} ad un prezzo medio di {price} € pagando {fee} € di commissioni"
totalResumeTemplate = "{symbol}: {volume} pagati {purchase} € che ad oggi valgono circa {value} €, un ritorno del {roi} %"


def main(person: str, printIt: bool = False):
    if person in config.email is False:
        return
    api = krakenex.API()
    kApi = KrakenAPI(api)
    personHomeFolder = common.buildBaseFolderName(person)
    currentMonth = datetime.now().month - 1
    monthlyResumeList = []
    totalResumeList = []
    for filename in os.listdir(personHomeFolder):
        f = os.path.join(personHomeFolder, filename)
        if os.path.isfile(f) is False:
            continue
        name, ext = os.path.splitext(filename)
        if ext != ".csv":
            continue
        # name is supposed to be the symbol
        _, lastPrice = kraken.getLastCandles(kApi, symbol=name)
        df = pandas.read_csv(f, index_col="date", parse_dates=True, date_parser=common.dateParser)
        if df.empty is False:
            volume = df["volume"].sum()
            purchase = (df["price"] * df["volume"]).sum()
            value = volume * lastPrice
            totalResumeList.append(totalResumeTemplate.format(
                symbol=hrSymbols[name],
                volume=round(volume, reportDecimals[name]),
                purchase=round(purchase, 2),
                value=round(value, 2),
                roi=round((value - purchase) / purchase * 100, 2),
            )
            )
            currentMonthDf = df[df.index.month == currentMonth]
            if currentMonthDf.empty is False:
                volume = currentMonthDf["volume"].sum()
                price = (currentMonthDf["price"] * currentMonthDf["volume"]).sum() / volume
                monthlyResumeList.append(monthlyResumeTemplate.format(
                    symbol=hrSymbols[name],
                    volume=round(volume, reportDecimals[name]),
                    price=round(price, 2),
                    fee=round(currentMonthDf["fees"].sum(), 2),
                )
                )
        time.sleep(2)
    totalResume = "\n".join(totalResumeList)
    monthlyResume = "\n".join(monthlyResumeList)
    emailContent = messageTemplate.format(
        person=string.capwords(person),
        month=months[currentMonth - 1],
        perSymbolMonthlyResume=monthlyResume,
        perSymbolTotalResume=totalResume
    )

    if printIt is True:
        print(emailContent)
        return

    # sending mail
    context = ssl.create_default_context()
    port = 465  # For SSL
    with smtplib.SMTP_SSL(config.mailer["server"], port, context=context) as server:
        server.login(config.mailer["address"], config.mailer["password"])
        msg = EmailMessage(policy=EmailPolicy(utf8=True))
        msg.set_content(emailContent)
        msg["Subject"] = emailSubject
        msg["From"] = config.mailer["address"]
        msg["To"] = config.email[person]
        server.send_message(msg)
    return


if __name__ == "__main__":
    args = sys.argv
    printIt = False
    if len(args) < 2:
        print("missing input args <person>")
        sys.exit()
    if len(args) >= 3 and args[2] == "print":
        printIt = True
    person = args[1]
    main(person, printIt)
