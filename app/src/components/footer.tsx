"use client"

import { Github, ExternalLink } from "lucide-react"

export function Footer() {
  return (
    <footer className="border-t border-border py-12 px-6">
      <div className="max-w-5xl mx-auto">
        <div className="grid md:grid-cols-4 gap-8">
          <div className="md:col-span-2">
            <div className="font-bold text-lg mb-2">Syncara</div>
            <p className="text-sm text-muted-foreground max-w-sm">
              Deterministic WebSocket-native reverse proxy.
              Config-driven. Production-ready. No magic.
            </p>
          </div>
          <div>
            <div className="font-semibold text-sm mb-3">Product</div>
            <div className="space-y-2 text-sm text-muted-foreground">
              <a href="#features" className="block hover:text-foreground transition-colors">Features</a>
              <a href="#how-it-works" className="block hover:text-foreground transition-colors">How it Works</a>
              <a href="#demo" className="block hover:text-foreground transition-colors">Demo</a>
              <a href="#install" className="block hover:text-foreground transition-colors">Install</a>
            </div>
          </div>
          <div>
            <div className="font-semibold text-sm mb-3">Community</div>
            <div className="space-y-2 text-sm text-muted-foreground">
              <a
                href="https://github.com/anomalyco/syncara"
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center gap-1.5 hover:text-foreground transition-colors"
              >
                <Github className="w-3.5 h-3.5" />
                GitHub
                <ExternalLink className="w-3 h-3" />
              </a>
              <a
                href="https://github.com/anomalyco/syncara/releases"
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center gap-1.5 hover:text-foreground transition-colors"
              >
                Releases
                <ExternalLink className="w-3 h-3" />
              </a>
            </div>
          </div>
        </div>
        <div className="mt-12 pt-8 border-t border-border text-sm text-muted-foreground">
          Open source under the MIT License.
        </div>
      </div>
    </footer>
  )
}
