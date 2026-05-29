"use client"

import { useState } from "react"
import { motion } from "framer-motion"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Play, Copy, Check, ArrowRight } from "lucide-react"

const defaultConfig = `listeners:
  - port: 8080

routes:
  - path: /
    proxy: http://localhost:9001`

const requestResults = [
  { label: "Routing Decision", value: "host: * → path: / → pool: default" },
  { label: "Load Balancer", value: "strategy: round-robin → upstream[0]" },
  { label: "Upstream", value: "localhost:9001 (healthy, latency: 1.2ms)" },
  { label: "Response", value: "200 OK (2.1ms total)" },
]

export function InteractiveDemo() {
  const [running, setRunning] = useState(false)
  const [step, setStep] = useState(-1)
  const [copied, setCopied] = useState(false)

  const runDemo = () => {
    setRunning(true)
    setStep(-1)
    requestResults.forEach((_, i) => {
      setTimeout(() => setStep(i), 500 + i * 600)
    })
    setTimeout(() => setRunning(false), 500 + requestResults.length * 600 + 400)
  }

  const copyConfig = () => {
    navigator.clipboard.writeText(defaultConfig)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <section id="demo" className="py-32 px-6">
      <div className="max-w-5xl mx-auto">
        <motion.div
          initial={{ opacity: 0, y: 30 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6 }}
          className="text-center mb-12"
        >
          <h2 className="text-3xl md:text-4xl font-bold mb-4">See it in action</h2>
          <p className="text-muted-foreground text-lg max-w-2xl mx-auto">
            Watch a request flow through Syncara. Your config determines everything.
          </p>
        </motion.div>

        <div className="grid md:grid-cols-2 gap-6">
          {/* Config panel */}
          <motion.div
            initial={{ opacity: 0, x: -30 }}
            whileInView={{ opacity: 1, x: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.6 }}
          >
            <div className="rounded-xl border border-border bg-card overflow-hidden">
              <div className="flex items-center justify-between px-4 py-3 border-b border-border bg-secondary/50">
                <span className="text-xs font-mono text-muted-foreground">syncara.yml</span>
                <button onClick={copyConfig} className="text-muted-foreground hover:text-foreground transition-colors">
                  {copied ? <Check className="w-4 h-4 text-emerald-400" /> : <Copy className="w-4 h-4" />}
                </button>
              </div>
              <div className="p-5 font-mono text-sm leading-relaxed">
                <div className="text-sky-400">listeners:</div>
                <div className="text-sky-400 ml-4">- port: <span className="text-amber-400">8080</span></div>
                <div>&nbsp;</div>
                <div className="text-sky-400">routes:</div>
                <div className="ml-4">
                  <span className="text-sky-400">- path:</span><span className="text-amber-400"> /</span>
                </div>
                <div className="ml-4">
                  <span className="text-sky-400">&nbsp;&nbsp;proxy:</span><span className="text-emerald-400"> http://localhost:9001</span>
                </div>
              </div>
            </div>
          </motion.div>

          {/* Results panel */}
          <motion.div
            initial={{ opacity: 0, x: 30 }}
            whileInView={{ opacity: 1, x: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.6 }}
          >
            <div className="rounded-xl border border-border bg-card overflow-hidden">
              <div className="flex items-center justify-between px-4 py-3 border-b border-border bg-secondary/50">
                <span className="text-xs font-mono text-muted-foreground">output</span>
                <Badge variant={running ? "success" : "secondary"}>
                  {running ? "processing" : "ready"}
                </Badge>
              </div>
              <div className="p-5">
                <Button
                  onClick={runDemo}
                  disabled={running}
                  size="sm"
                  className="mb-4"
                >
                  <Play className="w-4 h-4 mr-2" />
                  Send Request
                </Button>

                <div className="space-y-3 font-mono text-sm">
                  {requestResults.map((r, i) => (
                    <motion.div
                      key={r.label}
                      initial={{ opacity: 0, height: 0 }}
                      animate={{
                        opacity: step >= i ? 1 : 0,
                        height: step >= i ? "auto" : 0,
                      }}
                      transition={{ duration: 0.3 }}
                      className="overflow-hidden"
                    >
                      <div className="rounded-lg border border-border/50 bg-secondary/30 p-3">
                        <div className="text-xs text-muted-foreground mb-1">{r.label}</div>
                        <div className="text-foreground flex items-center gap-2">
                          {step >= i && step === i && (
                            <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse" />
                          )}
                          {r.value}
                        </div>
                      </div>
                    </motion.div>
                  ))}
                </div>
              </div>
            </div>
          </motion.div>
        </div>

        <motion.div
          initial={{ opacity: 0 }}
          whileInView={{ opacity: 1 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6, delay: 0.3 }}
          className="mt-12 text-center"
        >
          <p className="text-muted-foreground text-sm">
            Every decision is logged with a full scoring breakdown.{" "}
            <a href="#install" className="text-primary hover:underline">
              Try it locally in 30 seconds <ArrowRight className="inline w-3 h-3" />
            </a>
          </p>
        </motion.div>
      </div>
    </section>
  )
}
