import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'vz',
  description: 'Zero-config terminal data visualization',
  base: '/vz/',
  appearance: 'dark',
  head: [
    ['link', { rel: 'icon', href: '/vz/favicon.svg' }],
    ['meta', { property: 'og:title', content: 'vz — Zero-Config Terminal Data Visualization' }],
    ['meta', { property: 'og:description', content: 'CLI BI tool that auto-visualizes CSV, JSON, TSV data in your terminal.' }],
    ['meta', { property: 'og:type', content: 'website' }],
    ['meta', { name: 'twitter:card', content: 'summary' }],
  ],
  themeConfig: {
    logo: '/logo.svg',
    nav: [
      { text: 'Guide', link: '/guide/getting-started' },
      { text: 'Demo', link: '/demo' },
    ],
    sidebar: [
      {
        text: 'Guide',
        items: [
          { text: 'Getting Started', link: '/guide/getting-started' },
          { text: 'Chart Types', link: '/guide/chart-types' },
          { text: 'Output Modes', link: '/guide/output-modes' },
        ]
      }
    ],
    socialLinks: [
      { icon: 'github', link: 'https://github.com/r-hsnin/vz' }
    ],
    footer: {
      message: 'Released under the MIT License.',
      copyright: 'Built with <a href="https://ratatui.rs">ratatui</a> 🦀'
    },
    search: {
      provider: 'local'
    },
    editLink: {
      pattern: 'https://github.com/r-hsnin/vz/edit/main/docs/:path'
    }
  }
})
