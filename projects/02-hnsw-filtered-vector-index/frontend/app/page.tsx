export default function HomePage() {
  return (
    <div className="max-w-7xl mx-auto px-4 py-24 text-center">
      <h1 className="text-6xl font-extrabold text-white mb-6">
        Sub-Millisecond Vector Search with <br />
        <span className="text-cyan-400">Learned Filter Planning</span>
      </h1>
      <p className="text-slate-400 text-lg max-w-2xl mx-auto mb-10">
        From-scratch HNSW with AVX2/NEON SIMD kernels, Int8 scalar quantization, lock-striped concurrency, and roaring bitmap metadata filters.
      </p>
      <div className="flex justify-center gap-4">
        <a href="/search" className="px-8 py-3.5 rounded-xl bg-cyan-400 text-black font-semibold hover:bg-cyan-300">
          Launch 3D Search Explorer
        </a>
        <a href="/dashboard" className="px-8 py-3.5 rounded-xl bg-slate-900 border border-slate-800 text-white font-semibold">
          View Index Telemetry
        </a>
      </div>
    </div>
  )
}
