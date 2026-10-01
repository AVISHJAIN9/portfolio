-- 04 Columnar Query Engine Comprehensive Differential SQL Test Suite (65 Distinct Queries)

-- 1-10: Basic Scans, Projections, and Arithmetic
SELECT id, amount, region FROM sales WHERE amount > 500;
SELECT id, amount * 2 AS double_amt, region FROM sales WHERE amount < 200;
SELECT id, amount + 50 AS added_amt, category FROM sales WHERE amount BETWEEN 100 AND 300;
SELECT id, (amount - 10) / 2 AS calc_val FROM sales WHERE id < 1000;
SELECT id, region, category, status FROM sales WHERE status = 'Completed';
SELECT id, region, category FROM sales WHERE region = 'US-East' AND category = 'Electronics';
SELECT id, amount FROM sales WHERE region = 'EU-Central' OR category = 'Books';
SELECT id, amount, status FROM sales WHERE NOT (status = 'Cancelled');
SELECT id, region, amount FROM sales WHERE status IN ('Completed', 'Pending');
SELECT id, notes FROM sales WHERE notes = 'note_10';

-- 11-20: NULL Semantics & Filtering
SELECT id, region, notes FROM sales WHERE notes IS NULL;
SELECT id, region, notes FROM sales WHERE notes IS NOT NULL;
SELECT id, COALESCE(notes, 'unassigned') AS clean_note FROM sales WHERE id <= 500;
SELECT COUNT(notes) AS non_null_notes FROM sales;
SELECT COUNT(*) AS total_sales_count FROM sales;
SELECT region, COUNT(notes) AS region_notes_count FROM sales GROUP BY region;
SELECT category, COUNT(notes) AS cat_notes FROM sales WHERE notes IS NOT NULL GROUP BY category;
SELECT status, COUNT(*) AS null_note_status_count FROM sales WHERE notes IS NULL GROUP BY status;
SELECT id, amount, notes FROM sales WHERE amount > 800 AND notes IS NULL;
SELECT id, amount, notes FROM sales WHERE amount < 100 AND notes IS NOT NULL;

-- 21-30: Aggregations and GROUP BY
SELECT region, SUM(amount) AS total_sales FROM sales GROUP BY region;
SELECT region, AVG(amount) AS avg_sales, MIN(amount) AS min_amt, MAX(amount) AS max_amt FROM sales GROUP BY region;
SELECT category, COUNT(*) AS category_order_count FROM sales GROUP BY category;
SELECT region, category, SUM(amount) AS regional_cat_sales FROM sales GROUP BY region, category;
SELECT region, status, category, COUNT(*) AS slice_count FROM sales GROUP BY region, status, category;
SELECT status, SUM(amount) AS status_total FROM sales WHERE amount > 500 GROUP BY status;
SELECT category, AVG(amount) AS filtered_avg FROM sales WHERE region = 'US-West' GROUP BY category;
SELECT region, category, SUM(amount * 2) AS doubled_regional_sales FROM sales GROUP BY region, category;
SELECT customer_id, COUNT(*) AS customer_order_count FROM sales WHERE customer_id <= 100 GROUP BY customer_id;
SELECT customer_id, SUM(amount) AS cust_total FROM sales WHERE customer_id <= 50 GROUP BY customer_id;

-- 31-40: HAVING Filters
SELECT region, SUM(amount) AS big_regions FROM sales GROUP BY region HAVING SUM(amount) > 1000000;
SELECT category, COUNT(*) AS popular_categories FROM sales GROUP BY category HAVING COUNT(*) > 15000;
SELECT region, AVG(amount) AS high_avg_region FROM sales GROUP BY region HAVING AVG(amount) > 400;
SELECT region, category, SUM(amount) AS sub_sum FROM sales GROUP BY region, category HAVING SUM(amount) > 500000;
SELECT status, COUNT(*) AS status_cnt FROM sales GROUP BY status HAVING COUNT(*) > 20000;
SELECT region, COUNT(notes) AS noted_regions FROM sales GROUP BY region HAVING COUNT(notes) > 10000;
SELECT category, MIN(amount) AS cat_min FROM sales GROUP BY category HAVING MIN(amount) <= 15;
SELECT region, MAX(amount) AS cat_max FROM sales GROUP BY region HAVING MAX(amount) >= 990;
SELECT status, AVG(amount) AS status_avg FROM sales GROUP BY status HAVING AVG(amount) BETWEEN 400 AND 600;
SELECT region, category, COUNT(*) AS dense_bucket FROM sales WHERE amount > 500 GROUP BY region, category HAVING COUNT(*) > 5000;

-- 41-50: DISTINCT Clauses
SELECT DISTINCT region FROM sales;
SELECT DISTINCT category FROM sales;
SELECT DISTINCT status FROM sales;
SELECT DISTINCT region, category FROM sales;
SELECT DISTINCT category, status FROM sales;
SELECT COUNT(DISTINCT region) AS distinct_regions FROM sales;
SELECT COUNT(DISTINCT category) AS distinct_categories FROM sales;
SELECT COUNT(DISTINCT tier) AS distinct_tiers FROM customers;
SELECT DISTINCT tier FROM customers;
SELECT DISTINCT region FROM sales WHERE amount > 900;

-- 51-60: JOIN Types (INNER, LEFT OUTER, FILTERED JOINS)
SELECT sales.id, sales.amount, customers.tier FROM sales JOIN customers ON sales.customer_id = customers.id WHERE sales.id <= 500;
SELECT sales.id, sales.region, customers.tier FROM sales JOIN customers ON sales.customer_id = customers.id WHERE customers.tier = 'VIP' AND sales.id <= 1000;
SELECT customers.tier, SUM(sales.amount) AS tier_spending FROM sales JOIN customers ON sales.customer_id = customers.id GROUP BY customers.tier;
SELECT customers.tier, AVG(sales.amount) AS tier_avg FROM sales JOIN customers ON sales.customer_id = customers.id GROUP BY customers.tier;
SELECT sales.region, customers.tier, COUNT(*) AS regional_tier_orders FROM sales JOIN customers ON sales.customer_id = customers.id GROUP BY sales.region, customers.tier;
SELECT sales.id, sales.amount, customers.tier FROM sales LEFT JOIN customers ON sales.customer_id = customers.id WHERE sales.id <= 200;
SELECT sales.id, customers.tier, sales.notes FROM sales JOIN customers ON sales.customer_id = customers.id WHERE sales.notes IS NULL AND sales.id <= 500;
SELECT customers.tier, COUNT(sales.notes) AS noted_orders_per_tier FROM sales JOIN customers ON sales.customer_id = customers.id GROUP BY customers.tier;
SELECT sales.category, customers.tier, SUM(sales.amount) AS cat_tier_sales FROM sales JOIN customers ON sales.customer_id = customers.id GROUP BY sales.category, customers.tier;
SELECT sales.id, sales.amount FROM sales JOIN customers ON sales.customer_id = customers.id WHERE customers.tier IN ('VIP', 'Platinum') AND sales.amount > 750 AND sales.id <= 1000;

-- 61-65: ORDER BY, LIMIT, Empty Table, and Single Row Corner Cases
SELECT id, amount, region FROM sales ORDER BY amount DESC LIMIT 20;
SELECT id, region, category, amount FROM sales ORDER BY region ASC, amount DESC LIMIT 30;
SELECT * FROM empty_table;
SELECT COUNT(*) AS empty_count FROM empty_table;
SELECT val, val * 2 AS doubled FROM single_row_table;
