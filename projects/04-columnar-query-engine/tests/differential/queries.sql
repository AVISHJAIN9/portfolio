-- 04 Columnar Query Engine Comprehensive Differential SQL Test Suite (105 Distinct Queries)

-- category: Basic Scans, Projections, and Arithmetic
SELECT id, amount, region FROM sales WHERE amount > 500;
-- category: Basic Scans, Projections, and Arithmetic
SELECT id, amount * 2 AS double_amt, region FROM sales WHERE amount < 200;
-- category: Basic Scans, Projections, and Arithmetic
SELECT id, amount + 50 AS added_amt, category FROM sales WHERE amount BETWEEN 100 AND 300;
-- category: Basic Scans, Projections, and Arithmetic
SELECT id, (amount - 10) / 2 AS calc_val FROM sales WHERE id < 1000;
-- category: Basic Scans, Projections, and Arithmetic
SELECT id, region, category, status FROM sales WHERE status = 'Completed';
-- category: Basic Scans, Projections, and Arithmetic
SELECT id, region, category FROM sales WHERE region = 'US-East' AND category = 'Electronics';
-- category: Basic Scans, Projections, and Arithmetic
SELECT id, amount FROM sales WHERE region = 'EU-Central' OR category = 'Books';
-- category: Basic Scans, Projections, and Arithmetic
SELECT id, amount, status FROM sales WHERE NOT (status = 'Cancelled');
-- category: Basic Scans, Projections, and Arithmetic
SELECT id, region, amount FROM sales WHERE status IN ('Completed', 'Pending');
-- category: Basic Scans, Projections, and Arithmetic
SELECT id, notes FROM sales WHERE notes = 'note_10';
-- category: Basic Scans, Projections, and Arithmetic
SELECT id, amount * 3 - 50 AS complex_expr FROM sales WHERE id <= 500;
-- category: Basic Scans, Projections, and Arithmetic
SELECT id, amount, (amount > 500) AS is_large FROM sales WHERE id <= 200;
-- category: Basic Scans, Projections, and Arithmetic
SELECT id, region FROM sales WHERE region != 'US-East' AND id <= 400;
-- category: Basic Scans, Projections, and Arithmetic
SELECT id, amount FROM sales WHERE amount >= 250 AND amount <= 750 AND id <= 1000;
-- category: Basic Scans, Projections, and Arithmetic
SELECT id, category FROM sales WHERE category IN ('Toys', 'Home') AND id <= 600;
-- category: Basic Scans, Projections, and Arithmetic
SELECT id, status FROM sales WHERE status NOT IN ('Cancelled') AND id <= 800;
-- category: Basic Scans, Projections, and Arithmetic
SELECT id, amount, region FROM sales WHERE amount < 150 OR amount > 850;
-- category: Basic Scans, Projections, and Arithmetic
SELECT id, (amount + 100) * 2 AS boosted FROM sales WHERE id <= 300;
-- category: Basic Scans, Projections, and Arithmetic
SELECT id, amount / 10 AS tenth_amt FROM sales WHERE amount > 900;
-- category: Basic Scans, Projections, and Arithmetic
SELECT id, status, category FROM sales WHERE (category = 'Electronics' OR category = 'Books') AND status = 'Pending';

