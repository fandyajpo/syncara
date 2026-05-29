"use client"

import { useCallback, useEffect, useRef, useState } from "react"

interface MetricsSnapshot {
  requests_total: number
  responses_total: Record<string, number>
  latency_p50_ms: number
  latency_p99_ms: number
  active_connections: number
  upstreams_healthy: number
  upstreams_total: number
}

export default function Metrics() {
  const ADMIN_HOST = "127.0.0.1:9090"
  const [connected, setConnected] = useState(false)
  const [metrics, setMetrics] = useState<MetricsSnapshot | null>(null)
  const [, setTick] = useState(0)

  const fetchMetrics = useCallback(async () => {
    try {
      const res = await fetch(`http://${ADMIN_HOST}/status`)
      if (!res.ok) return
      const data = await res.json()
      setMetrics({
        requests_total: data.requests_total ?? 0,
        responses_total: data.responses_total ?? {},
        latency_p50_ms: data.latency_p50_ms ?? 0,
        latency_p99_ms: data.latency_p99_ms ?? 0,
        active_connections: data.active_connections ?? 0,
        upstreams_healthy: data.upstreams_healthy ?? 0,
        upstreams_total: data.upstreams_total ?? 0,
      })
      setConnected(true)
    } catch {
      setConnected(false)
    }
  }, [])

  useEffect(() => {
    fetchMetrics()
    const id = setInterval(() => {
      fetchMetrics()
      setTick((t) => t + 1)
    }, 2000)
    return () => clearInterval(id)
  }, [fetchMetrics])

  return (
    <>
      <header className="border-b border-border bg-card sticky top-0 z-40">
        <div className="px-6 h-14 flex items-center gap-4">
          <h1 className="text-sm font-semibold text-foreground">Metrics</h1>
          <div className="ml-auto flex items-center gap-1.5">
            <span className={`inline-block w-2 h-2 rounded-full ${connected ? "bg-green-500 animate-pulse" : "bg-red-500"}`} />
            <span className="text-xs text-muted-foreground">{connected ? "Live" : "Disconnected"}</span>
          </div>
        </div>
      </header>

      <div className="px-6 py-6">
        {!metrics ? (
          <div className="flex flex-col items-center justify-center py-24 text-muted-foreground">
            <div className="w-10 h-10 rounded-full border-2 border-border border-t-primary animate-spin mb-4" />
            <p className="text-sm">Loading metrics...</p>
          </div>
        ) : (
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            <div className="rounded-lg border border-border/40 bg-card/40 p-4">
              <div className="text-xs text-muted-foreground mb-1">Requests Total</div>
              <div className="text-2xl font-mono font-bold">{metrics.requests_total.toLocaleString()}</div>
            </div>
            <div className="rounded-lg border border-border/40 bg-card/40 p-4">
              <div className="text-xs text-muted-foreground mb-1">Latency p50</div>
              <div className="text-2xl font-mono font-bold">{metrics.latency_p50_ms}<span className="text-sm text-muted-foreground font-normal">ms</span></div>
            </div>
            <div className="rounded-lg border border-border/40 bg-card/40 p-4">
              <div className="text-xs text-muted-foreground mb-1">Latency p99</div>
              <div className="text-2xl font-mono font-bold">{metrics.latency_p99_ms}<span className="text-sm text-muted-foreground font-normal">ms</span></div>
            </div>
            <div className="rounded-lg border border-border/40 bg-card/40 p-4">
              <div className="text-xs text-muted-foreground mb-1">Active Connections</div>
              <div className="text-2xl font-mono font-bold">{metrics.active_connections}</div>
            </div>
            <div className="rounded-lg border border-border/40 bg-card/40 p-4">
              <div className="text-xs text-muted-foreground mb-1">Upstream Health</div>
              <div className="text-2xl font-mono font-bold">
                {metrics.upstreams_healthy}<span className="text-sm text-muted-foreground font-normal">/{metrics.upstreams_total}</span>
              </div>
            </div>
            <div className="rounded-lg border border-border/40 bg-card/40 p-4">
              <div className="text-xs text-muted-foreground mb-1">Response Statuses</div>
              <div className="mt-1 space-y-1">
                {Object.entries(metrics.responses_total).map(([k, v]) => (
                  <div key={k} className="flex justify-between text-xs font-mono">
                    <span className="text-muted-foreground">{k}</span>
                    <span>{v}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}
      </div>
    </>
  )
}
