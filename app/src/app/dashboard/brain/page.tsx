import type { Metadata } from "next"
import { DashboardPage } from "@/components/dashboard-page"

export const metadata: Metadata = {
  title: "Traffic Brain — Syncara",
  description: "Real-time Syncara traffic brain dashboard. Live routing decisions with explainable scoring.",
}

export default function Brain() {
  return <DashboardPage />
}
