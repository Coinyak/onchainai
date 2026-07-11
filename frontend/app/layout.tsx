import type { Metadata } from "next";
import { Inter, JetBrains_Mono } from "next/font/google";
import { Analytics } from "@vercel/analytics/next";
import "./globals.css";
import { Providers } from "./providers";
import { TopNav } from "@/components/layout/TopNav";
import { SiteFooter } from "@/components/layout/SiteFooter";
import { DEFAULT_OG_IMAGE_PATH, SITE_ORIGIN } from "@/lib/site";

const inter = Inter({
  variable: "--font-inter",
  subsets: ["latin"],
});

const jetbrainsMono = JetBrains_Mono({
  variable: "--font-jetbrains-mono",
  subsets: ["latin"],
});

export const metadata: Metadata = {
  metadataBase: new URL(SITE_ORIGIN),
  title: {
    default: "OnchainAI",
    template: "%s",
  },
  description: "Crypto tool directory — MCP, CLI, SDK, API, x402, RWA, AI agents",
  openGraph: {
    title: "OnchainAI",
    description: "Crypto tool directory — MCP, CLI, SDK, API, x402, RWA, AI agents",
    siteName: "OnchainAI",
    type: "website",
    images: [
      {
        url: DEFAULT_OG_IMAGE_PATH,
        width: 1200,
        height: 630,
        alt: "OnchainAI",
      },
    ],
  },
  twitter: {
    card: "summary_large_image",
    title: "OnchainAI",
    description: "Crypto tool directory — MCP, CLI, SDK, API, x402, RWA, AI agents",
    images: [DEFAULT_OG_IMAGE_PATH],
  },
  icons: {
    icon: [
      { url: "/brand/onchainai-icon-32.png", sizes: "32x32", type: "image/png" },
      { url: "/brand/onchainai-icon-16.png", sizes: "16x16", type: "image/png" },
    ],
    apple: [{ url: "/brand/onchainai-icon-180.png", sizes: "180x180", type: "image/png" }],
  },
  manifest: "/site.webmanifest",
  // Base.dev domain verification (dashboard.base.org → Settings → Builder Codes)
  other: {
    "base:app_id": "6a479e8a2876ee6c1138a70a",
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html
      lang="en"
      className={`${inter.variable} ${jetbrainsMono.variable} h-full antialiased`}
    >
      <body className="site-app-shell min-h-screen flex flex-col bg-neutral-bg text-primary font-sans">
        <Providers>
          <TopNav />
          <main className="site-page-body flex-1 flex flex-col">{children}</main>
          <SiteFooter />
          {/* Only on Vercel — local next start has no /_vercel/insights and fails browser gates. */}
          {process.env.VERCEL ? <Analytics /> : null}
        </Providers>
      </body>
    </html>
  );
}