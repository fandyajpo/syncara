"use client"

import { useCallback, useEffect, useRef, useState } from "react"

interface UpstreamInfo {
  addr: string
  score: number
  healthy: boolean
  active_connections: number
  latency_p50_ms: number
  pool: string
}

export default function Upstreams() {
  const [upstreams, setUpstreams] = useState<UpstreamInfo[]>([])
  const [connected, setConnected] = useState(false)
  const ADMIN_HOST = "127.0.0.1:9090"

  const handleEvent = useCallback((data: string) => {
    try {
      const event = JSON.parse(data)
      if (event.type !== "routing_decision") return
      setUpstreams((prev) => {
        const m = new Map(prev.map((u) => [u.addr, u]))
        for (const s of event.scores) {
          const existing = m.get(s.addr)
          m.set(s.addr, {
            addr: s.addr,
            score: s.score,
            healthy: s.score >= 50,
            active_connections: existing?.active_connections ?? 0,
            latency_p50_ms: existing?.latency_p50_ms ?? 0,
            pool: event.pool_name,
          })
        }
        return [...m.values()]
      })
    } catch { /* ignore parse errors */ }
  }, [])

  useEffect(() => {
    const url = `http://${ADMIN_HOST}/events`
    const source = new EventSource(url)
    source.onopen = () => setConnected(true)
    source.addEventListener("realtime", (e: MessageEvent) => handleEvent(e.data))
    source.onerror = () => setConnected(false)
    return () => source.close()
  }, [handleEvent])

  return (
    <>
      <header className="border-b border-border bg-card sticky top-0 z-40">
        <div className="px-6 h-14 flex items-center gap-4">
          <h1 className="text-sm font-semibold text-foreground">Upstreams</h1>
          <div className="ml-auto flex items-center gap-1.5">
            <span className={`inline-block w-2 h-2 rounded-full ${connected ? "bg-green-500 animate-pulse" : "bg-red-500"}`} />
            <span className="text-xs text-muted-foreground">{connected ? "Live" : "Disconnected"}</span>
          </div>
        </div>
      </header>

      <div className="px-6 py-6">
        {upstreams.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-24 text-muted-foreground">
            <div className="w-10 h-10 rounded-full border-2 border-border border-t-primary animate-spin mb-4" />
            <p className="text-sm">Waiting for upstream data...</p>
          </div>
        ) : (
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            {upstreams.map((u) => (
              <div key={u.addr} className="rounded-lg border border-border/40 bg-card/40 p-4 hover:border-border/60 transition-colors">
                <div className="flex items-center justify-between mb-3">
                  <span className="font-mono text-sm font-semibold truncate">{u.addr}</span>
                  <span className={`inline-block w-2.5 h-2.5 rounded-full ${u.healthy ? "bg-green-500" : "bg-red-500"}`} />
                </div>
                <div className="space-y-2 text-xs text-muted-foreground">
                  <div className="flex justify-between">
                    <span>Score</span>
                    <span className="font-mono text-foreground">{u.score}</span>
                  </div>
                  <div className="flex justify-between">
                    <span>Pool</span>
                    <span className="font-mono text-foreground">{u.pool}</span>
                  </div>
                  <div className="flex justify-between">
                    <span>Active connections</span>
                    <span className="font-mono text-foreground">{u.active_connections}</span>
                  </div>
                  <div className="flex justify-between">
                    <span>Latency p50</span>
                    <span className="font-mono text-foreground">{u.latency_p50_ms}ms</span>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </>
  )
}
