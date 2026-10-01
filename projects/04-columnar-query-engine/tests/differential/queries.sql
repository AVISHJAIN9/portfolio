-- 50+ Canonical Differential Queries for Columnar Query Engine vs DuckDB
-- Covers: Filters, Aggregates, Multi-Key Group By, Having, Order By, Limit, Distinct, Nulls, Joins, Strings, Edge Cases

-- Basic Filters & Counts (1-5)
SELECT COUNT(*) FROM sales WHERE amount > 500;
SELECT COUNT(*) FROM sales WHERE amount < 200;
SELECT COUNT(*) FROM sales WHERE amount = 450;
SELECT COUNT(*) FROM sales WHERE region = 'US-East';
SELECT COUNT(*) FROM sales WHERE region = 'EU-Central';

-- Range Predicates & Arithmetic (6-10)
SELECT SUM(amount) FROM sales WHERE amount > 100;
SELECT SUM(amount) FROM sales WHERE amount < 800;
SELECT SUM(amount) FROM sales WHERE amount > 250 AND amount < 750;
SELECT COUNT(*) FROM sales WHERE amount >= 50 AND amount <= 950;
SELECT SUM(amount) FROM sales WHERE region = 'AP-South';

-- Single-key Group By (11-15)
SELECT region, SUM(amount) FROM sales GROUP BY region;
SELECT region, COUNT(*) FROM sales GROUP BY region;
SELECT category, SUM(amount) FROM sales GROUP BY category;
SELECT category, COUNT(*) FROM sales GROUP BY category;
SELECT status, COUNT(*) FROM sales GROUP BY status;

-- Multi-key Group By (16-20)
SELECT region, category, SUM(amount) FROM sales GROUP BY region, category;
SELECT region, category, COUNT(*) FROM sales GROUP BY region, category;
SELECT region, status, SUM(amount) FROM sales GROUP BY region, status;
SELECT category, status, COUNT(*) FROM sales GROUP BY category, status;
SELECT region, category, status, COUNT(*) FROM sales GROUP BY region, category, status;

-- Filtered Aggregations (21-25)
SELECT region, SUM(amount) FROM sales WHERE amount > 500 GROUP BY region;
SELECT category, SUM(amount) FROM sales WHERE amount < 300 GROUP BY category;
SELECT region, COUNT(*) FROM sales WHERE category = 'Electronics' GROUP BY region;
SELECT category, COUNT(*) FROM sales WHERE region = 'US-West' GROUP BY category;
SELECT status, SUM(amount) FROM sales WHERE amount > 100 GROUP BY status;

-- HAVING Clauses (26-30)
SELECT region, SUM(amount) FROM sales GROUP BY region HAVING SUM(amount) > 10000;
SELECT category, COUNT(*) FROM sales GROUP BY category HAVING COUNT(*) > 500;
SELECT region, category, SUM(amount) FROM sales GROUP BY region, category HAVING SUM(amount) > 5000;
SELECT status, COUNT(*) FROM sales GROUP BY status HAVING COUNT(*) > 1000;
SELECT region, COUNT(*) FROM sales GROUP BY region HAVING COUNT(*) > 2000;

-- DISTINCT & Projections (31-35)
SELECT DISTINCT region FROM sales;
SELECT DISTINCT category FROM sales;
SELECT DISTINCT status FROM sales;
SELECT DISTINCT region, category FROM sales;
SELECT DISTINCT region, status FROM sales;

-- ORDER BY & LIMIT (36-40)
SELECT id, amount FROM sales ORDER BY amount ASC LIMIT 10;
SELECT id, amount FROM sales ORDER BY amount DESC LIMIT 10;
SELECT region, SUM(amount) FROM sales GROUP BY region ORDER BY SUM(amount) DESC;
SELECT category, COUNT(*) FROM sales GROUP BY category ORDER BY COUNT(*) ASC;
SELECT id, region, amount FROM sales WHERE amount > 900 ORDER BY id ASC LIMIT 5;

-- NULL Semantics & IS NULL / IS NOT NULL (41-45)
SELECT COUNT(*) FROM sales WHERE notes IS NULL;
SELECT COUNT(*) FROM sales WHERE notes IS NOT NULL;
SELECT SUM(amount) FROM sales WHERE notes IS NOT NULL;
SELECT region, COUNT(*) FROM sales WHERE notes IS NULL GROUP BY region;
SELECT category, SUM(amount) FROM sales WHERE notes IS NOT NULL GROUP BY category;

-- JOIN Queries (46-50)
SELECT COUNT(*) FROM sales JOIN customers ON sales.customer_id = customers.id;
SELECT customers.tier, COUNT(*) FROM sales JOIN customers ON sales.customer_id = customers.id GROUP BY customers.tier;
SELECT customers.tier, SUM(sales.amount) FROM sales JOIN customers ON sales.customer_id = customers.id GROUP BY customers.tier;
SELECT sales.region, customers.tier, COUNT(*) FROM sales JOIN customers ON sales.customer_id = customers.id GROUP BY sales.region, customers.tier;
SELECT COUNT(*) FROM sales JOIN customers ON sales.customer_id = customers.id WHERE sales.amount > 500;

-- Edge Cases: Empty Tables & Single-row Tables (51-54)
SELECT COUNT(*) FROM empty_table;
SELECT SUM(val) FROM empty_table;
SELECT COUNT(*) FROM single_row_table;
SELECT SUM(val) FROM single_row_table;
