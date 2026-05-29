"use client"

import { useEffect, useState } from "react"

export default function Config() {
  const ADMIN_HOST = "127.0.0.1:9090"
  const [connected, setConnected] = useState(false)
  const [config, setConfig] = useState<string | null>(null)
  const [status, setStatus] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false
    async function load() {
      try {
        const [configRes, statusRes] = await Promise.all([
          fetch(`http://${ADMIN_HOST}/status`),
          fetch(`http://${ADMIN_HOST}/health`),
        ])
        if (cancelled) return
        const configJson = await configRes.json()
        const health = await statusRes.text()
        setConfig(JSON.stringify(configJson, null, 2))
        setStatus(health)
        setConnected(true)
      } catch {
        if (!cancelled) setConnected(false)
      }
    }
    load()
    const id = setInterval(load, 5000)
    return () => { cancelled = true; clearInterval(id) }
  }, [])

  return (
    <>
      <header className="border-b border-border bg-card sticky top-0 z-40">
        <div className="px-6 h-14 flex items-center gap-4">
          <h1 className="text-sm font-semibold text-foreground">Config</h1>
          <div className="ml-auto flex items-center gap-1.5">
            <span className={`inline-block w-2 h-2 rounded-full ${connected ? "bg-green-500 animate-pulse" : "bg-red-500"}`} />
            <span className="text-xs text-muted-foreground">{connected ? "Live" : "Disconnected"}</span>
          </div>
        </div>
      </header>

      <div className="px-6 py-6">
        {status !== null && (
          <div className="mb-4 rounded-lg border border-border/30 bg-card/30 px-4 py-3 text-xs">
            <span className="text-muted-foreground">Health: </span>
            <span className={`font-mono font-semibold ${status.includes("ok") ? "text-green-400" : "text-red-400"}`}>{status}</span>
          </div>
        )}
        <div className="rounded-lg border border-border/40 bg-card/40">
          <div className="px-4 py-3 border-b border-border/40">
            <h2 className="text-sm font-semibold">Runtime State</h2>
          </div>
          {config ? (
            <pre className="p-4 text-xs font-mono text-muted-foreground overflow-x-auto max-h-[60vh] overflow-y-auto">{config}</pre>
          ) : (
            <div className="flex items-center justify-center py-16 text-muted-foreground text-sm">
              <div className="w-6 h-6 rounded-full border-2 border-border border-t-primary animate-spin mr-3" />
              Loading config...
            </div>
          )}
        </div>
      </div>
    </>
  )
}
