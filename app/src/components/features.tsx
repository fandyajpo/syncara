"use client"

import { Shield, Zap, Route, Gauge, Webhook, BrainCircuit, ArrowRight } from "lucide-react"
import { motion } from "framer-motion"

const features = [
  {
    icon: Webhook,
    title: "WebSocket Native",
    description: "Automatic WebSocket upgrade detection and tunneling. No special config needed — Syncara handles WS connections out of the box.",
    gradient: "from-blue-500/20 to-cyan-500/20",
  },
  {
    icon: Route,
    title: "Deterministic Routing",
    description: "Every routing decision is explainable. Host-based, path-based, and weighted routing with zero black-box behavior.",
    gradient: "from-purple-500/20 to-pink-500/20",
  },
  {
    icon: Gauge,
    title: "Load Balancing",
    description: "Five strategies: round-robin, least-connections, IP hash, weighted, and Brain — a deterministic scoring engine.",
    gradient: "from-emerald-500/20 to-teal-500/20",
  },
  {
    icon: BrainCircuit,
    title: "Traffic Brain",
    description: "Latency-aware, health-aware routing that scores upstreams on response time, connection load, and failure state.",
    gradient: "from-orange-500/20 to-red-500/20",
  },
  {
    icon: Shield,
    title: "Security Built In",
    description: "Per-IP rate limiting, connection caps, request validation, and timeouts. Production-safe defaults out of the box.",
    gradient: "from-indigo-500/20 to-purple-500/20",
  },
  {
    icon: Zap,
    title: "147 ns Routing",
    description: "Micro-benchmarked hot path. Round-robin selection in ~147 ns. Designed for latency-sensitive real-time applications.",
    gradient: "from-amber-500/20 to-orange-500/20",
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
    <section id="features" className="py-32 px-6 relative overflow-hidden">
      <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_top,hsl(var(--primary)/0.03),transparent_50%)]" />
      <div className="max-w-6xl mx-auto relative z-10">
        <motion.div
          initial={{ opacity: 0, y: 30 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6 }}
          className="text-center mb-6"
        >
          <div className="inline-flex items-center gap-2 px-4 py-1.5 rounded-full border border-border bg-secondary/50 text-sm text-muted-foreground mb-6">
            <Zap className="w-4 h-4" />
            Production-ready features
          </div>
          <h2 className="text-3xl md:text-5xl font-bold mb-4">Everything you need, nothing you don&apos;t</h2>
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
          className="grid md:grid-cols-2 lg:grid-cols-3 gap-6 mt-16"
        >
          {features.map((f) => (
            <motion.div key={f.title} variants={item}>
              <div className="group rounded-xl border border-border bg-card p-6 card-hover h-full relative overflow-hidden">
                <div className={`absolute inset-0 bg-gradient-to-br ${f.gradient} opacity-0 group-hover:opacity-100 transition-opacity duration-500`} />
                <div className="relative z-10">
                  <div className="w-12 h-12 rounded-xl bg-primary/10 flex items-center justify-center mb-4 group-hover:bg-primary/20 transition-all duration-300 group-hover:scale-110">
                    <f.icon className="w-6 h-6 text-primary" />
                  </div>
                  <h3 className="font-semibold text-lg mb-2 group-hover:text-primary transition-colors">{f.title}</h3>
                  <p className="text-sm text-muted-foreground leading-relaxed">{f.description}</p>
                </div>
              </div>
            </motion.div>
          ))}
        </motion.div>

        <motion.div
          initial={{ opacity: 0 }}
          whileInView={{ opacity: 1 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6, delay: 0.4 }}
          className="mt-12 text-center"
        >
          <a
            href="#how-it-works"
            className="inline-flex items-center gap-2 text-sm text-primary hover:text-primary/80 transition-colors"
          >
            See how routing works <ArrowRight className="w-3.5 h-3.5" />
          </a>
        </motion.div>
      </div>
    </section>
  )
}
