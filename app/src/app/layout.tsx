import type { Metadata } from "next"
import { Inter, JetBrains_Mono } from "next/font/google"
import { Nav } from "@/components/nav"
import "./globals.css"

const inter = Inter({ subsets: ["latin"], variable: "--font-sans" })
const mono = JetBrains_Mono({ subsets: ["latin"], variable: "--font-mono" })

export const metadata: Metadata = {
  title: "Syncara — Deterministic reverse proxy for WebSocket and HTTP traffic",
  description:
    "Syncara is a fast, deterministic reverse proxy and load balancer for real-time WebSocket and HTTP applications. YAML-driven. Single binary. No magic.",
}

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className="dark">
      <body className={`${inter.variable} ${mono.variable} font-sans`}>
        <Nav />
        {children}
      </body>
    </html>
  )
}
