"use client"

import { Shield, Zap, Route, Gauge, Webhook, BrainCircuit } from "lucide-react"
import { motion } from "framer-motion"

const features = [
  {
    icon: Webhook,
    title: "WebSocket Native",
    description: "Automatic WebSocket upgrade detection and tunneling. No special config needed — Syncara handles WS connections out of the box.",
  },
  {
    icon: Route,
    title: "Deterministic Routing",
    description: "Every routing decision is explainable. Host-based, path-based, and weighted routing with zero black-box behavior.",
  },
  {
    icon: Gauge,
    title: "Load Balancing",
    description: "Five strategies: round-robin, least-connections, IP hash, weighted, and Brain — a deterministic scoring engine.",
  },
  {
    icon: BrainCircuit,
    title: "Traffic Brain",
    description: "Latency-aware, health-aware routing that scores upstreams on response time, connection load, and failure state.",
  },
  {
    icon: Shield,
    title: "Security Built In",
    description: "Per-IP rate limiting, connection caps, request validation, and timeouts. Production-safe defaults out of the box.",
  },
  {
    icon: Zap,
    title: "147 ns Routing",
    description: "Micro-benchmarked hot path. Round-robin selection in ~147 ns. Designed for latency-sensitive real-time applications.",
  },
]

const container = {
  hidden: { opacity: 0 },
  show: {
    opacity: 1,
    transition: { staggerChildren: 0.1 },
  },
}

const item = {
  hidden: { opacity: 0, y: 30 },
  show: { opacity: 1, y: 0, transition: { duration: 0.6, ease: "easeOut" } },
}

export function Features() {
  return (
    <section id="features" className="py-32 px-6">
      <div className="max-w-6xl mx-auto">
        <motion.div
          initial={{ opacity: 0, y: 30 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6 }}
          className="text-center mb-20"
        >
          <h2 className="text-3xl md:text-4xl font-bold mb-4">Everything you need, nothing you don&apos;t</h2>
          <p className="text-muted-foreground text-lg max-w-2xl mx-auto">
            Syncara is built for production real-time traffic. Every feature solves a concrete problem.
            No bloat. No plugins. No surprises.
          </p>
        </motion.div>

        <motion.div
          variants={container}
          initial="hidden"
          whileInView="show"
          viewport={{ once: true }}
          className="grid md:grid-cols-2 lg:grid-cols-3 gap-6"
        >
          {features.map((f) => (
            <motion.div key={f.title} variants={item}>
              <div className="group rounded-xl border border-border bg-card p-6 hover:border-primary/30 transition-all duration-300">
                <div className="w-10 h-10 rounded-lg bg-primary/10 flex items-center justify-center mb-4 group-hover:bg-primary/20 transition-colors">
                  <f.icon className="w-5 h-5 text-primary" />
                </div>
                <h3 className="font-semibold mb-2">{f.title}</h3>
                <p className="text-sm text-muted-foreground leading-relaxed">{f.description}</p>
              </div>
            </motion.div>
          ))}
        </motion.div>
      </div>
    </section>
  )
}
