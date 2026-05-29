"use client"

import { motion } from "framer-motion"
import { ArrowDown, Globe, Route, Scale, Server } from "lucide-react"

const steps = [
  { icon: Globe, label: "Client", desc: "HTTP / WebSocket request arrives" },
  { icon: Route, label: "Router", desc: "Matches host + path to a pool" },
  { icon: Scale, label: "Balancer", desc: "Selects upstream via strategy" },
  { icon: Server, label: "Upstream", desc: "Request forwarded, response returned" },
]

export function HowItWorks() {
  return (
    <section id="how-it-works" className="py-32 px-6 bg-secondary/20">
      <div className="max-w-5xl mx-auto">
        <motion.div
          initial={{ opacity: 0, y: 30 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6 }}
          className="text-center mb-20"
        >
          <h2 className="text-3xl md:text-4xl font-bold mb-4">How it works</h2>
          <p className="text-muted-foreground text-lg max-w-2xl mx-auto">
            A request flows through four stages. Every decision is logged, measurable, and determined by your config.
          </p>
        </motion.div>

        <div className="relative flex flex-col md:flex-row items-center justify-center gap-0 md:gap-4">
          {steps.map((step, i) => (
            <div key={step.label} className="flex flex-col md:flex-row items-center">
              <motion.div
                initial={{ opacity: 0, y: 30 }}
                whileInView={{ opacity: 1, y: 0 }}
                viewport={{ once: true }}
                transition={{ duration: 0.6, delay: i * 0.15 }}
                className="flex flex-col items-center text-center"
              >
                <div className="w-16 h-16 rounded-xl bg-primary/10 border border-primary/20 flex items-center justify-center mb-4 relative">
                  <step.icon className="w-7 h-7 text-primary" />
                  <div className="absolute -top-2 -right-2 w-6 h-6 rounded-full bg-primary text-primary-foreground text-xs font-bold flex items-center justify-center">
                    {i + 1}
                  </div>
                </div>
                <h3 className="font-semibold mb-1">{step.label}</h3>
                <p className="text-sm text-muted-foreground max-w-[160px]">{step.desc}</p>
              </motion.div>

              {i < steps.length - 1 && (
                <motion.div
                  initial={{ opacity: 0 }}
                  whileInView={{ opacity: 1 }}
                  viewport={{ once: true }}
                  transition={{ duration: 0.6, delay: i * 0.15 + 0.3 }}
                  className="flex items-center justify-center py-4 md:py-0 md:px-2"
                >
                  <ArrowDown className="w-5 h-5 text-muted-foreground md:hidden" />
                  <svg className="hidden md:block w-12 h-4 text-muted-foreground/40" viewBox="0 0 48 16" fill="none">
                    <path d="M47.7071 8.70711C48.0976 8.31658 48.0976 7.68342 47.7071 7.29289L41.3431 0.928932C40.9526 0.538408 40.3195 0.538408 39.9289 0.928932C39.5384 1.31946 39.5384 1.95262 39.9289 2.34315L45.5858 8L39.9289 13.6569C39.5384 14.0474 39.5384 14.6805 39.9289 15.0711C40.3195 15.4616 40.9526 15.4616 41.3431 15.0711L47.7071 8.70711ZM0 9H47V7H0V9Z" fill="currentColor" />
                  </svg>
                </motion.div>
              )}
            </div>
          ))}
        </div>
      </div>
    </section>
  )
}
