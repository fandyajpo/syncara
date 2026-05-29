"use client"

import { motion } from "framer-motion"
import { Zap, Route, Server, Github } from "lucide-react"

const stats = [
  { icon: Zap, value: "147 ns", label: "Hot path routing" },
  { icon: Route, value: "5", label: "Balancing strategies" },
  { icon: Server, value: "100%", label: "In-memory, no deps" },
  { icon: Github, value: "Open Source", label: "MIT License" },
]

export function StatsBar() {
  return (
    <section className="py-16 px-6 border-y border-border bg-secondary/10">
      <div className="max-w-5xl mx-auto">
        <div className="grid grid-cols-2 md:grid-cols-4 gap-8">
          {stats.map((s, i) => (
            <motion.div
              key={s.label}
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ duration: 0.5, delay: i * 0.1 }}
              className="text-center"
            >
              <div className="w-12 h-12 rounded-xl bg-primary/10 flex items-center justify-center mx-auto mb-3">
                <s.icon className="w-6 h-6 text-primary" />
              </div>
              <div className="font-bold text-2xl gradient-text">{s.value}</div>
              <div className="text-sm text-muted-foreground">{s.label}</div>
            </motion.div>
          ))}
        </div>
      </div>
    </section>
  )
}