-- category: Three-Valued NULL Semantics & Filtering
SELECT id, region, notes FROM sales WHERE notes IS NULL;
-- category: Three-Valued NULL Semantics & Filtering
SELECT id, region, notes FROM sales WHERE notes IS NOT NULL;
-- category: Three-Valued NULL Semantics & Filtering
SELECT id, COALESCE(notes, 'unassigned') AS clean_note FROM sales WHERE id <= 500;
-- category: Three-Valued NULL Semantics & Filtering
SELECT COUNT(notes) AS non_null_notes FROM sales;
-- category: Three-Valued NULL Semantics & Filtering
SELECT COUNT(*) AS total_sales_count FROM sales;
-- category: Three-Valued NULL Semantics & Filtering
SELECT region, COUNT(notes) AS region_notes_count FROM sales GROUP BY region;
-- category: Three-Valued NULL Semantics & Filtering
SELECT category, COUNT(notes) AS cat_notes FROM sales WHERE notes IS NOT NULL GROUP BY category;
-- category: Three-Valued NULL Semantics & Filtering
SELECT status, COUNT(*) AS null_note_status_count FROM sales WHERE notes IS NULL GROUP BY status;
-- category: Three-Valued NULL Semantics & Filtering
SELECT id, amount, notes FROM sales WHERE amount > 800 AND notes IS NULL;
-- category: Three-Valued NULL Semantics & Filtering
SELECT id, amount, notes FROM sales WHERE amount < 100 AND notes IS NOT NULL;
-- category: Three-Valued NULL Semantics & Filtering
SELECT id, amount, notes FROM sales WHERE amount IS NULL;
-- category: Three-Valued NULL Semantics & Filtering
SELECT id, amount, notes FROM sales WHERE amount IS NOT NULL AND id <= 1000;
-- category: Three-Valued NULL Semantics & Filtering
SELECT id, region, notes FROM sales WHERE region IS NULL;
-- category: Three-Valued NULL Semantics & Filtering
SELECT id, region, notes FROM sales WHERE region IS NOT NULL AND id <= 500;
-- category: Three-Valued NULL Semantics & Filtering
SELECT id, customer_id FROM sales WHERE customer_id IS NULL;
-- category: Three-Valued NULL Semantics & Filtering
SELECT id, customer_id FROM sales WHERE customer_id IS NOT NULL AND id <= 500;
-- category: Three-Valued NULL Semantics & Filtering
SELECT COUNT(amount) AS non_null_amount_count FROM sales;
-- category: Three-Valued NULL Semantics & Filtering
SELECT COUNT(region) AS non_null_region_count FROM sales;
-- category: Three-Valued NULL Semantics & Filtering
SELECT COUNT(customer_id) AS non_null_cust_count FROM sales;
-- category: Three-Valued NULL Semantics & Filtering
SELECT SUM(amount) AS sum_with_nulls FROM sales;
-- category: Three-Valued NULL Semantics & Filtering
SELECT AVG(amount) AS avg_with_nulls FROM sales;
-- category: Three-Valued NULL Semantics & Filtering
SELECT MIN(amount) AS min_with_nulls FROM sales;
-- category: Three-Valued NULL Semantics & Filtering
SELECT MAX(amount) AS max_with_nulls FROM sales;
-- category: Three-Valued NULL Semantics & Filtering
SELECT region, SUM(amount) AS null_grouped_sum FROM sales GROUP BY region;
-- category: Three-Valued NULL Semantics & Filtering
SELECT category, AVG(amount) AS null_grouped_avg FROM sales GROUP BY category;

-- category: Aggregations and GROUP BY
SELECT region, SUM(amount) AS total_sales FROM sales GROUP BY region;
-- category: Aggregations and GROUP BY
SELECT region, AVG(amount) AS avg_sales, MIN(amount) AS min_amt, MAX(amount) AS max_amt FROM sales GROUP BY region;
-- category: Aggregations and GROUP BY
SELECT category, COUNT(*) AS category_order_count FROM sales GROUP BY category;
-- category: Aggregations and GROUP BY
SELECT region, category, SUM(amount) AS regional_cat_sales FROM sales GROUP BY region, category;
-- category: Aggregations and GROUP BY
SELECT region, status, category, COUNT(*) AS slice_count FROM sales GROUP BY region, status, category;
-- category: Aggregations and GROUP BY
SELECT status, SUM(amount) AS status_total FROM sales WHERE amount > 500 GROUP BY status;
-- category: Aggregations and GROUP BY
SELECT category, AVG(amount) AS filtered_avg FROM sales WHERE region = 'US-West' GROUP BY category;
-- category: Aggregations and GROUP BY
SELECT region, category, SUM(amount * 2) AS doubled_regional_sales FROM sales GROUP BY region, category;
-- category: Aggregations and GROUP BY
SELECT customer_id, COUNT(*) AS customer_order_count FROM sales WHERE customer_id <= 100 GROUP BY customer_id;
-- category: Aggregations and GROUP BY
SELECT customer_id, SUM(amount) AS cust_total FROM sales WHERE customer_id <= 50 GROUP BY customer_id;
-- category: Aggregations and GROUP BY
SELECT category, status, COUNT(*) AS cat_stat_count FROM sales GROUP BY category, status;
-- category: Aggregations and GROUP BY
SELECT region, MIN(amount) AS min_reg_amt FROM sales GROUP BY region;
-- category: Aggregations and GROUP BY
SELECT region, MAX(amount) AS max_reg_amt FROM sales GROUP BY region;
-- category: Aggregations and GROUP BY
SELECT status, COUNT(DISTINCT category) AS distinct_cats_per_status FROM sales GROUP BY status;
-- category: Aggregations and GROUP BY
SELECT category, SUM(COALESCE(amount, 0)) AS coalesced_sum FROM sales GROUP BY category;
-- category: Aggregations and GROUP BY
SELECT customer_id, AVG(amount) AS cust_avg FROM sales WHERE customer_id <= 20 GROUP BY customer_id;
-- category: Aggregations and GROUP BY
SELECT region, status, AVG(amount) AS reg_stat_avg FROM sales GROUP BY region, status;
-- category: Aggregations and GROUP BY
SELECT category, COUNT(notes) AS noted_count FROM sales GROUP BY category;
-- category: Aggregations and GROUP BY
SELECT status, SUM(amount) AS completed_sum FROM sales WHERE status = 'Completed' GROUP BY status;
-- category: Aggregations and GROUP BY
SELECT region, COUNT(*) AS total_txns FROM sales GROUP BY region;

