SELECT t.state, COUNT(*) AS total FROM customers t INNER JOIN orders o ON t.id = o.customer_id GROUP BY t.state ORDER BY t.state;
