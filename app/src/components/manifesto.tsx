"use client"

import { motion } from "framer-motion"

const principles = [
  {
    quote: "Every routing decision is deterministic and explainable. There is no black box. There is no training phase.",
    author: "Determinism",
  },
  {
    quote: "We optimize for the operator at 3 AM, not the demo at a conference. Boring infrastructure is trustworthy infrastructure.",
    author: "Boring by Design",
  },
  {
    quote: "Features justify inclusion by production value. Every knob solves a specific problem. No unused complexity.",
    author: "Production First",
  },
  {
    quote: "When something goes wrong, the operator should trace the decision to a specific config value. Every time.",
    author: "Total Observability",
  },
]

export function Manifesto() {
  return (
    <section className="py-32 px-6 bg-secondary/20">
      <div className="max-w-5xl mx-auto">
        <motion.div
          initial={{ opacity: 0, y: 30 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6 }}
          className="text-center mb-16"
        >
          <h2 className="text-3xl md:text-4xl font-bold mb-4">The Syncara Philosophy</h2>
          <p className="text-muted-foreground text-lg max-w-2xl mx-auto">
            We believe infrastructure should be predictable, observable, and boring.
          </p>
        </motion.div>

        <div className="grid md:grid-cols-2 gap-6">
          {principles.map((p, i) => (
            <motion.div
              key={p.author}
              initial={{ opacity: 0, y: 30 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ duration: 0.6, delay: i * 0.1 }}
            >
              <div className="relative rounded-xl border border-border bg-card p-8 h-full">
                <div className="absolute top-0 left-8 -translate-y-1/2 w-8 h-8 rounded-full bg-primary/20 flex items-center justify-center">
                  <svg className="w-4 h-4 text-primary" viewBox="0 0 24 24" fill="currentColor">
                    <path d="M9.983 3v7.391c0 5.704-3.731 9.57-8.983 10.609l-.995-2.151c2.432-.917 3.995-3.638 3.995-5.849h-4v-10h9.983zm14.017 0v7.391c0 5.704-3.748 9.571-9 10.609l-.996-2.151c2.433-.917 3.996-3.638 3.996-5.849h-3.983v-10h9.983z" />
                  </svg>
                </div>
                <blockquote className="text-lg leading-relaxed mb-4 mt-2">
                  &ldquo;{p.quote}&rdquo;
                </blockquote>
                <cite className="text-sm text-muted-foreground not-italic">— {p.author}</cite>
              </div>
            </motion.div>
          ))}
        </div>
      </div>
    </section>
  )
}
