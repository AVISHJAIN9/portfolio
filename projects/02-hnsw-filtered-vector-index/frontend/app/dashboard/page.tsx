export default function DashboardPage() {
  return (
    <div className="max-w-7xl mx-auto px-4 py-10">
      <h1 className="text-3xl font-bold text-white mb-6">Vector Index Performance Metrics</h1>
      <div className="grid grid-cols-1 md:grid-cols-4 gap-6 mb-8">
        <div className="p-6 rounded-xl bg-slate-900 border border-slate-800">
          <div className="text-slate-400 text-xs">UNFILTERED RECALL@10</div>
          <div className="text-2xl font-bold text-cyan-400">TO BE MEASURED</div>
          <div className="text-xs text-slate-500 mt-2">Target &gt;= 0.985</div>
        </div>
        <div className="p-6 rounded-xl bg-slate-900 border border-slate-800">
          <div className="text-slate-400 text-xs">FILTERED RECALL (5% SELECTIVITY)</div>
          <div className="text-2xl font-bold text-emerald-400">TO BE MEASURED</div>
          <div className="text-xs text-slate-500 mt-2">Target &gt;= 0.950</div>
        </div>
        <div className="p-6 rounded-xl bg-slate-900 border border-slate-800">
          <div className="text-slate-400 text-xs">QUERY THROUGHPUT (FLOAT32)</div>
          <div className="text-2xl font-bold text-white">TO BE MEASURED</div>
          <div className="text-xs text-slate-500 mt-2">Target &gt;= 4,500 QPS/core</div>
        </div>
        <div className="p-6 rounded-xl bg-slate-900 border border-slate-800">
          <div className="text-slate-400 text-xs">INT8 QUANTIZED QPS</div>
          <div className="text-2xl font-bold text-purple-400">TO BE MEASURED</div>
          <div className="text-xs text-slate-500 mt-2">Target &gt;= 12,000 QPS/core</div>
        </div>
      </div>
    </div>
  )
}
