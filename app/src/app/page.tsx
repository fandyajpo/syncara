import { Hero } from "@/components/hero"
import { Features } from "@/components/features"
import { StatsBar } from "@/components/stats-bar"
import { LocalVsProduction } from "@/components/local-vs-production"
import { HowItWorks } from "@/components/how-it-works"
import { InteractiveDemo } from "@/components/interactive-demo"
import { Manifesto } from "@/components/manifesto"
import { InstallSection } from "@/components/install-section"
import { DockerSection } from "@/components/docker-section"
import { CtaSection } from "@/components/cta-section"
import { Footer } from "@/components/footer"

export default function Home() {
  return (
    <>
      <Hero />
      <StatsBar />
      <Features />
      <LocalVsProduction />
      <HowItWorks />
      <InteractiveDemo />
      <Manifesto />
      <InstallSection />
      <DockerSection />
      <CtaSection />
      <Footer />
    </>
  )
}
