"use client"

import { motion } from "framer-motion"
import { ArrowRight, BookOpen } from "lucide-react"
import { Button } from "@/components/ui/button"

export function CtaSection() {
  return (
    <section className="py-32 px-6 bg-secondary/20">
      <div className="max-w-3xl mx-auto text-center">
        <motion.div
          initial={{ opacity: 0, y: 30 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6 }}
        >
          <div className="inline-flex items-center gap-2 px-4 py-1.5 rounded-full border border-border bg-secondary/50 text-sm text-muted-foreground mb-6">
            <BookOpen className="w-4 h-4" />
            Start learning in 5 minutes
          </div>

          <h2 className="text-3xl md:text-4xl font-bold mb-4">Ready to route some traffic?</h2>
          <p className="text-muted-foreground text-lg mb-10 max-w-xl mx-auto">
            From a single backend to production WebSocket farms — Syncara scales with you.
          </p>

          <div className="flex items-center justify-center gap-4">
            <Button size="lg" asChild>
              <a href="#install">
                Install Now
                <ArrowRight className="ml-2 w-4 h-4" />
              </a>
            </Button>
            <Button size="lg" variant="outline" asChild>
              <a href="#demo">
                See the Demo
              </a>
            </Button>
          </div>
        </motion.div>
      </div>
    </section>
  )
}
