import { Metadata } from "next"
import { DocsPage } from "@/components/docs-page"

export const metadata: Metadata = {
  title: "Documentation — Syncara",
  description: "Complete Syncara documentation: installation, configuration, features, CLI reference, and examples.",
}

export default function Docs() {
  return <DocsPage />
}
