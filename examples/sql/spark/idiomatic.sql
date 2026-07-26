SELECT id, tag FROM customers LATERAL VIEW EXPLODE(tags) t2 AS tag;
