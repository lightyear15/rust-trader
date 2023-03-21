with b as
(select * from transactions where side = 'Buy'),
s as 
(select * from transactions where side = 'Sell'),
inter as 
(select b.exchange, b.symbol, 
(case when s.tstamp is null then b.tstamp else s.tstamp end) as tstamp, 
(case b.fees_asset when 'EUR' then b.fees + coalesce(s.fees, 0.0) when 'BNB' then (b.fees + coalesce(s.fees, 0.0)) * 280.0 else  -1000000.0 end) as fees,
(case when s.id is null then b.price else null end) as price,
b.volume - s.volume as volume, 
s.price * s.volume - b.price * b.volume + s.price*(b.volume - s.volume) as profit, 
s.tstamp - b.tstamp as elapsed
from b
left join s on b.id = s.reference and b.exchange = s.exchange and b.symbol = s.symbol
)
select *
from inter
where extract(month from current_timestamp) = extract(month from tstamp) and extract(year from current_timestamp) = extract(year from tstamp)
order by tstamp desc



--- find dangling buy event 
with b as
(select * from transactions where side = 'Buy'),
s as 
(select * from transactions where side = 'Sell')
select b.exchange, b.symbol, b.tstamp, b.price, b.id, b.volume, b.volume * b.price as cost,
(case b.exchange when  'binance' then b.volume * 0.99 when 'kraken' then b.volume * 0.95 else -10000 end) as estimated_sell_volume,
(case b.exchange when  'binance' then b.price * 1.01 when 'kraken' then b.price * 1.05 else -10000 end) as estimated_sell_price
from b
left join s on b.id = s.reference and b.exchange = s.exchange and b.symbol = s.symbol
where s.id is null
order by b.tstamp asc



--- insert missing lines
insert into transactions 
      ( exchange,    symbol,                tstamp,  side,    price,   volume,      id,fees, fees_asset, reference)
values('binance', 'BETHBUSD', '2023-03-12 19:02:33','Sell',  1497.49,  0.0348, 92857971, 0.0,      'BNB', 33417)


delete from transactions
where id = 3092425



--- find dangling sell event 
with b as
(select * from transactions where side = 'Buy'),
s as 
(select * from transactions where side = 'Sell')
select *
from s
left join b on b.id = s.reference and b.exchange = s.exchange and b.symbol = s.symbol
where b.id is null
order by s.tstamp asc



--- monthly resume
with buyT as
(select * from transactions where side = 'Buy'),
sellT as 
(select * from transactions where side = 'Sell'),
inter as 
(select extract(year from sellT.tstamp) as year, extract(month from sellT.tstamp) as month, buyT.fees_asset, 
avg(sellT.tstamp - buyT.tstamp) as elapsed, sum(buyT.fees + sellT.fees) as fees, 
sum(sellT.price * sellT.volume - buyT.price * buyT.volume + sellT.price*(buyT.volume - sellT.volume)) as profit
from buyT
inner join sellT on buyT.id = sellT.reference and buyT.exchange = sellT.exchange and buyT.symbol = sellT.symbol
group by year, month, buyT.fees_asset
order by year, month
)
select year, month, 
avg(elapsed), sum(profit - (case fees_asset  when 'BNB' then 280.0 when 'EUR' then 1.0 end)* fees) as profit
from inter
group by year, month
order by year, month



--- resume binance
with b as
(select * from transactions where side = 'Buy' and exchange = 'binance' and fees_asset = 'BNB'),
s as 
(select * from transactions where side = 'Sell' and exchange = 'binance' and fees_asset = 'BNB'),
i as 
(select s.tstamp as tstamp, b.fees + s.fees as fees, 
s.price * s.volume - b.price * b.volume + s.price*(b.volume - s.volume) as profit,
s.tstamp - b.tstamp as elapsed
from b
inner join s on b.id = s.reference and b.exchange = s.exchange and b.symbol = s.symbol
)
select 
extract(year from tstamp) as year, extract(month from tstamp) as month, 
sum(profit) - sum(fees) * 280.0 as net_profit, avg(elapsed) as elapsed,
sum(fees) as native_fees, count(*) as counter
from i
group by extract(year from tstamp), extract(month from tstamp)
order by extract(year from tstamp), extract(month from tstamp)



--- resume kraken
with b as
(select * from transactions where side = 'Buy' and exchange = 'kraken' and fees_asset = 'EUR'),
s as 
(select * from transactions where side = 'Sell' and exchange = 'kraken' and fees_asset = 'EUR'),
i as 
(select s.tstamp as tstamp, b.fees + s.fees as fees, 
s.price * s.volume - b.price * b.volume + s.price*(b.volume - s.volume) as profit,
s.tstamp - b.tstamp as elapsed
from b
inner join s on b.id = s.reference and b.exchange = s.exchange and b.symbol = s.symbol
)
select extract(year from tstamp) as year, extract(month from tstamp) as month, sum(profit) - sum(fees) as net_profit, avg(elapsed) as elapsed, count(*) as counter
from i
group by extract(year from tstamp), extract(month from tstamp)
order by extract(year from tstamp), extract(month from tstamp)




------------ old

---- total resume
with b as
(select * from transactions where side = 'Buy'),
s as 
(select * from transactions where side = 'Sell'),
i as 
(select b.exchange, b.symbol, 
(case when s.tstamp is null then b.tstamp else s.tstamp end) as tstamp, 
(case b.fees_asset when 'EUR' then b.fees + coalesce(s.fees, 0.0) when 'BNB' then (b.fees + coalesce(s.fees, 0.0)) * 280.0 else  -1000000.0 end) as fees,
(case when s.id is null then b.price else null end) as price,
b.volume - s.volume as volume, 
s.price * s.volume - b.price * b.volume + s.price*(b.volume - s.volume) as profit,
s.tstamp - b.tstamp as elapsed
from b
left join s on b.id = s.reference and b.exchange = s.exchange and b.symbol = s.symbol
--where extract(month from current_timestamp) = extract(month from b.tstamp) and extract(year from current_timestamp) = extract(year from b.tstamp)
)
select symbol, sum(profit) as profit,
avg(elapsed) as elapsed,
sum(fees) as fees, count(*) as counter,
sum(volume) as volume
from i
group by symbol















with b as (select * from transactions where side = 'Buy'),
s as (select * from transactions where side = 'Sell'),
i as (select b.exchange, b.symbol, b.tstamp as buyTstamp, b.price as buyPrice, b.volume as buyVolume, b.fees as buyFees,
s.tstamp as sellTstamp, s.price as sellPrice, s.volume as sellVolume, s.fees as sellFees
from b
left join s on b.id = s.reference and b.exchange = s.exchange and b.symbol = s.symbol and b.fees_asset = s.fees_asset
)
select
i.exchange, i.symbol,
(case when i.sellTstamp is null then buyTstamp else sellTstamp end) as tstamp,
buyFees + coalesce(sellFees, 0.0) as fees,
(case when sellPrice is null then buyPrice else null end) as price,
buyVolume - sellVolume as volume,
sellPrice * sellVolume - buyPrice * buyVolume + sellPrice*(buyVolume - sellVolume) as profit,
sellTstamp - buyTstamp as elapsed
from i
order by tstamp desc
limit 10



