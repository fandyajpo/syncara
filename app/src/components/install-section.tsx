"use client"

import { useState } from "react"
import { motion } from "framer-motion"
import { Copy, Check, Terminal } from "lucide-react"

export function InstallSection() {
  const [copied, setCopied] = useState(false)

  const copyCmd = () => {
    navigator.clipboard.writeText('curl -fsSL https://syncara.sh/install.sh | sh')
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <section id="install" className="py-32 px-6">
      <div className="max-w-3xl mx-auto text-center">
        <motion.div
          initial={{ opacity: 0, y: 30 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6 }}
        >
          <div className="inline-flex items-center gap-2 px-4 py-1.5 rounded-full border border-border bg-secondary/50 text-sm text-muted-foreground mb-6">
            <Terminal className="w-4 h-4" />
            One command, zero dependencies
          </div>

          <h2 className="text-3xl md:text-4xl font-bold mb-4">Install in 30 seconds</h2>
          <p className="text-muted-foreground text-lg mb-10 max-w-xl mx-auto">
            No Rust toolchain. No npm. No Docker. Just a static binary.
          </p>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 30 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6, delay: 0.2 }}
          className="relative group"
        >
          <div className="rounded-xl border border-border bg-card overflow-hidden">
            <div className="flex items-center justify-between px-4 py-3 border-b border-border bg-secondary/50">
              <span className="text-xs font-mono text-muted-foreground">install.sh</span>
              <button onClick={copyCmd} className="text-muted-foreground hover:text-foreground transition-colors">
                {copied ? <Check className="w-4 h-4 text-emerald-400" /> : <Copy className="w-4 h-4" />}
              </button>
            </div>
            <div className="p-5 font-mono text-sm flex items-center gap-3">
              <span className="text-emerald-400">$</span>
              <span className="text-foreground">curl -fsSL https://syncara.sh/install.sh | sh</span>
            </div>
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0 }}
          whileInView={{ opacity: 1 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6, delay: 0.4 }}
          className="mt-8 grid grid-cols-1 md:grid-cols-3 gap-4 text-left"
        >
          <div className="rounded-lg border border-border bg-card p-4">
            <div className="font-semibold text-sm mb-1">macOS</div>
            <code className="text-xs text-muted-foreground">brew install syncara</code>
          </div>
          <div className="rounded-lg border border-border bg-card p-4">
            <div className="font-semibold text-sm mb-1">Linux</div>
            <code className="text-xs text-muted-foreground">curl pipe install (above)</code>
          </div>
          <div className="rounded-lg border border-border bg-card p-4">
            <div className="font-semibold text-sm mb-1">Windows</div>
            <code className="text-xs text-muted-foreground">irm .../install.ps1 | iex</code>
          </div>
        </motion.div>

        <motion.p
          initial={{ opacity: 0 }}
          whileInView={{ opacity: 1 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6, delay: 0.5 }}
          className="mt-10 text-sm text-muted-foreground"
        >
          Or download the binary directly from{" "}
          <a href="https://github.com/anomalyco/syncara/releases" className="text-primary hover:underline">
            GitHub Releases
          </a>
          .
        </motion.p>
      </div>
    </section>
  )
}
