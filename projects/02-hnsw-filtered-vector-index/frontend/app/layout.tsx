import './globals.css'
import type { Metadata } from 'next'

export const metadata: Metadata = {
  title: 'HNSW Filtered Vector Index | SIMD Acceleration & 3D Visualizer',
  description: 'Industrial-grade vector search engine with multi-layer HNSW graphs, Int8 quantization, and learned filter query planning.',
}

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className="dark">
      <body className="bg-slate-950 text-slate-100 min-h-screen antialiased flex flex-col font-sans">
        <header className="sticky top-0 z-50 backdrop-blur-md bg-slate-950/80 border-b border-slate-800">
          <div className="max-w-7xl mx-auto px-4 h-16 flex items-center justify-between">
            <span className="font-bold text-lg text-white">HNSW<span className="text-cyan-400">Index</span></span>
            <nav className="flex space-x-6 text-sm">
              <a href="/dashboard" className="text-slate-300 hover:text-cyan-400">Dashboard</a>
              <a href="/search" className="text-slate-300 hover:text-cyan-400">Search Explorer</a>
              <a href="/ai-insights" className="text-slate-300 hover:text-cyan-400">AI Planner</a>
            </nav>
          </div>
        </header>
        <main className="flex-1">{children}</main>
      </body>
    </html>
  )
}
