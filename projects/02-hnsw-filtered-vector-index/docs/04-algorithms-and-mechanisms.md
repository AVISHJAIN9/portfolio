# 04 - Algorithms and Mechanisms: HNSW Index

## 1. Multi-Layer HNSW Graph Construction [Specified]
- Layer assignment follows an exponential decay distribution:
$$l = \lfloor -\ln(\text{uniform}(0, 1)) \cdot m_L \rfloor \quad \text{where } m_L = \frac{1}{\ln(M)}$$
- Neighbor selection uses the **HNSW Heuristic (Algorithm 4 in Malkov & Yashunin 2018)**: selects diverse neighbors by pruning candidates that are closer to an already-selected neighbor than to the base node, preventing graph clustering.

## 2. Int8 Scalar Quantization & Rescoring [Specified]
- Quantization maps each dimension $x_i$ of vector $\mathbf{x}$ to $q_i \in [-128, 127]$:
$$q_i = \text{round}\left( 255 \cdot \frac{x_i - x_{min}}{x_{max} - x_{min}} \right) - 128$$
- Top-$K$ candidates retrieved via Int8 SIMD dot products are rescored against uncompressed Float32 vectors in the final candidate buffer to eliminate quantization loss.

## 3. Three-Strategy Filtered Search [Specified]
| Strategy | Selectivity Range | Mechanism | Tradeoff |
| :--- | :--- | :--- | :--- |
| **Brute-Force Scan** | $< 0.1\%$ | Evaluates distance only on candidate doc IDs from Roaring Bitmap | Optimal when candidate count is tiny ($< 500$ vectors) |
| **In-Graph Filtered** | $0.1\% - 15\%$ | Traverses graph; skips non-matching nodes for candidate list but traverses through them | Maintains graph navigability without getting stuck |
| **Post-Filter** | $> 15\%$ | Standard HNSW beam search with $efSearch = \lceil K / \text{Selectivity} \rceil$ | Zero graph traversal overhead when matching density is high |
