"use client"

import { Button } from "@/components/ui/button"
import { Github, Terminal, ArrowRight } from "lucide-react"
import { motion } from "framer-motion"

export function Hero() {
  return (
    <section className="relative min-h-screen flex items-center justify-center overflow-hidden">
      {/* Background grid */}
      <div className="absolute inset-0 bg-[linear-gradient(rgba(99,102,241,0.03)_1px,transparent_1px),linear-gradient(90deg,rgba(99,102,241,0.03)_1px,transparent_1px)] bg-[size:64px_64px]" />

      {/* Gradient orbs */}
      <div className="absolute top-1/4 -left-32 w-96 h-96 bg-primary/20 rounded-full blur-[128px]" />
      <div className="absolute bottom-1/4 -right-32 w-96 h-96 bg-purple-500/10 rounded-full blur-[128px]" />

      <div className="relative z-10 max-w-5xl mx-auto px-6 text-center">
        <motion.div
          initial={{ opacity: 0, y: 40 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.8, ease: "easeOut" }}
        >
          <div className="inline-flex items-center gap-2 px-4 py-1.5 rounded-full border border-border bg-secondary/50 text-sm text-muted-foreground mb-8">
            <span className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse" />
            v0.1.0 — Open Source
          </div>
        </motion.div>

        <motion.h1
          initial={{ opacity: 0, y: 40 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.8, delay: 0.1, ease: "easeOut" }}
          className="text-5xl md:text-7xl lg:text-8xl font-bold tracking-tight mb-6"
        >
          Syncara
        </motion.h1>

        <motion.p
          initial={{ opacity: 0, y: 40 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.8, delay: 0.2, ease: "easeOut" }}
          className="text-xl md:text-2xl text-muted-foreground max-w-2xl mx-auto mb-4"
        >
          Deterministic WebSocket-native reverse proxy.
          <br />
          <span className="text-foreground font-semibold">No magic.</span>
        </motion.p>

        <motion.p
          initial={{ opacity: 0, y: 40 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.8, delay: 0.3, ease: "easeOut" }}
          className="text-muted-foreground max-w-xl mx-auto mb-10"
        >
          Traffic routing you can reason about. Real-time. Config-driven.
          Boring infrastructure you can trust at 3 AM.
        </motion.p>

        <motion.div
          initial={{ opacity: 0, y: 40 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.8, delay: 0.4, ease: "easeOut" }}
          className="flex items-center justify-center gap-4 mb-16"
        >
          <Button size="lg" asChild>
            <a href="#install">
              Get Started
              <ArrowRight className="ml-2 w-4 h-4" />
            </a>
          </Button>
          <Button size="lg" variant="outline" asChild>
            <a href="https://github.com/anomalyco/syncara" target="_blank" rel="noopener noreferrer">
              <Github className="mr-2 w-4 h-4" />
              GitHub
            </a>
          </Button>
        </motion.div>

        {/* Terminal preview */}
        <motion.div
          initial={{ opacity: 0, y: 40 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.8, delay: 0.6, ease: "easeOut" }}
          className="max-w-2xl mx-auto"
        >
          <div className="rounded-xl border border-border bg-card overflow-hidden text-left">
            <div className="flex items-center gap-2 px-4 py-3 border-b border-border bg-secondary/50">
              <div className="w-3 h-3 rounded-full bg-red-500/50" />
              <div className="w-3 h-3 rounded-full bg-yellow-500/50" />
              <div className="w-3 h-3 rounded-full bg-emerald-500/50" />
              <span className="text-xs text-muted-foreground ml-2 font-mono">terminal</span>
            </div>
            <div className="p-5 font-mono text-sm leading-relaxed">
              <div className="flex items-center gap-2 text-muted-foreground mb-2">
                <span className="text-emerald-400">$</span>
                <span>curl -fsSL https://syncara.sh/install.sh | sh</span>
              </div>
              <div className="flex items-center gap-2 text-muted-foreground mb-2">
                <span className="text-emerald-400">✓</span>
                <span>Installed to /usr/local/bin/syncara</span>
              </div>
              <div className="flex items-center gap-2 text-muted-foreground mb-2">
                <span className="text-emerald-400">$</span>
                <span>syncara start</span>
              </div>
              <div className="flex items-start gap-2">
                <span className="text-emerald-400">╭─</span>
                <div>
                  <div className="text-foreground">Syncara v0.1.0</div>
                  <div className="text-muted-foreground">Listening on 0.0.0.0:8080</div>
                  <div className="text-muted-foreground">1 route(s), 1 pool(s), 1 upstream(s)</div>
                </div>
              </div>
            </div>
          </div>
        </motion.div>
      </div>
    </section>
  )
}
