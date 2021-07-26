
with opening as (
select tstamp, symbol, price, volume, id
from transactions
where reference = 0
),
closing as (
select symbol, price, volume, id, reference
from transactions
where reference <> 0
)

select opening.tstamp, opening.symbol, (closing.price - opening.price) / opening.price * 100 as ror
from closing
join opening on closing.reference = opening.id
order by opening.tstamp
