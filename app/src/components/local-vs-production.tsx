"use client"

import { motion } from "framer-motion"
import { Laptop, Server, ArrowRight } from "lucide-react"

const comparisons = [
  {
    icon: Laptop,
    title: "Local Testing",
    desc: "Run multiple backends on your machine to try Syncara.",
    code: "# Terminal 1 — backend A\npython3 -m http.server 9001 &\n\n# Terminal 2 — backend B\npython3 -m http.server 9002 &\n\n# Terminal 3 — Syncara\nsyncara start --backend localhost:9001\n\n# See balancing in action\ncurl http://localhost:8080/\ncurl http://localhost:8080/",
  },
  {
    icon: Server,
    title: "Production Deployment",
    desc: "Backends are separate machines. Syncara runs on one server.",
    code: "# syncara.yml\npools:\n  - name: web\n    strategy: round-robin\n    upstreams:\n      - addr: \"10.0.1.42:3000\"\n      - addr: \"10.0.1.85:3000\"\n      - addr: \"10.0.1.12:3000\"\n\nroutes:\n  - path: /\n    pool: web",
  },
]

export function LocalVsProduction() {
  return (
    <section className="py-32 px-6">
      <div className="max-w-5xl mx-auto">
        <motion.div
          initial={{ opacity: 0, y: 30 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6 }}
          className="text-center mb-16"
        >
          <h2 className="text-3xl md:text-4xl font-bold mb-4">Same proxy. Different machines.</h2>
          <p className="text-muted-foreground text-lg max-w-2xl mx-auto">
            Test with multiple backends on your laptop. Deploy the same config to production.
            Syncara doesn&apos;t care where the backends live.
          </p>
        </motion.div>

        <div className="grid md:grid-cols-2 gap-8">
          {comparisons.map((item, i) => (
            <motion.div
              key={item.title}
              initial={{ opacity: 0, y: 30 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ duration: 0.6, delay: i * 0.15 }}
            >
              <div className="rounded-xl border border-border bg-card overflow-hidden h-full">
                <div className="flex items-center gap-3 px-6 pt-6 pb-4">
                  <div className="w-10 h-10 rounded-lg bg-primary/10 flex items-center justify-center">
                    <item.icon className="w-5 h-5 text-primary" />
                  </div>
                  <div>
                    <h3 className="font-semibold">{item.title}</h3>
                    <p className="text-sm text-muted-foreground">{item.desc}</p>
                  </div>
                </div>
                <div className="px-6 pb-6">
                  <div className="rounded-lg bg-black/40 border border-border/50 p-4 font-mono text-xs leading-relaxed whitespace-pre">
                    <code className="text-muted-foreground">{item.code}</code>
                  </div>
                </div>
              </div>
            </motion.div>
          ))}
        </div>

        <motion.div
          initial={{ opacity: 0 }}
          whileInView={{ opacity: 1 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6, delay: 0.3 }}
          className="mt-12 text-center"
        >
          <p className="text-muted-foreground text-sm">
            Same config file works everywhere. Change the IPs, keep the logic.{" "}
            <a href="#install" className="text-primary hover:underline">
              Try it now <ArrowRight className="inline w-3 h-3" />
            </a>
          </p>
        </motion.div>
      </div>
    </section>
  )
}
