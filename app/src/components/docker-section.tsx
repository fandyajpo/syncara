"use client"

import { motion } from "framer-motion"
import { Container } from "lucide-react"

export function DockerSection() {
  return (
    <section className="py-32 px-6 bg-secondary/20">
      <div className="max-w-4xl mx-auto">
        <motion.div
          initial={{ opacity: 0, y: 30 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6 }}
          className="text-center mb-16"
        >
          <div className="inline-flex items-center gap-2 px-4 py-1.5 rounded-full border border-border bg-secondary/50 text-sm text-muted-foreground mb-6">
            <Container className="w-4 h-4" />
            Docker support built in
          </div>
          <h2 className="text-3xl md:text-4xl font-bold mb-4">Run with Docker</h2>
          <p className="text-muted-foreground text-lg max-w-2xl mx-auto">
            Single static binary, no runtime deps. Alpine or scratch — your choice.
          </p>
        </motion.div>

        <div className="grid md:grid-cols-2 gap-6">
          <motion.div
            initial={{ opacity: 0, x: -30 }}
            whileInView={{ opacity: 1, x: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.6 }}
          >
            <div className="rounded-xl border border-border bg-card overflow-hidden h-full">
              <div className="px-5 py-3 border-b border-border bg-secondary/50">
                <span className="text-xs font-mono text-muted-foreground">Dockerfile</span>
              </div>
              <div className="p-5 font-mono text-xs leading-relaxed">
                <div><span className="text-purple-400">FROM</span><span className="text-amber-400"> debian:bookworm-slim</span></div>
                <div>&nbsp;</div>
                <div><span className="text-purple-400">COPY</span> syncara /usr/local/bin/syncara</div>
                <div>&nbsp;</div>
                <div><span className="text-purple-400">EXPOSE</span> 8080 9090</div>
                <div><span className="text-purple-400">ENTRYPOINT</span> [<span className="text-emerald-400">&quot;syncara&quot;</span>]</div>
                <div><span className="text-purple-400">CMD</span> [<span className="text-emerald-400">&quot;start&quot;</span>]</div>
              </div>
            </div>
          </motion.div>

          <motion.div
            initial={{ opacity: 0, x: 30 }}
            whileInView={{ opacity: 1, x: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.6 }}
          >
            <div className="rounded-xl border border-border bg-card overflow-hidden h-full">
              <div className="px-5 py-3 border-b border-border bg-secondary/50">
                <span className="text-xs font-mono text-muted-foreground">docker-compose.yml</span>
              </div>
              <div className="p-5 font-mono text-xs leading-relaxed">
                <div><span className="text-purple-400">services</span>:</div>
                <div className="ml-4"><span className="text-purple-400">syncara</span>:</div>
                <div className="ml-8"><span className="text-sky-400">build</span>: .</div>
                <div className="ml-8"><span className="text-sky-400">ports</span>:</div>
                <div className="ml-12">- <span className="text-amber-400">&quot;8080:8080&quot;</span></div>
                <div className="ml-12">- <span className="text-amber-400">&quot;9090:9090&quot;</span></div>
                <div className="ml-8"><span className="text-sky-400">volumes</span>:</div>
                <div className="ml-12">- ./syncara.yml:/etc/syncara/syncara.yml</div>
                <div className="ml-8"><span className="text-sky-400">command</span>: [<span className="text-emerald-400">&quot;start&quot;</span>, <span className="text-emerald-400">&quot;-c&quot;</span>, <span className="text-emerald-400">&quot;/etc/syncara/syncara.yml&quot;</span>]</div>
              </div>
            </div>
          </motion.div>
        </div>

        <motion.div
          initial={{ opacity: 0, y: 30 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.6, delay: 0.2 }}
          className="mt-8"
        >
          <div className="rounded-xl border border-border bg-card overflow-hidden">
            <div className="px-5 py-3 border-b border-border bg-secondary/50 flex items-center gap-2">
              <span className="w-2 h-2 rounded-full bg-emerald-500" />
              <span className="text-xs font-mono text-muted-foreground">Quick start with Docker</span>
            </div>
            <div className="p-5 font-mono text-sm space-y-2">
              <div><span className="text-emerald-400">$</span> docker build -t syncara .</div>
              <div><span className="text-emerald-400">$</span> docker run -p 8080:8080 -p 9090:9090 \</div>
              <div className="ml-4">-v $(pwd)/syncara.yml:/etc/syncara/syncara.yml \</div>
              <div className="ml-4">syncara start -c /etc/syncara/syncara.yml</div>
              <div>&nbsp;</div>
              <div className="text-muted-foreground"># Or zero-config with env vars:</div>
              <div><span className="text-emerald-400">$</span> docker run -p 8080:8080 syncara start --backend host.docker.internal:3000</div>
            </div>
          </div>
        </motion.div>
      </div>
    </section>
  )
}
