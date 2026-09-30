export default function HomePage() {
  return (
    <div className="max-w-5xl mx-auto py-16 text-center">
      <h1 className="text-5xl font-extrabold text-white mb-6">Distributed Tracing with Tail Sampling</h1>
      <p className="text-slate-400 text-lg mb-8">OTLP-compliant distributed tracing backend with memory-bounded buffers, tail-based sampling policies, and dependency graphs.</p>
      <a href="/dashboard" className="px-6 py-3 rounded-lg bg-cyan-400 text-black font-semibold">Open Telemetry Dashboard</a>
    </div>
  )
}
