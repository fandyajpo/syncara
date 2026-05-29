"use client"

import { useState } from "react"
import { motion, AnimatePresence } from "framer-motion"
import { Globe, Route, Scale, Server, ArrowRight, ChevronDown } from "lucide-react"
import { Badge } from "@/components/ui/badge"

const steps = [
  {
    icon: Globe,
    label: "Client",
    desc: "HTTP / WebSocket request arrives",
    detail: "Syncara validates headers, checks rate limits, and enforces connection caps before any routing decision.",
    color: "from-blue-500/20 to-cyan-500/20",
  },
  {
    icon: Route,
    label: "Router",
    desc: "Matches host + path to a pool",
    detail: "Rules are evaluated in priority order. First match wins. Each match is logged with the exact rule that triggered.",
    color: "from-purple-500/20 to-pink-500/20",
  },
  {
    icon: Scale,
    label: "Balancer",
    desc: "Selects upstream via strategy",
    detail: "The selected strategy scores all healthy upstreams. The highest score wins. Brain strategy explains every point.",
    color: "from-emerald-500/20 to-teal-500/20",
  },
  {
    icon: Server,
    label: "Upstream",
    desc: "Request forwarded, response returned",
    detail: "Connection is proxied. Latency is recorded. Health is tracked. The operator sees every metric in real time.",
    color: "from-orange-500/20 to-red-500/20",
  },
]

export function HowItWorks() {
  const [activeStep, setActiveStep] = useState<number | null>(null)

  return (
    <section id="how-it-works" className="py-32 px-6 relative overflow-hidden">
      <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_bottom,hsl(var(--primary)/0.02),transparent_50%)]" />
      <div className="max-w-5xl mx-auto relative z-10">
        <motion.div
          initial={{ opacity: 0, y: 30 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6 }}
          className="text-center mb-16"
        >
          <h2 className="text-3xl md:text-5xl font-bold mb-4">How it works</h2>
          <p className="text-muted-foreground text-lg max-w-2xl mx-auto">
            A request flows through four stages. Every decision is deterministic, logged, and determined by your config.
          </p>
        </motion.div>

        {/* Pipeline */}
        <div className="relative">
          {/* Connecting line (desktop) */}
          <div className="hidden md:block absolute top-16 left-[12%] right-[12%] h-0.5 bg-gradient-to-r from-primary/20 via-primary/40 to-primary/20" />

          <div className="grid md:grid-cols-4 gap-6 md:gap-4">
            {steps.map((step, i) => (
              <motion.div
                key={step.label}
                initial={{ opacity: 0, y: 30 }}
                whileInView={{ opacity: 1, y: 0 }}
                viewport={{ once: true }}
                transition={{ duration: 0.6, delay: i * 0.15 }}
                className="relative"
              >
                <div
                  className="flex flex-col items-center text-center cursor-pointer group"
                  onClick={() => setActiveStep(activeStep === i ? null : i)}
                >
                  <div className={`w-20 h-20 rounded-2xl bg-gradient-to-br ${step.color} border border-primary/20 flex items-center justify-center mb-4 relative transition-all duration-300 group-hover:scale-110 group-hover:shadow-lg group-hover:shadow-primary/10`}>
                    <step.icon className="w-9 h-9 text-primary" />
                    <div className="absolute -top-2 -right-2 w-7 h-7 rounded-full bg-primary text-primary-foreground text-xs font-bold flex items-center justify-center shadow-lg">
                      {i + 1}
                    </div>
                  </div>
                  <h3 className="font-semibold text-lg mb-1 group-hover:text-primary transition-colors">{step.label}</h3>
                  <p className="text-sm text-muted-foreground">{step.desc}</p>
                  <ChevronDown className={`w-4 h-4 text-muted-foreground mt-2 transition-transform duration-300 ${activeStep === i ? 'rotate-180' : ''}`} />
                </div>

                <AnimatePresence>
                  {activeStep === i && (
                    <motion.div
                      initial={{ opacity: 0, height: 0 }}
                      animate={{ opacity: 1, height: "auto" }}
                      exit={{ opacity: 0, height: 0 }}
                      transition={{ duration: 0.3 }}
                      className="overflow-hidden mt-4"
                    >
                      <div className="rounded-xl border border-border bg-card p-4 text-sm text-muted-foreground leading-relaxed">
                        {step.detail}
                      </div>
                    </motion.div>
                  )}
                </AnimatePresence>
              </motion.div>
            ))}
          </div>
        </div>

        {/* Flow summary */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6, delay: 0.5 }}
          className="mt-16 text-center"
        >
          <div className="inline-flex items-center gap-3 px-6 py-3 rounded-xl border border-border bg-card">
            <span className="text-sm text-muted-foreground">client</span>
            <ArrowRight className="w-4 h-4 text-primary" />
            <Badge variant="secondary">validate</Badge>
            <ArrowRight className="w-4 h-4 text-primary" />
            <Badge variant="secondary">route</Badge>
            <ArrowRight className="w-4 h-4 text-primary" />
            <Badge variant="secondary">select</Badge>
            <ArrowRight className="w-4 h-4 text-primary" />
            <span className="text-sm text-muted-foreground">upstream</span>
          </div>
        </motion.div>
      </div>
    </section>
  )
}
