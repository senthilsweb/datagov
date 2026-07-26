SELECT t.state, COUNT(*) AS total FROM customers AS t INNER JOIN orders AS o ON t.id = o.customer_id GROUP BY t.state ORDER BY t.state
