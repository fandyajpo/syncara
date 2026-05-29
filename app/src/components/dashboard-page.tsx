"use client"

import { useCallback, useEffect, useRef, useState } from "react"

// ── Types ────────────────────────────────────────────────

interface Deduction {
  reason: string
  points: number
}

interface Score {
  addr: string
  score: number
  deductions: Deduction[]
}

interface RoutingDecision {
  type: "routing_decision"
  selected: string
  explanation: string
  pool_name: string
  scores: Score[]
  timestamp: string
}

interface HealthTransition {
  type: "health_transition"
  addr: string
  healthy: boolean
  pool_name: string
  timestamp: string
}

type RealtimeEvent = RoutingDecision | HealthTransition

interface UpstreamHistory {
  addr: string
  scores: number[]
}

// ── Colors ───────────────────────────────────────────────

function scoreColor(score: number): string {
  if (score >= 80) return "#22c55e"
  if (score >= 50) return "#eab308"
  return "#ef4444"
}

function scoreBg(score: number): string {
  if (score >= 80) return "bg-green-500/10 border-green-500/30"
  if (score >= 50) return "bg-yellow-500/10 border-yellow-500/30"
  return "bg-red-500/10 border-red-500/30"
}

const ADMIN_HOST = "127.0.0.1:9090"

// ── Sparkline ────────────────────────────────────────────

function Sparkline({ data, color }: { data: number[]; color: string }) {
  if (data.length < 2) return null
  const w = 48
  const h = 16
  const min = Math.min(...data)
  const max = Math.max(...data)
  const range = Math.max(max - min, 1)
  const pts = data.map((v, i) => `${(i / (data.length - 1)) * w},${h - ((v - min) / range) * h}`).join(" ")
  return (
    <svg width={w} height={h} className="shrink-0 opacity-60">
      <polyline fill="none" stroke={color} strokeWidth="1.5" points={pts} />
    </svg>
  )
}

// ── Mini score bar ───────────────────────────────────────

function ScoreBar({ score, maxScore, animate }: { score: number; maxScore: number; animate: boolean }) {
  const pct = maxScore > 0 ? (score / maxScore) * 100 : 0
  return (
    <div className="h-1.5 rounded-full bg-secondary/30 w-full overflow-hidden">
      <div
        className={`h-full rounded-full ${animate ? "transition-all duration-500 ease-out" : ""}`}
        style={{
          width: `${pct}%`,
          backgroundColor: scoreColor(score),
        }}
      />
    </div>
  )
}

// ── Flow diagram ─────────────────────────────────────────

function FlowDiagram({ selected }: { selected: string | null }) {
  return (
    <div className="flex items-center gap-2 text-xs font-mono">
      <span className="px-2 py-1 rounded bg-secondary/40 text-muted-foreground">IN</span>
      <svg width="20" height="2" className="text-muted-foreground/40"><line x1="0" y1="1" x2="20" y2="1" stroke="currentColor" strokeWidth="1" /></svg>
      <span className="px-2 py-1 rounded bg-primary/10 text-primary font-semibold">Brain</span>
      <svg width="20" height="2" className="text-muted-foreground/40"><line x1="0" y1="1" x2="20" y2="1" stroke="currentColor" strokeWidth="1" /></svg>
      <span className={`px-2 py-1 rounded transition-all duration-300 ${
        selected ? "bg-green-500/20 text-green-400 border border-green-500/40" : "bg-secondary/40 text-muted-foreground"
      }`}>
        {selected || "—"}
      </span>
    </div>
  )
}

// ── Heat timeline ────────────────────────────────────────

function HeatTimeline({ decisions }: { decisions: RoutingDecision[] }) {
  const maxPixels = 120
  const items = decisions.slice(-maxPixels)
  if (items.length === 0) return null

  return (
    <div className="flex gap-px items-end h-6">
      {items.map((d, i) => {
        const best = Math.max(...d.scores.map(s => s.score), 1)
        const avg = d.scores.reduce((a, s) => a + s.score, 0) / d.scores.length
        return (
          <div
            key={`${d.timestamp}-${i}`}
            className="flex-1 rounded-t-sm transition-all duration-200"
            style={{
              height: `${Math.max(4, (avg / best) * 24)}px`,
              backgroundColor: scoreColor(avg),
              opacity: d.selected ? 1 : 0.6,
            }}
            title={d.explanation}
          />
        )
      })}
    </div>
  )
}

