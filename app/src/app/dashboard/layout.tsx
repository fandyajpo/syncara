"use client"

import { useState } from "react"
import Link from "next/link"
import { usePathname } from "next/navigation"
import {
  Brain,
  Gauge,
  Server,
  Settings,
  ChevronLeft,
  Menu,
  Zap,
} from "lucide-react"

const navItems = [
  { label: "Traffic Brain", href: "/dashboard/brain", icon: Brain },
  { label: "Upstreams", href: "/dashboard/upstreams", icon: Server },
  { label: "Metrics", href: "/dashboard/metrics", icon: Gauge },
  { label: "Config", href: "/dashboard/config", icon: Settings },
]

export default function DashboardLayout({ children }: { children: React.ReactNode }) {
  const pathname = usePathname()
  const [collapsed, setCollapsed] = useState(false)

  return (
    <div className="min-h-screen bg-background text-foreground flex">
      {/* Sidebar */}
      <aside
        className={`border-r border-border bg-card flex flex-col transition-all duration-200 ${
          collapsed ? "w-14" : "w-56"
        } shrink-0`}
      >
        {/* Logo */}
        <div className="h-14 border-b border-border flex items-center gap-2 px-3">
          <Link href="/dashboard/brain" className="flex items-center gap-2 min-w-0">
            <span className="w-7 h-7 rounded-lg bg-primary flex items-center justify-center text-sm font-bold text-primary-foreground shrink-0">
              S
            </span>
            {!collapsed && (
              <span className="text-sm font-semibold truncate">Dashboard</span>
            )}
          </Link>
          <button
            onClick={() => setCollapsed((c) => !c)}
            className="p-1 rounded-md hover:bg-secondary/60 ml-auto text-muted-foreground hover:text-foreground transition-colors"
          >
            <ChevronLeft className={`w-4 h-4 transition-transform ${collapsed ? "rotate-180" : ""}`} />
          </button>
        </div>

        {/* Nav */}
        <nav className="flex-1 py-3 px-2 space-y-1">
          {navItems.map((item) => {
            const active = pathname === item.href || pathname.startsWith(item.href + "/")
            return (
              <Link
                key={item.href}
                href={item.href}
                className={`flex items-center gap-2 px-2.5 py-2 rounded-lg text-sm transition-colors ${
                  active
                    ? "bg-primary/10 text-primary font-medium"
                    : "text-muted-foreground hover:text-foreground hover:bg-secondary/40"
                }`}
                title={collapsed ? item.label : undefined}
              >
                <item.icon className="w-4.5 h-4.5 shrink-0" />
                {!collapsed && <span className="truncate">{item.label}</span>}
              </Link>
            )
          })}
        </nav>

        {/* Bottom */}
        <div className="border-t border-border p-3">
          {!collapsed && (
            <Link
              href="/"
              className="flex items-center gap-2 text-xs text-muted-foreground hover:text-foreground transition-colors"
            >
              <Zap className="w-3.5 h-3.5" />
              Back to site
            </Link>
          )}
        </div>
      </aside>

      {/* Main */}
      <main className="flex-1 min-w-0">{children}</main>
    </div>
  )
}
