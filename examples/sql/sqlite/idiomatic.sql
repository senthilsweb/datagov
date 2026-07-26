INSERT INTO customers (id, state) VALUES (1, 'CA') ON CONFLICT (id) DO NOTHING;