// ── Main component ───────────────────────────────────────

const MAX_DISPLAY = 200
const FLUSH_MS = 50 // flush buffer ~20 times/sec max

export function DashboardPage() {
  const [connected, setConnected] = useState(false)
  const [decisions, setDecisions] = useState<RoutingDecision[]>([])
  const [healthEvents, setHealthEvents] = useState<HealthTransition[]>([])
  const [autoScroll, setAutoScroll] = useState(true)
  const [flowTarget, setFlowTarget] = useState<string | null>(null)
  const [histories, setHistories] = useState<Map<string, number[]>>(new Map())
  const listRef = useRef<HTMLDivElement>(null)

  // Mutable buffer refs — no re-renders per event
  const bufRef = useRef<RoutingDecision[]>([])
  const healthBufRef = useRef<HealthTransition[]>([])
  const histBufRef = useRef<Map<string, number[]>>(new Map())
  const flowBufRef = useRef<string | null>(null)
  const [lastSecondCount, setLastSecondCount] = useState(0)
  const countRef = useRef(0)
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null)

  // Decisions/sec counter
  useEffect(() => {
    const id = setInterval(() => {
      setLastSecondCount(countRef.current)
      countRef.current = 0
    }, 1000)
    return () => clearInterval(id)
  }, [])

  // Periodic flush — batches incoming events into one state update
  useEffect(() => {
    timerRef.current = setInterval(() => {
      const events = bufRef.current
      const hEvents = healthBufRef.current
      const ft = flowBufRef.current

      bufRef.current = []
      healthBufRef.current = []
      flowBufRef.current = null

      if (events.length > 0 || hEvents.length > 0 || ft !== null) {
        setDecisions((prev) => {
          let next = prev
          for (const e of events) {
            next = [...next, e]
          }
          return next.length > MAX_DISPLAY ? next.slice(next.length - MAX_DISPLAY) : next
        })
        if (ft !== null) setFlowTarget(ft)
        if (events.length > 0) {
          setHistories((prev) => {
            const next = new Map(histBufRef.current)
            // Carry over entries from prev that aren't in histBufRef
            for (const [k, v] of prev) {
              if (!next.has(k)) next.set(k, v)
            }
            return next
          })
        }
        if (hEvents.length > 0) {
          setHealthEvents((prev) => [...prev, ...hEvents].slice(-10))
        }
      }
    }, FLUSH_MS)
    return () => { if (timerRef.current) clearInterval(timerRef.current) }
  }, [])

  // EventSource — writes into mutable refs only (no state)
  useEffect(() => {
    const url = `http://${ADMIN_HOST}/events`
    const source = new EventSource(url)

    source.onopen = () => setConnected(true)

    source.addEventListener("realtime", (e: MessageEvent) => {
      try {
        const event: RealtimeEvent = JSON.parse(e.data)
        if (event.type === "routing_decision") {
          countRef.current++
          bufRef.current.push(event)
          flowBufRef.current = event.selected
          for (const s of event.scores) {
            const h = histBufRef.current.get(s.addr) || []
            h.push(s.score)
            if (h.length > 30) h.shift()
            histBufRef.current.set(s.addr, h)
          }
        } else if (event.type === "health_transition") {
          healthBufRef.current.push(event)
        }
      } catch (err) {
        console.error("failed to parse event", err)
      }
    })

    source.onerror = () => { setConnected(false) }
    return () => source.close()
  }, [])

  useEffect(() => {
    if (autoScroll && listRef.current) {
      listRef.current.scrollTop = listRef.current.scrollHeight
    }
  }, [decisions, autoScroll])

  // Compute live stats
  const totalRouted = decisions.length
  const upstreamAddrs = [...new Set(decisions.flatMap(d => d.scores.map(s => s.addr)))]
  const recent = decisions.slice(-10)
  const leader = recent.length > 0
    ? [...recent.reduce((acc, d) => {
        acc.set(d.selected, (acc.get(d.selected) || 0) + 1)
        return acc
      }, new Map<string, number>())].sort((a, b) => b[1] - a[1])[0]?.[0] || null
  : null

  // Unique upstreams for health dots
  const allUpstreams = decisions.length > 0
    ? decisions[decisions.length - 1].scores.map(s => s.addr)
    : []

  return (
    <>
      {/* ── Header ── */}
      <header className="border-b border-border bg-card sticky top-0 z-40">
        <div className="px-6 h-14 flex items-center gap-4">
          <h1 className="text-sm font-semibold text-foreground">Traffic Brain</h1>

          <div className="ml-auto flex items-center gap-4">
            {/* Connection */}
            <div className="flex items-center gap-1.5">
              <span className={`inline-block w-2 h-2 rounded-full ${connected ? "bg-green-500 animate-pulse" : "bg-red-500"}`} />
              <span className="text-xs text-muted-foreground hidden sm:inline">{connected ? "Live" : "Disconnected"}</span>
            </div>
            {/* Decisions/sec */}
            <div className="flex items-center gap-1 text-xs text-muted-foreground">
              <span className="font-mono tabular-nums text-foreground font-semibold">{lastSecondCount}</span>
              <span className="hidden sm:inline">/s</span>
            </div>
            {/* Total */}
            <div className="flex items-center gap-1 text-xs text-muted-foreground">
              <span className="font-mono tabular-nums text-foreground">{totalRouted}</span>
              <span className="hidden sm:inline">total</span>
            </div>
            {/* Leader */}
            {leader && (
              <div className="flex items-center gap-1 text-xs text-muted-foreground">
                <span className="text-green-400 hidden sm:inline">leader</span>
                <span className="font-mono text-foreground">{leader}</span>
              </div>
            )}
            {/* Auto-scroll */}
            <label className="flex items-center gap-1.5 text-xs text-muted-foreground cursor-pointer">
              <input type="checkbox" checked={autoScroll} onChange={(e) => setAutoScroll(e.target.checked)} className="accent-primary" />
              <span className="hidden sm:inline">Scroll</span>
            </label>
          </div>
        </div>
      </header>

      <div className="px-6 py-4">
        {/* ── Live Stats Bar ── */}
        <div className="flex flex-wrap items-center gap-3 mb-4 text-xs">
          <FlowDiagram selected={flowTarget} />

          <div className="flex-1" />

          {/* Health dots per upstream */}
          <div className="flex items-center gap-2">
            {allUpstreams.map((addr) => {
              const last = decisions[decisions.length - 1]
              const score = last?.scores.find(s => s.addr === addr)?.score ?? 100
              return (
                <div key={addr} className="flex items-center gap-1" title={`${addr} — score ${score}`}>
                  <span className="inline-block w-2 h-2 rounded-full" style={{ backgroundColor: scoreColor(score) }} />
                  <span className="font-mono text-muted-foreground hidden sm:inline">{addr}</span>
                </div>
              )
            })}
          </div>
        </div>

        {/* ── Heat Timeline ── */}
        {decisions.length > 1 && (
          <div className="mb-4 rounded-lg border border-border/30 bg-card/30 p-2">
            <HeatTimeline decisions={decisions} />
          </div>
        )}

        {/* ── Health transition banners ── */}
        <div className="fixed top-16 right-4 z-50 space-y-2 w-72">
          {healthEvents.map((h, i) => (
            <div
              key={`${h.timestamp}-${i}`}
              className={`px-3 py-2 rounded-lg border text-sm shadow-lg animate-in slide-in-from-right ${
                h.healthy
                  ? "bg-green-500/10 border-green-500/30 text-green-400"
                  : "bg-red-500/10 border-red-500/30 text-red-400"
              }`}
              style={{ animation: "fadeIn 0.3s ease-out" }}
            >
              <div className="font-medium">{h.healthy ? "✓ Recovered" : "✗ Unhealthy"}</div>
              <div className="text-xs opacity-80 font-mono">{h.addr}</div>
            </div>
          ))}
        </div>

        {/* ── Decision List ── */}
        {decisions.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-24 text-muted-foreground">
            <div className="w-12 h-12 rounded-full border-2 border-border border-t-primary animate-spin mb-4" />
            <p className="text-sm">Waiting for routing decisions...</p>
          </div>
        ) : (
          <div
            ref={listRef}
            className="space-y-2 overflow-y-auto pr-1"
            style={{ maxHeight: "calc(100vh - 14rem)" }}
            onScroll={(e) => {
              const el = e.currentTarget
              setAutoScroll(el.scrollHeight - el.scrollTop - el.clientHeight < 60)
            }}
          >
            {[...decisions].reverse().map((d, ri) => {
              const maxScore = Math.max(...d.scores.map(s => s.score), 1)
              return (
                <div
                  key={`${d.timestamp}-${ri}`}
                  className="rounded-lg border border-border/40 bg-card/40 p-3 text-sm hover:border-border/60 transition-colors"
                >
                  {/* Card header */}
                  <div className="flex items-center justify-between gap-2 mb-2">
                    <div className="flex items-center gap-2 min-w-0">
                      <span className="text-xs text-muted-foreground/60 font-mono shrink-0">
                        {new Date(d.timestamp).toLocaleTimeString()}
                      </span>
                      <span className="px-1.5 py-0.5 rounded bg-secondary/40 text-[10px] font-mono text-muted-foreground uppercase">
                        {d.pool_name}
                      </span>
                      <span className="px-1.5 py-0.5 rounded bg-primary/10 text-primary text-xs font-mono font-semibold truncate">
                        {d.selected}
                      </span>
                    </div>
                    <span className="text-[10px] text-muted-foreground/30 font-mono shrink-0">
                      #{totalRouted - decisions.indexOf(d)}
                    </span>
                  </div>

                  {/* Explanation */}
                  <p className="text-xs text-muted-foreground/70 mb-2 leading-relaxed line-clamp-1">{d.explanation}</p>

                  {/* Score rows */}
                  <div className="space-y-1.5">
                    {d.scores.map((s) => {
                      const isSelected = s.addr === d.selected
                      const hist = histories.get(s.addr) || []
                      return (
                        <div
                          key={s.addr}
                          className={`relative rounded px-3 py-2 transition-all duration-300 ${
                            isSelected
                              ? "bg-primary/5 border border-primary/20 shadow-[0_0_12px_rgba(59,130,246,0.15)]"
                              : scoreBg(s.score)
                          }`}
                        >
                          {/* Glow pulse animation on selection */}
                          {isSelected && (
                            <span className="absolute inset-0 rounded animate-pulse bg-primary/5 pointer-events-none" style={{ animation: "glowPulse 0.8s ease-out" }} />
                          )}
                          <div className="flex items-center gap-2 relative z-10">
                            <span className={`font-mono text-xs shrink-0 ${isSelected ? "text-primary font-semibold" : "text-muted-foreground"}`}>
                              {s.addr}
                            </span>
                            <Sparkline data={hist} color={scoreColor(s.score)} />
                            <div className="flex-1 min-w-0">
                              <ScoreBar score={s.score} maxScore={maxScore} animate={true} />
                            </div>
                            <span className={`font-mono text-xs tabular-nums shrink-0 ${isSelected ? "text-foreground font-bold" : "text-muted-foreground"}`}
                              style={{ color: isSelected ? undefined : scoreColor(s.score) }}>
                              {s.score}
                            </span>
                            {s.deductions.length > 0 && (
                              <span className="text-[10px] text-muted-foreground/40 shrink-0 hidden sm:inline" title={s.deductions.map(d => d.reason).join("; ")}>
                                -{s.deductions.reduce((a, d) => a + d.points, 0)}
                              </span>
                            )}
                          </div>
                        </div>
                      )
                    })}
                  </div>
                </div>
              )
            })}
          </div>
        )}
      </div>

      <style>{`@keyframes fadeIn { from { opacity: 0; transform: translateX(20px); } to { opacity: 1; transform: translateX(0); } } @keyframes glowPulse { 0% { opacity: 0.4; } 50% { opacity: 0.1; } 100% { opacity: 0; } }`}</style>
    </>
  )
}