-- category: HAVING Filters
SELECT region, SUM(amount) AS big_regions FROM sales GROUP BY region HAVING SUM(amount) > 1000000;
-- category: HAVING Filters
SELECT category, COUNT(*) AS popular_categories FROM sales GROUP BY category HAVING COUNT(*) > 15000;
-- category: HAVING Filters
SELECT region, AVG(amount) AS high_avg_region FROM sales GROUP BY region HAVING AVG(amount) > 400;
-- category: HAVING Filters
SELECT region, category, SUM(amount) AS sub_sum FROM sales GROUP BY region, category HAVING SUM(amount) > 500000;
-- category: HAVING Filters
SELECT status, COUNT(*) AS status_cnt FROM sales GROUP BY status HAVING COUNT(*) > 20000;
-- category: HAVING Filters
SELECT region, COUNT(notes) AS noted_regions FROM sales GROUP BY region HAVING COUNT(notes) > 10000;
-- category: HAVING Filters
SELECT category, MIN(amount) AS cat_min FROM sales GROUP BY category HAVING MIN(amount) <= 15;
-- category: HAVING Filters
SELECT region, MAX(amount) AS cat_max FROM sales GROUP BY region HAVING MAX(amount) >= 990;
-- category: HAVING Filters
SELECT status, AVG(amount) AS status_avg FROM sales GROUP BY status HAVING AVG(amount) BETWEEN 400 AND 600;
-- category: HAVING Filters
SELECT region, category, COUNT(*) AS dense_bucket FROM sales WHERE amount > 500 GROUP BY region, category HAVING COUNT(*) > 5000;

-- category: DISTINCT Clauses
SELECT DISTINCT region FROM sales;
-- category: DISTINCT Clauses
SELECT DISTINCT category FROM sales;
-- category: DISTINCT Clauses
SELECT DISTINCT status FROM sales;
-- category: DISTINCT Clauses
SELECT DISTINCT region, category FROM sales;
-- category: DISTINCT Clauses
SELECT DISTINCT category, status FROM sales;
-- category: DISTINCT Clauses
SELECT COUNT(DISTINCT region) AS distinct_regions FROM sales;
-- category: DISTINCT Clauses
SELECT COUNT(DISTINCT category) AS distinct_categories FROM sales;
-- category: DISTINCT Clauses
SELECT COUNT(DISTINCT tier) AS distinct_tiers FROM customers;
-- category: DISTINCT Clauses
SELECT DISTINCT tier FROM customers;
-- category: DISTINCT Clauses
SELECT DISTINCT region FROM sales WHERE amount > 900;

