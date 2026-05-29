"use client"

import { Button } from "@/components/ui/button"
import { Github, ArrowRight, Terminal, Shield, Zap, Route } from "lucide-react"
import { motion } from "framer-motion"

const stats = [
  { icon: Zap, value: "147 ns", label: "Routing latency" },
  { icon: Route, value: "5", label: "Strategies" },
  { icon: Shield, value: "99.9%", label: "Predictable routing" },
]

export function Hero() {
  return (
    <section className="relative min-h-screen flex items-center justify-center overflow-hidden pt-16">
      {/* Animated grid background */}
      <div className="absolute inset-0 bg-[linear-gradient(rgba(99,102,241,0.03)_1px,transparent_1px),linear-gradient(90deg,rgba(99,102,241,0.03)_1px,transparent_1px)] bg-[size:64px_64px] animate-grid-scroll" />

      {/* Gradient orbs */}
      <div className="absolute top-1/4 -left-32 w-96 h-96 bg-primary/20 rounded-full blur-[128px] animate-aurora" />
      <div className="absolute bottom-1/4 -right-32 w-96 h-96 bg-purple-500/10 rounded-full blur-[128px] animate-aurora" style={{ animationDelay: "-3s" }} />
      <div className="absolute top-1/3 right-1/4 w-64 h-64 bg-pink-500/5 rounded-full blur-[96px] animate-float" />

      {/* Floating dots */}
      <div className="absolute inset-0 overflow-hidden pointer-events-none">
        {[...Array(20)].map((_, i) => (
          <motion.div
            key={i}
            className="absolute w-1 h-1 rounded-full bg-primary/20"
            style={{
              left: `${5 + (i * 7) % 90}%`,
              top: `${10 + (i * 13) % 80}%`,
            }}
            animate={{
              y: [0, -30, 0],
              opacity: [0.2, 0.6, 0.2],
            }}
            transition={{
              duration: 3 + (i % 3),
              repeat: Infinity,
              delay: i * 0.3,
            }}
          />
        ))}
      </div>

      <div className="relative z-10 max-w-5xl mx-auto px-6 text-center">
        {/* Badge */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.6, ease: "easeOut" }}
        >
          <div className="inline-flex items-center gap-2 px-4 py-1.5 rounded-full border border-border bg-secondary/50 text-sm text-muted-foreground mb-8 hover:border-primary/30 transition-colors">
            <span className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse" />
            v0.1.1 — Open Source
            <span className="w-px h-3 bg-border mx-1" />
            <span className="text-primary">MIT License</span>
          </div>
        </motion.div>

        {/* Main heading */}
        <motion.h1
          initial={{ opacity: 0, y: 30 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.8, delay: 0.1, ease: "easeOut" }}
          className="text-5xl md:text-7xl lg:text-8xl font-bold tracking-tight mb-6 leading-[1.1]"
        >
          Route traffic{" "}
          <span className="gradient-text">with certainty</span>
        </motion.h1>

        {/* Subtitle */}
        <motion.p
          initial={{ opacity: 0, y: 30 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.8, delay: 0.2, ease: "easeOut" }}
          className="text-xl md:text-2xl text-muted-foreground max-w-3xl mx-auto mb-4"
        >
          A deterministic reverse proxy for WebSocket and HTTP traffic.
          <br />
          <span className="text-foreground font-semibold">
            Every decision explained. No black box.
          </span>
        </motion.p>

        <motion.p
          initial={{ opacity: 0, y: 30 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.8, delay: 0.3, ease: "easeOut" }}
          className="text-muted-foreground max-w-xl mx-auto mb-10 text-sm"
        >
          YAML-driven. Single binary. No dependencies. Built for the operator at 3 AM.
        </motion.p>

        {/* CTA Buttons */}
        <motion.div
          initial={{ opacity: 0, y: 30 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.8, delay: 0.35, ease: "easeOut" }}
          className="flex items-center justify-center gap-4 mb-16 flex-wrap"
        >
          <Button size="lg" className="animate-pulse-glow" asChild>
            <a href="#install">
              Get Started
              <ArrowRight className="ml-2 w-4 h-4" />
            </a>
          </Button>
          <Button size="lg" variant="outline" asChild>
            <a href="https://github.com/fandyajpo/syncara" target="_blank" rel="noopener noreferrer">
              <Github className="mr-2 w-4 h-4" />
              GitHub
            </a>
          </Button>
          <Button size="lg" variant="ghost" asChild>
            <a href="#demo">
              Watch Demo
            </a>
          </Button>
        </motion.div>

        {/* Metrics row */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.8, delay: 0.45, ease: "easeOut" }}
          className="flex items-center justify-center gap-8 mb-16 flex-wrap"
        >
          {stats.map((s) => (
            <div key={s.label} className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-lg bg-primary/10 flex items-center justify-center">
                <s.icon className="w-5 h-5 text-primary" />
              </div>
              <div className="text-left">
                <div className="font-bold text-lg">{s.value}</div>
                <div className="text-xs text-muted-foreground">{s.label}</div>
              </div>
            </div>
          ))}
        </motion.div>

        {/* Terminal preview */}
        <motion.div
          initial={{ opacity: 0, y: 40 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.8, delay: 0.55, ease: "easeOut" }}
          className="max-w-2xl mx-auto"
        >
          <div className="rounded-xl border border-border bg-card overflow-hidden text-left glow-border">
            <div className="flex items-center gap-2 px-4 py-3 border-b border-border bg-secondary/50">
              <div className="w-3 h-3 rounded-full bg-red-500/50" />
              <div className="w-3 h-3 rounded-full bg-yellow-500/50" />
              <div className="w-3 h-3 rounded-full bg-emerald-500/50" />
              <span className="text-xs text-muted-foreground ml-2 font-mono">terminal</span>
            </div>
            <div className="p-5 font-mono text-sm leading-relaxed">
              <motion.div
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                transition={{ duration: 0.5, delay: 1.0 }}
                className="flex items-center gap-2 text-muted-foreground"
              >
                <span className="text-emerald-400">$</span>
                <span>curl -fsSL https://syncara.sh/install.sh | sh</span>
              </motion.div>

              <motion.div
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                transition={{ duration: 0.5, delay: 1.5 }}
                className="flex items-center gap-2 text-muted-foreground mt-2"
              >
                <span className="text-emerald-400">✓</span>
                <span>Installed to /usr/local/bin/syncara</span>
              </motion.div>

              <motion.div
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                transition={{ duration: 0.5, delay: 2.0 }}
                className="flex items-center gap-2 text-muted-foreground mt-2"
              >
                <span className="text-emerald-400">$</span>
                <span>syncara start</span>
              </motion.div>

              <motion.div
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                transition={{ duration: 0.5, delay: 2.5 }}
                className="flex items-start gap-2 mt-2"
              >
                <span className="text-emerald-400">╭─</span>
                <div>
                  <div className="text-foreground font-semibold">Syncara v0.1.1</div>
                  <div className="text-muted-foreground">Listening on 0.0.0.0:8080</div>
                  <div className="text-muted-foreground">1 route(s), 1 pool(s), 1 upstream(s)</div>
                  <div className="text-primary text-xs mt-1">
                    Brain strategy engaged — routing with full observability
                  </div>
                </div>
              </motion.div>
            </div>
          </div>
        </motion.div>
      </div>
    </section>
  )
}