-- category: JOIN Types
SELECT sales.id, sales.amount, customers.tier FROM sales JOIN customers ON sales.customer_id = customers.id WHERE sales.id <= 500;
-- category: JOIN Types
SELECT sales.id, sales.region, customers.tier FROM sales JOIN customers ON sales.customer_id = customers.id WHERE customers.tier = 'VIP' AND sales.id <= 1000;
-- category: JOIN Types
SELECT customers.tier, SUM(sales.amount) AS tier_spending FROM sales JOIN customers ON sales.customer_id = customers.id GROUP BY customers.tier;
-- category: JOIN Types
SELECT customers.tier, AVG(sales.amount) AS tier_avg FROM sales JOIN customers ON sales.customer_id = customers.id GROUP BY customers.tier;
-- category: JOIN Types
SELECT sales.region, customers.tier, COUNT(*) AS regional_tier_orders FROM sales JOIN customers ON sales.customer_id = customers.id GROUP BY sales.region, customers.tier;
-- category: JOIN Types
SELECT sales.id, sales.amount, customers.tier FROM sales LEFT JOIN customers ON sales.customer_id = customers.id WHERE sales.id <= 200;
-- category: JOIN Types
SELECT sales.id, customers.tier, sales.notes FROM sales JOIN customers ON sales.customer_id = customers.id WHERE sales.notes IS NULL AND sales.id <= 500;
-- category: JOIN Types
SELECT customers.tier, COUNT(sales.notes) AS noted_orders_per_tier FROM sales JOIN customers ON sales.customer_id = customers.id GROUP BY customers.tier;
-- category: JOIN Types
SELECT sales.category, customers.tier, SUM(sales.amount) AS cat_tier_sales FROM sales JOIN customers ON sales.customer_id = customers.id GROUP BY sales.category, customers.tier;
-- category: JOIN Types
SELECT sales.id, sales.amount FROM sales JOIN customers ON sales.customer_id = customers.id WHERE customers.tier IN ('VIP', 'Platinum') AND sales.amount > 750 AND sales.id <= 1000;

-- category: ORDER BY and LIMIT
SELECT id, amount, region FROM sales ORDER BY amount DESC NULLS LAST, id ASC LIMIT 20;
-- category: ORDER BY and LIMIT
SELECT id, region, category, amount FROM sales ORDER BY region ASC NULLS FIRST, amount DESC NULLS LAST, id ASC LIMIT 30;
-- category: ORDER BY and LIMIT
SELECT id, amount, notes FROM sales ORDER BY notes ASC NULLS FIRST, id ASC LIMIT 25;
-- category: ORDER BY and LIMIT
SELECT id, amount, notes FROM sales ORDER BY notes DESC NULLS LAST, id ASC LIMIT 25;
-- category: ORDER BY and LIMIT
SELECT id, amount, customer_id FROM sales ORDER BY customer_id ASC NULLS FIRST, amount DESC NULLS LAST, id ASC LIMIT 20;
-- category: ORDER BY and LIMIT
SELECT region, SUM(amount) AS reg_tot FROM sales GROUP BY region ORDER BY reg_tot DESC NULLS LAST;
-- category: ORDER BY and LIMIT
SELECT category, AVG(amount) AS cat_avg FROM sales GROUP BY category ORDER BY cat_avg ASC NULLS FIRST;
-- category: Edge Cases
SELECT * FROM empty_table;
-- category: Edge Cases
SELECT COUNT(*) AS empty_count FROM empty_table;
-- category: Edge Cases
SELECT val, val * 2 AS doubled FROM single_row_table;

-- category: WINDOW_FUNCTIONS
SELECT id, amount, ROW_NUMBER() OVER (PARTITION BY region ORDER BY amount) FROM sales;
-- category: WINDOW_FUNCTIONS
SELECT id, region, RANK() OVER (PARTITION BY region ORDER BY amount DESC) FROM sales;
-- category: WINDOW_FUNCTIONS
SELECT id, category, DENSE_RANK() OVER (PARTITION BY category ORDER BY id) FROM sales;
-- category: WINDOW_FUNCTIONS
SELECT id, amount, SUM(amount) OVER (PARTITION BY region) FROM sales;
-- category: WINDOW_FUNCTIONS
SELECT id, amount, AVG(amount) OVER (PARTITION BY category) FROM sales;
-- category: WINDOW_FUNCTIONS
SELECT id, LAG(amount, 1) OVER (PARTITION BY region ORDER BY id) FROM sales;
-- category: WINDOW_FUNCTIONS
SELECT id, LEAD(amount, 1) OVER (PARTITION BY category ORDER BY id) FROM sales;
-- category: WINDOW_FUNCTIONS
SELECT id, FIRST_VALUE(amount) OVER (PARTITION BY region ORDER BY id) FROM sales;
-- category: WINDOW_FUNCTIONS
SELECT id, LAST_VALUE(amount) OVER (PARTITION BY region ORDER BY id) FROM sales;
-- category: WINDOW_FUNCTIONS
SELECT id, NTILE(4) OVER (ORDER BY amount) FROM sales;
